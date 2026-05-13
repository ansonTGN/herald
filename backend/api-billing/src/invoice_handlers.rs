use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Response,
};
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use herald_api_base::application::http::common::auth_utils::{
    require_authenticated_user_in_realm, require_realm_access,
};
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::billing::invoice::{
    ActorType, InvoicePdfGenerator, InvoiceRepository, InvoiceSource, InvoiceStatus,
    InvoiceStatusTransition, NewInvoice, NewLineItem, UpdateInvoiceDraft,
};
use herald_core::domain::billing::invoice_service::validate_status_transition;
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::infrastructure::billing::IronPressInvoicePdfGenerator;

use crate::handlers::require_billing_permission;
use crate::invoice_types::*;

// ============================================================================
// Helper functions
// ============================================================================

/// Extract the user ID from an Identity, if it represents a user session.
fn actor_user_id_from_identity(identity: &Identity) -> Option<Uuid> {
    if identity.is_user() {
        Uuid::parse_str(&identity.user_id()).ok()
    } else {
        None
    }
}

/// Helper: load invoice detail and return 404 if not found.
async fn load_detail(
    state: &AppState,
    realm_id: &str,
    invoice_id: Uuid,
) -> Result<herald_core::domain::billing::invoice::InvoiceDetail, ApiError> {
    state
        .invoice_repository
        .find_with_items(realm_id, invoice_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Invoice not found"))
}

async fn validate_account_in_realm(
    pool: &PgPool,
    account_id: Uuid,
    realm_id: &str,
) -> Result<(), ApiError> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account WHERE id = $1 AND realm_id = $2)")
            .bind(account_id)
            .bind(realm_id)
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if !exists {
        return Err(ApiError::bad_request(format!(
            "Account {} does not exist in this realm",
            account_id
        )));
    }
    Ok(())
}

enum OwnedResource {
    PaymentAttempt,
    Subscription,
}

impl OwnedResource {
    fn table_name(&self) -> &'static str {
        match self {
            Self::PaymentAttempt => "payment_attempts",
            Self::Subscription => "subscription",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::PaymentAttempt => "payment attempt",
            Self::Subscription => "subscription",
        }
    }
}

async fn validate_resource_ownership(
    pool: &PgPool,
    resource: OwnedResource,
    resource_id: Uuid,
    user_id: Uuid,
    realm_id: &str,
) -> Result<(), ApiError> {
    let query = format!(
        "SELECT user_id FROM {} WHERE id = $1 AND realm_id = $2",
        resource.table_name()
    );
    let owner: Option<Uuid> = sqlx::query_scalar(&query)
        .bind(resource_id)
        .bind(realm_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    match owner {
        Some(uid) if uid == user_id => Ok(()),
        Some(_) => Err(ApiError::forbidden(format!(
            "You can only apply for invoices for your own {}s",
            resource.label()
        ))),
        None => Err(ApiError::bad_request(format!(
            "{} {} not found",
            resource.label(),
            resource_id
        ))),
    }
}

// ============================================================================
// Seller Config Handlers
// ============================================================================

#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/invoice-seller-config",
    tag = "billing-invoice",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Seller config found", body = SellerConfigResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Seller config not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_seller_config(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
) -> Result<Json<SellerConfigResponse>, ApiError> {
    tracing::info!("Getting seller config for realm: {}", realm_id);
    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let config = state
        .invoice_repository
        .find_seller_config(&realm_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Seller config not found for this realm"))?;

    Ok(Json(SellerConfigResponse::from(config)))
}

#[utoipa::path(
    put,
    path = "/api/bill/{realmId}/invoice-seller-config",
    tag = "billing-invoice",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = SellerConfigRequest,
    responses(
        (status = 200, description = "Seller config saved", body = SellerConfigResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn upsert_seller_config(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<SellerConfigRequest>,
) -> Result<Json<SellerConfigResponse>, ApiError> {
    tracing::info!("Upserting seller config for realm: {}", realm_id);
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;
    request
        .validate()
        .map_err(|e: validator::ValidationErrors| {
            CoreError::BadRequest(format!("Validation failed: {}", e))
        })?;

    let now = chrono::Utc::now();
    let config = herald_core::domain::billing::invoice::InvoiceSellerConfig {
        realm_id: realm_id.clone(),
        seller_name: request.seller_name,
        seller_address: request.seller_address,
        seller_email: request.seller_email,
        seller_phone: request.seller_phone,
        default_payment_terms: request.default_payment_terms,
        created_at: now,
        updated_at: now,
    };

    let saved = state
        .invoice_repository
        .upsert_seller_config(config)
        .await?;
    Ok(Json(SellerConfigResponse::from(saved)))
}

// ============================================================================
// Invoice CRUD Handlers
// ============================================================================

#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/invoices",
    tag = "billing-invoice",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = CreateInvoiceRequest,
    responses(
        (status = 201, description = "Invoice created", body = InvoiceDetailResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_invoice(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<CreateInvoiceRequest>,
) -> Result<(StatusCode, Json<InvoiceDetailResponse>), ApiError> {
    tracing::info!("Creating invoice for realm: {}", realm_id);
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;
    request
        .validate()
        .map_err(|e: validator::ValidationErrors| {
            CoreError::BadRequest(format!("Validation failed: {}", e))
        })?;

    validate_account_in_realm(&state.pool, request.account_id, &realm_id).await?;
    if let Some(applicant_id) = request.applicant_user_id {
        validate_account_in_realm(&state.pool, applicant_id, &realm_id).await?;
    }

    let line_items: Vec<NewLineItem> = request
        .line_items
        .into_iter()
        .map(|li| NewLineItem {
            name: li.name,
            description: li.description,
            quantity: li.quantity,
            unit_price: li.unit_price,
        })
        .collect();

    let new_invoice = NewInvoice {
        realm_id: realm_id.clone(),
        source: InvoiceSource::AdminManual,
        account_id: request.account_id,
        applicant_user_id: request.applicant_user_id,
        subscription_id: request.subscription_id,
        payment_attempt_id: request.payment_attempt_id,
        currency: request.currency,
        line_items,
        actor_user_id: actor_user_id_from_identity(&identity),
        billing_name: request.billing_name,
        billing_address: request.billing_address,
        billing_email: request.billing_email,
        billing_phone: request.billing_phone,
        seller_name: request.seller_name,
        seller_address: request.seller_address,
        seller_email: request.seller_email,
        seller_phone: request.seller_phone,
        discount_mode: parse_adjustment_mode(request.discount_mode.as_deref()),
        discount_value: request.discount_value,
        tax_mode: parse_adjustment_mode(request.tax_mode.as_deref()),
        tax_value: request.tax_value,
        shipping_mode: parse_adjustment_mode(request.shipping_mode.as_deref()),
        shipping_value: request.shipping_value,
        due_date: request.due_date,
        payment_terms: request.payment_terms,
        notes: request.notes,
    };

    let invoice = state.invoice_repository.create_invoice(new_invoice).await?;

    let detail = load_detail(&state, &realm_id, invoice.id).await?;

    Ok((
        StatusCode::CREATED,
        Json(invoice_to_detail_response(detail)),
    ))
}

#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/invoices",
    tag = "billing-invoice",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        InvoiceListQuery
    ),
    responses(
        (status = 200, description = "Invoices listed", body = InvoiceListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_invoices(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(query): Query<InvoiceListQuery>,
) -> Result<Json<InvoiceListResponse>, ApiError> {
    tracing::info!("Listing invoices for realm: {}", realm_id);
    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let filters = query.to_filters();

    let result = state
        .invoice_repository
        .list_admin(&realm_id, filters)
        .await?;

    Ok(Json(InvoiceListResponse {
        total: result.total,
        page: result.page,
        page_size: result.page_size,
        data: result.data.into_iter().map(summary_to_response).collect(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/invoices/{invoiceId}",
    tag = "billing-invoice",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("invoiceId" = Uuid, Path, description = "Invoice ID")
    ),
    responses(
        (status = 200, description = "Invoice detail", body = InvoiceDetailResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Invoice not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_invoice(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, invoice_id)): Path<(String, Uuid)>,
) -> Result<Json<InvoiceDetailResponse>, ApiError> {
    tracing::info!("Getting invoice {} for realm: {}", invoice_id, realm_id);
    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let detail = load_detail(&state, &realm_id, invoice_id).await?;

    Ok(Json(invoice_to_detail_response(detail)))
}

#[utoipa::path(
    patch,
    path = "/api/bill/{realmId}/invoices/{invoiceId}",
    tag = "billing-invoice",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("invoiceId" = Uuid, Path, description = "Invoice ID")
    ),
    request_body = UpdateInvoiceRequest,
    responses(
        (status = 200, description = "Invoice updated", body = InvoiceDetailResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Invoice not found", body = ErrorResponse),
        (status = 409, description = "Conflict - invoice not in draft status", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_invoice(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, invoice_id)): Path<(String, Uuid)>,
    Json(request): Json<UpdateInvoiceRequest>,
) -> Result<Json<InvoiceDetailResponse>, ApiError> {
    tracing::info!("Updating invoice {} for realm: {}", invoice_id, realm_id);
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    let line_items = request.line_items.map(|items| {
        items
            .into_iter()
            .map(|li| NewLineItem {
                name: li.name,
                description: li.description,
                quantity: li.quantity,
                unit_price: li.unit_price,
            })
            .collect()
    });

    let update = UpdateInvoiceDraft {
        realm_id: realm_id.clone(),
        invoice_id,
        actor_user_id: actor_user_id_from_identity(&identity),
        billing_name: request.billing_name,
        billing_address: request.billing_address,
        billing_email: request.billing_email,
        billing_phone: request.billing_phone,
        seller_name: request.seller_name,
        seller_address: request.seller_address,
        seller_email: request.seller_email,
        seller_phone: request.seller_phone,
        line_items,
        discount_mode: parse_adjustment_mode(request.discount_mode.as_deref()),
        discount_value: request.discount_value,
        tax_mode: parse_adjustment_mode(request.tax_mode.as_deref()),
        tax_value: request.tax_value,
        shipping_mode: parse_adjustment_mode(request.shipping_mode.as_deref()),
        shipping_value: request.shipping_value,
        due_date: request.due_date,
        payment_terms: request.payment_terms,
        notes: request.notes,
    };

    state.invoice_repository.update_draft(update).await?;

    let detail = load_detail(&state, &realm_id, invoice_id).await?;

    Ok(Json(invoice_to_detail_response(detail)))
}

// ============================================================================
// Status Transition Handlers
// ============================================================================

#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/invoices/{invoiceId}/issue",
    tag = "billing-invoice",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("invoiceId" = Uuid, Path, description = "Invoice ID")
    ),
    request_body = IssueInvoiceRequest,
    responses(
        (status = 200, description = "Invoice issued", body = InvoiceDetailResponse),
        (status = 400, description = "Bad request - no line items or zero total", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Invoice not found", body = ErrorResponse),
        (status = 409, description = "Conflict - invalid status transition", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn issue_invoice(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, invoice_id)): Path<(String, Uuid)>,
    Json(_request): Json<IssueInvoiceRequest>,
) -> Result<Json<InvoiceDetailResponse>, ApiError> {
    tracing::info!("Issuing invoice {} for realm: {}", invoice_id, realm_id);
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    let detail = load_detail(&state, &realm_id, invoice_id).await?;

    validate_status_transition(
        detail.invoice.status,
        InvoiceStatus::Issued,
        detail.line_items.len(),
        detail.invoice.total,
        ActorType::User,
        false,
        None,
    )?;

    let actor_user_id = Uuid::parse_str(&identity.user_id()).ok();
    state
        .invoice_repository
        .transition_status(InvoiceStatusTransition {
            realm_id: realm_id.clone(),
            invoice_id,
            target_status: InvoiceStatus::Issued,
            actor_user_id,
            actor_type: ActorType::User,
            void_reason: None,
        })
        .await?;

    let detail = load_detail(&state, &realm_id, invoice_id).await?;

    Ok(Json(invoice_to_detail_response(detail)))
}

#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/invoices/{invoiceId}/void",
    tag = "billing-invoice",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("invoiceId" = Uuid, Path, description = "Invoice ID")
    ),
    request_body = VoidInvoiceRequest,
    responses(
        (status = 200, description = "Invoice voided", body = InvoiceDetailResponse),
        (status = 400, description = "Bad request - void reason required for issued invoices", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Invoice not found", body = ErrorResponse),
        (status = 409, description = "Conflict - terminal state", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn void_invoice(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, invoice_id)): Path<(String, Uuid)>,
    Json(request): Json<VoidInvoiceRequest>,
) -> Result<Json<InvoiceDetailResponse>, ApiError> {
    tracing::info!("Voiding invoice {} for realm: {}", invoice_id, realm_id);
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    let detail = load_detail(&state, &realm_id, invoice_id).await?;

    validate_status_transition(
        detail.invoice.status,
        InvoiceStatus::Void,
        detail.line_items.len(),
        detail.invoice.total,
        ActorType::User,
        false,
        request.void_reason.as_deref(),
    )?;

    let actor_user_id = Uuid::parse_str(&identity.user_id()).ok();
    state
        .invoice_repository
        .transition_status(InvoiceStatusTransition {
            realm_id: realm_id.clone(),
            invoice_id,
            target_status: InvoiceStatus::Void,
            actor_user_id,
            actor_type: ActorType::User,
            void_reason: request.void_reason,
        })
        .await?;

    let detail = load_detail(&state, &realm_id, invoice_id).await?;

    Ok(Json(invoice_to_detail_response(detail)))
}

#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/invoices/{invoiceId}/mark-paid",
    tag = "billing-invoice",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("invoiceId" = Uuid, Path, description = "Invoice ID")
    ),
    request_body = MarkPaidRequest,
    responses(
        (status = 200, description = "Invoice marked as paid", body = InvoiceDetailResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Invoice not found", body = ErrorResponse),
        (status = 409, description = "Conflict - invalid status transition", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn mark_paid(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, invoice_id)): Path<(String, Uuid)>,
    Json(_request): Json<MarkPaidRequest>,
) -> Result<Json<InvoiceDetailResponse>, ApiError> {
    tracing::info!(
        "Marking invoice {} as paid for realm: {}",
        invoice_id,
        realm_id
    );
    require_billing_permission(&state, &identity, &realm_id, "manage").await?;

    let detail = load_detail(&state, &realm_id, invoice_id).await?;

    validate_status_transition(
        detail.invoice.status,
        InvoiceStatus::Paid,
        detail.line_items.len(),
        detail.invoice.total,
        ActorType::User,
        false,
        None,
    )?;

    let actor_user_id = Uuid::parse_str(&identity.user_id()).ok();
    state
        .invoice_repository
        .transition_status(InvoiceStatusTransition {
            realm_id: realm_id.clone(),
            invoice_id,
            target_status: InvoiceStatus::Paid,
            actor_user_id,
            actor_type: ActorType::User,
            void_reason: None,
        })
        .await?;

    let detail = load_detail(&state, &realm_id, invoice_id).await?;

    Ok(Json(invoice_to_detail_response(detail)))
}

// ============================================================================
// User-Facing Invoice Handlers
// ============================================================================

#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/my/invoices",
    tag = "billing-invoice",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = ApplyInvoiceRequest,
    responses(
        (status = 201, description = "Invoice application created", body = InvoiceDetailResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn apply_invoice(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<ApplyInvoiceRequest>,
) -> Result<(StatusCode, Json<InvoiceDetailResponse>), ApiError> {
    tracing::info!("User applying for invoice in realm: {}", realm_id);
    let applicant_user_id =
        require_authenticated_user_in_realm(&identity, &realm_id, "apply invoices")?;
    request
        .validate()
        .map_err(|e: validator::ValidationErrors| {
            CoreError::BadRequest(format!("Validation failed: {}", e))
        })?;

    if request.payment_attempt_id.is_none() && request.subscription_id.is_none() {
        return Err(ApiError::bad_request(
            "At least one of paymentAttemptId or subscriptionId is required",
        ));
    }

    if let Some(pa_id) = request.payment_attempt_id {
        validate_resource_ownership(
            &state.pool,
            OwnedResource::PaymentAttempt,
            pa_id,
            applicant_user_id,
            &realm_id,
        )
        .await?;
    }
    if let Some(sub_id) = request.subscription_id {
        validate_resource_ownership(
            &state.pool,
            OwnedResource::Subscription,
            sub_id,
            applicant_user_id,
            &realm_id,
        )
        .await?;
    }

    let seller_config = state
        .invoice_repository
        .find_seller_config(&realm_id)
        .await?
        .ok_or_else(|| {
            ApiError::bad_request(
                "No seller configuration found for this realm. An admin must configure seller info first.",
            )
        })?;

    let new_invoice = NewInvoice {
        realm_id: realm_id.clone(),
        source: InvoiceSource::UserApplication,
        account_id: applicant_user_id,
        applicant_user_id: Some(applicant_user_id),
        subscription_id: request.subscription_id,
        payment_attempt_id: request.payment_attempt_id,
        currency: request.currency,
        line_items: vec![],
        actor_user_id: Some(applicant_user_id),
        billing_name: request.billing_name,
        billing_address: request.billing_address,
        billing_email: request.billing_email,
        billing_phone: request.billing_phone,
        seller_name: seller_config.seller_name,
        seller_address: seller_config.seller_address,
        seller_email: seller_config.seller_email,
        seller_phone: seller_config.seller_phone,
        discount_mode: None,
        discount_value: None,
        tax_mode: None,
        tax_value: None,
        shipping_mode: None,
        shipping_value: None,
        due_date: request.due_date,
        payment_terms: seller_config.default_payment_terms,
        notes: request.notes,
    };

    let invoice = state.invoice_repository.create_invoice(new_invoice).await?;

    let detail = load_detail(&state, &realm_id, invoice.id).await?;

    Ok((
        StatusCode::CREATED,
        Json(invoice_to_detail_response(detail)),
    ))
}

#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/my/invoices",
    tag = "billing-invoice",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        InvoiceListQuery
    ),
    responses(
        (status = 200, description = "My invoices listed", body = InvoiceListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_my_invoices(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(query): Query<InvoiceListQuery>,
) -> Result<Json<InvoiceListResponse>, ApiError> {
    tracing::info!("Listing my invoices for realm: {}", realm_id);
    let user_id = require_authenticated_user_in_realm(&identity, &realm_id, "view invoices")?;

    let filters = query.to_filters();

    let result = state
        .invoice_repository
        .list_user(&realm_id, user_id, filters)
        .await?;

    Ok(Json(InvoiceListResponse {
        total: result.total,
        page: result.page,
        page_size: result.page_size,
        data: result.data.into_iter().map(summary_to_response).collect(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/my/invoices/{invoiceId}",
    tag = "billing-invoice",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("invoiceId" = Uuid, Path, description = "Invoice ID")
    ),
    responses(
        (status = 200, description = "Invoice detail", body = InvoiceDetailResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Invoice not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_my_invoice(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, invoice_id)): Path<(String, Uuid)>,
) -> Result<Json<InvoiceDetailResponse>, ApiError> {
    tracing::info!("Getting my invoice {} for realm: {}", invoice_id, realm_id);
    require_realm_access(&identity, &realm_id, "view invoices")?;

    let detail = load_detail(&state, &realm_id, invoice_id).await?;

    // Verify the invoice belongs to the current user
    let current_user_id = Uuid::parse_str(&identity.user_id()).ok();
    if detail.invoice.applicant_user_id != current_user_id {
        return Err(ApiError::forbidden("You can only view your own invoices"));
    }

    Ok(Json(invoice_to_detail_response(detail)))
}

// ============================================================================
// PDF Download Handlers
// ============================================================================

/// Validate that an invoice status allows PDF download (not draft).
fn validate_pdf_status(status: InvoiceStatus) -> Result<(), ApiError> {
    if status == InvoiceStatus::Draft {
        return Err(ApiError::conflict(
            "PDF is not available for draft invoices. Issue the invoice first.",
        ));
    }
    Ok(())
}

/// Build a PDF response with Content-Type and Content-Disposition headers.
fn build_pdf_response(pdf_bytes: Vec<u8>, invoice_number: &str) -> Response {
    use axum::http::header;

    let disposition = format!("attachment; filename=\"{}.pdf\"", invoice_number);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/pdf")
        .header(header::CONTENT_DISPOSITION, disposition)
        .body(axum::body::Body::from(pdf_bytes))
        .unwrap()
}

#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/invoices/{invoiceId}/pdf",
    tag = "billing-invoice",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("invoiceId" = Uuid, Path, description = "Invoice ID")
    ),
    responses(
        (status = 200, description = "Invoice PDF binary data"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Invoice not found", body = ErrorResponse),
        (status = 409, description = "Conflict - invoice is draft", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn download_invoice_pdf(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, invoice_id)): Path<(String, Uuid)>,
) -> Result<Response, ApiError> {
    tracing::info!(
        "Downloading invoice PDF {} for realm: {}",
        invoice_id,
        realm_id
    );
    require_billing_permission(&state, &identity, &realm_id, "view").await?;

    let detail = load_detail(&state, &realm_id, invoice_id).await?;
    validate_pdf_status(detail.invoice.status)?;

    let generator = IronPressInvoicePdfGenerator;
    let pdf_bytes = generator.generate(&detail).await?;

    Ok(build_pdf_response(
        pdf_bytes,
        &detail.invoice.invoice_number,
    ))
}

#[utoipa::path(
    get,
    path = "/api/bill/{realmId}/my/invoices/{invoiceId}/pdf",
    tag = "billing-invoice",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("invoiceId" = Uuid, Path, description = "Invoice ID")
    ),
    responses(
        (status = 200, description = "Invoice PDF binary data"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - not your invoice", body = ErrorResponse),
        (status = 404, description = "Invoice not found", body = ErrorResponse),
        (status = 409, description = "Conflict - invoice is draft", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn download_my_invoice_pdf(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, invoice_id)): Path<(String, Uuid)>,
) -> Result<Response, ApiError> {
    tracing::info!(
        "Downloading my invoice PDF {} for realm: {}",
        invoice_id,
        realm_id
    );
    require_realm_access(&identity, &realm_id, "view invoices")?;

    let detail = load_detail(&state, &realm_id, invoice_id).await?;

    // Verify the invoice belongs to the current user
    let current_user_id = Uuid::parse_str(&identity.user_id()).ok();
    if detail.invoice.applicant_user_id != current_user_id {
        return Err(ApiError::forbidden(
            "You can only download your own invoices",
        ));
    }

    validate_pdf_status(detail.invoice.status)?;

    let generator = IronPressInvoicePdfGenerator;
    let pdf_bytes = generator.generate(&detail).await?;

    Ok(build_pdf_response(
        pdf_bytes,
        &detail.invoice.invoice_number,
    ))
}
