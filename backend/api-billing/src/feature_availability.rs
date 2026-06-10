use axum::{
    Json,
    extract::{Extension, Path, State},
};
use herald_api_base::application::http::common::auth_utils::require_authenticated_user_in_realm;
use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::PermissionService;
use herald_core::domain::billing::BillingRepository;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FeatureAvailabilityResponse {
    pub admin: AdminFeatureAvailability,
    pub user: UserFeatureAvailability,
    pub facts: FeatureAvailabilityFacts,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminFeatureAvailability {
    pub billing_visible: bool,
    pub billing_config_visible: bool,
    pub entitlement_mappings_visible: bool,
    pub invoices_visible: bool,
    pub subscription_history_visible: bool,
    pub points_visible: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserFeatureAvailability {
    pub points_visible: bool,
    pub points_purchase_visible: bool,
    pub subscription_visible: bool,
    pub invoices_visible: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FeatureAvailabilityFacts {
    pub has_payment_providers: bool,
    pub has_entitlement_mappings: bool,
    pub has_enabled_mappings: bool,
    pub has_one_time_mappings: bool,
    pub has_invoice_seller_config: bool,
    pub has_invoices: bool,
    pub has_subscription_history: bool,
}

#[derive(Debug, Clone)]
struct FeatureFacts {
    has_payment_providers: bool,
    has_entitlement_mappings: bool,
    has_enabled_mappings: bool,
    has_one_time_mappings: bool,
    has_invoice_seller_config: bool,
    has_invoices: bool,
    has_subscription_history: bool,
}

#[utoipa::path(
    get,
    path = "/api/realms/{realmId}/feature-availability",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    responses(
        (status = 200, description = "Feature availability summary", body = FeatureAvailabilityResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "realms",
    operation_id = "get_feature_availability"
)]
pub async fn get_feature_availability(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<FeatureAvailabilityResponse>, ApiError> {
    let user_id =
        require_authenticated_user_in_realm(&identity, &realm_id, "feature availability")?;
    let (can_view_billing, can_view_points) = tokio::try_join!(
        has_permission(&state, &realm_id, &user_id, "billing", "view"),
        has_permission(&state, &realm_id, &user_id, "points", "view"),
    )?;
    let facts = load_feature_facts(&state, &realm_id).await?;

    let admin_billing_visible = can_view_billing;
    let admin_points_visible = can_view_points;
    let user_subscription_visible = facts.has_enabled_mappings;
    let user_points_visible = facts.has_one_time_mappings;
    let user_invoices_visible = facts.has_invoice_seller_config;

    Ok(Json(FeatureAvailabilityResponse {
        admin: AdminFeatureAvailability {
            billing_visible: admin_billing_visible,
            billing_config_visible: admin_billing_visible,
            entitlement_mappings_visible: admin_billing_visible,
            invoices_visible: admin_billing_visible,
            subscription_history_visible: admin_billing_visible,
            points_visible: admin_points_visible,
        },
        user: UserFeatureAvailability {
            points_visible: user_points_visible,
            points_purchase_visible: facts.has_one_time_mappings,
            subscription_visible: user_subscription_visible,
            invoices_visible: user_invoices_visible,
        },
        facts: FeatureAvailabilityFacts {
            has_payment_providers: facts.has_payment_providers,
            has_entitlement_mappings: facts.has_entitlement_mappings,
            has_enabled_mappings: facts.has_enabled_mappings,
            has_one_time_mappings: facts.has_one_time_mappings,
            has_invoice_seller_config: facts.has_invoice_seller_config,
            has_invoices: facts.has_invoices,
            has_subscription_history: facts.has_subscription_history,
        },
    }))
}

async fn has_permission(
    state: &AppState,
    realm_id: &str,
    user_id: &Uuid,
    resource: &str,
    action: &str,
) -> Result<bool, ApiError> {
    state
        .permission_checker
        .check_permission(realm_id, &user_id.to_string(), resource, action)
        .await
        .map_err(|e| {
            tracing::error!(
                realm_id = %realm_id,
                user_id = %user_id,
                resource = %resource,
                action = %action,
                error = %e,
                "Failed to check feature availability permission"
            );
            ApiError::internal("Failed to check permission")
        })
}

async fn load_feature_facts(state: &AppState, realm_id: &str) -> Result<FeatureFacts, ApiError> {
    let facts = state
        .billing_repository
        .check_feature_facts(realm_id, &state.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                realm_id = %realm_id,
                error = %e,
                "Failed to load feature availability facts"
            );
            ApiError::internal("Failed to load feature availability")
        })?;

    Ok(FeatureFacts {
        has_payment_providers: facts.has_payment_providers,
        has_entitlement_mappings: facts.has_entitlement_mappings,
        has_enabled_mappings: facts.has_enabled_mappings,
        has_one_time_mappings: facts.has_one_time_mappings,
        has_invoice_seller_config: facts.has_invoice_seller_config,
        has_invoices: facts.has_invoices,
        has_subscription_history: facts.has_subscription_history,
    })
}
