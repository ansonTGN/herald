use axum::{
    Json,
    extract::{Extension, Path, State},
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::shopify_claim_types::{ShopifyClaimRequest, ShopifyClaimResponse};
use herald_api_base::application::http::server::api_entities::{ApiError, ErrorResponse};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::billing::SubscriptionStatus;
use herald_core::domain::points::PointsRepository;
use validator::Validate;

#[derive(Debug, Clone, sqlx::FromRow)]
struct ClaimableSubscriptionRow {
    subscription_id: Uuid,
    user_id: Option<Uuid>,
    plan_id: Option<Uuid>,
    current_period_end: Option<DateTime<Utc>>,
    status: String,
}

#[utoipa::path(
    post,
    path = "/api/bill/{realmId}/shopify/claim",
    tag = "billing",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = ShopifyClaimRequest,
    responses(
        (status = 200, description = "Shopify subscriptions claimed successfully", body = ShopifyClaimResponse),
        (status = 400, description = "Invalid claim request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Realm or Shopify binding not found", body = ErrorResponse),
        (status = 409, description = "Shopify customer already claimed by another user", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("session_token" = [])),
    summary = "Claim Shopify subscriptions",
    description = "Claims unowned Shopify subscriptions for the current authenticated user and optionally grants the current period once."
)]
pub async fn claim_shopify_subscriptions(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Json(request): Json<ShopifyClaimRequest>,
) -> Result<Json<ShopifyClaimResponse>, ApiError> {
    if !identity.is_user() {
        return Err(ApiError::forbidden(
            "Access denied: authenticated user session required".to_string(),
        ));
    }

    if !identity.has_access_to_realm(&realm_id) {
        return Err(ApiError::forbidden(
            "Access denied: cannot claim Shopify subscriptions from a different realm".to_string(),
        ));
    }

    request
        .validate()
        .map_err(|e| ApiError::bad_request(format!("Invalid claim request: {e}")))?;

    let user_id = Uuid::parse_str(&identity.user_id())
        .map_err(|e| ApiError::bad_request(format!("Invalid authenticated user id: {e}")))?;

    let shop_domain = load_shopify_shop_domain(&state.pool, &realm_id).await?;
    let shopify_customer_id = resolve_shopify_customer_id(&state.pool, &realm_id, &request).await?;

    match load_existing_shopify_user_binding(
        &state.pool,
        &realm_id,
        &shop_domain,
        &shopify_customer_id,
    )
    .await?
    {
        Some(Some(existing_user_id)) if existing_user_id != user_id => {
            return Err(ApiError::conflict(
                "This Shopify customer is already claimed by another user".to_string(),
            ));
        }
        Some(Some(_)) | Some(None) | None => {
            upsert_shopify_user_binding(
                &state.pool,
                &realm_id,
                &shop_domain,
                &shopify_customer_id,
                user_id,
            )
            .await?;
        }
    }

    let claimable_subscriptions =
        load_claimable_subscriptions(&state.pool, &realm_id, &shop_domain, &shopify_customer_id)
            .await?;

    let mut claimed_subscription_ids = Vec::new();
    let mut granted_subscription_ids = Vec::new();
    let mut skipped_subscription_ids = Vec::new();

    for row in claimable_subscriptions {
        let subscription_id = row.subscription_id;

        match row.user_id {
            Some(existing_user_id) if existing_user_id != user_id => {
                return Err(ApiError::conflict(
                    "One or more Shopify subscriptions are already claimed by another user"
                        .to_string(),
                ));
            }
            Some(_) => {
                claimed_subscription_ids.push(subscription_id);
            }
            None => {
                assign_subscription_user(&state.pool, subscription_id, user_id).await?;
                claimed_subscription_ids.push(subscription_id);
            }
        }

        let status = row
            .status
            .parse::<SubscriptionStatus>()
            .map_err(|e| ApiError::bad_request(format!("Invalid subscription status: {e}")))?;

        let Some(plan_id) = row.plan_id else {
            skipped_subscription_ids.push(subscription_id);
            continue;
        };

        let Some(period_end) = row.current_period_end else {
            skipped_subscription_ids.push(subscription_id);
            continue;
        };

        if !status.has_access() || period_end <= Utc::now() {
            skipped_subscription_ids.push(subscription_id);
            continue;
        }

        // Look up entitlement mapping via plan_id as entitlement_key
        let entitlement_mapping = state
            .points_repository
            .find_points_policy_by_entitlement_key(&realm_id, &plan_id.to_string())
            .await?;

        let Some(entitlement_mapping) = entitlement_mapping else {
            skipped_subscription_ids.push(subscription_id);
            continue;
        };

        if !entitlement_mapping.grant_on_subscribe {
            skipped_subscription_ids.push(subscription_id);
            continue;
        };

        let claim_event_id = format!(
            "shopify_claim_grant:{}:{}",
            subscription_id,
            period_end.to_rfc3339()
        );

        state
            .subscription_service
            .handle_subscription_paid(
                user_id,
                &realm_id,
                &entitlement_mapping.entitlement_key,
                false,
                period_end,
                claim_event_id,
            )
            .await?;

        granted_subscription_ids.push(subscription_id);
    }

    info!(
        realm_id = %realm_id,
        user_id = %user_id,
        shop_domain = %shop_domain,
        shopify_customer_id = %shopify_customer_id,
        claimed = claimed_subscription_ids.len(),
        granted = granted_subscription_ids.len(),
        skipped = skipped_subscription_ids.len(),
        "Claimed Shopify subscriptions for user"
    );

    Ok(Json(ShopifyClaimResponse {
        realm_id,
        user_id,
        shop_domain,
        shopify_customer_id,
        claimed_subscription_ids,
        granted_subscription_ids,
        skipped_subscription_ids,
    }))
}

async fn load_shopify_shop_domain(pool: &PgPool, realm_id: &str) -> Result<String, ApiError> {
    let shop_domain = sqlx::query_scalar::<_, String>(
        "SELECT config_value
         FROM realm_config
         WHERE realm_id = $1
           AND config_type = 'shopify'
           AND config_key = 'shop_domain'
           AND enabled = true
         LIMIT 1",
    )
    .bind(realm_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to load Shopify config: {e}")))?
    .ok_or_else(|| ApiError::not_found("Shopify configuration not found"))?;

    Ok(shop_domain)
}

async fn resolve_shopify_customer_id(
    pool: &PgPool,
    realm_id: &str,
    request: &ShopifyClaimRequest,
) -> Result<String, ApiError> {
    let mut candidate = request
        .shopify_customer_id
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if let Some(contract_id) = request
        .contract_id
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        let binding_customer_id =
            load_customer_id_by_contract_id(pool, realm_id, &contract_id).await?;
        candidate = reconcile_customer_id(candidate, binding_customer_id)?;
    }

    if let Some(order_id) = request
        .order_id
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        let binding_customer_id = load_customer_id_by_order_id(pool, realm_id, &order_id).await?;
        candidate = reconcile_customer_id(candidate, binding_customer_id)?;
    }

    candidate.ok_or_else(|| {
        ApiError::not_found("Unable to resolve Shopify customer id from claim request".to_string())
    })
}

fn reconcile_customer_id(
    current: Option<String>,
    next: Option<String>,
) -> Result<Option<String>, ApiError> {
    match (current, next) {
        (Some(current), Some(next)) if current != next => Err(ApiError::conflict(
            "Conflicting Shopify customer ids provided in claim request".to_string(),
        )),
        (Some(current), _) => Ok(Some(current)),
        (None, Some(next)) => Ok(Some(next)),
        (None, None) => Ok(None),
    }
}

async fn load_customer_id_by_contract_id(
    pool: &PgPool,
    realm_id: &str,
    contract_id: &str,
) -> Result<Option<String>, ApiError> {
    let row = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT customer_id
         FROM shopify_subscription_binding
         WHERE realm_id = $1
           AND contract_id = $2
         LIMIT 1",
    )
    .bind(realm_id)
    .bind(contract_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to load Shopify binding: {e}")))?;

    Ok(row.and_then(|(customer_id,)| customer_id))
}

async fn load_customer_id_by_order_id(
    pool: &PgPool,
    realm_id: &str,
    order_id: &str,
) -> Result<Option<String>, ApiError> {
    let row = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT customer_id
         FROM shopify_subscription_binding
         WHERE realm_id = $1
           AND last_order_id = $2
         LIMIT 1",
    )
    .bind(realm_id)
    .bind(order_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to load Shopify binding: {e}")))?;

    Ok(row.and_then(|(customer_id,)| customer_id))
}

async fn load_existing_shopify_user_binding(
    pool: &PgPool,
    realm_id: &str,
    shop_domain: &str,
    shopify_customer_id: &str,
) -> Result<Option<Option<Uuid>>, ApiError> {
    let row = sqlx::query_as::<_, (Option<Uuid>,)>(
        "SELECT user_id
         FROM shopify_user_binding
         WHERE realm_id = $1
           AND shop_domain = $2
           AND shopify_customer_id = $3
         LIMIT 1",
    )
    .bind(realm_id)
    .bind(shop_domain)
    .bind(shopify_customer_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to load Shopify user binding: {e}")))?;

    Ok(row.map(|(user_id,)| user_id))
}

async fn upsert_shopify_user_binding(
    pool: &PgPool,
    realm_id: &str,
    shop_domain: &str,
    shopify_customer_id: &str,
    user_id: Uuid,
) -> Result<(), ApiError> {
    let existing_user_id =
        load_existing_shopify_user_binding(pool, realm_id, shop_domain, shopify_customer_id)
            .await?;

    match existing_user_id {
        Some(Some(existing_user_id)) if existing_user_id != user_id => Err(ApiError::conflict(
            "This Shopify customer is already claimed by another user".to_string(),
        )),
        Some(Some(_)) => {
            sqlx::query(
                "UPDATE shopify_user_binding
                 SET status = 'active',
                     updated_at = NOW()
                 WHERE realm_id = $1
                   AND shop_domain = $2
                   AND shopify_customer_id = $3",
            )
            .bind(realm_id)
            .bind(shop_domain)
            .bind(shopify_customer_id)
            .execute(pool)
            .await
            .map_err(|e| {
                ApiError::internal(format!("Failed to update Shopify user binding: {e}"))
            })?;
            Ok(())
        }
        Some(None) => {
            sqlx::query(
                "UPDATE shopify_user_binding
                 SET user_id = $1,
                     status = 'active',
                     updated_at = NOW()
                 WHERE realm_id = $2
                   AND shop_domain = $3
                   AND shopify_customer_id = $4",
            )
            .bind(user_id)
            .bind(realm_id)
            .bind(shop_domain)
            .bind(shopify_customer_id)
            .execute(pool)
            .await
            .map_err(|e| {
                ApiError::internal(format!("Failed to update Shopify user binding: {e}"))
            })?;
            Ok(())
        }
        None => {
            sqlx::query(
                "INSERT INTO shopify_user_binding
                 (realm_id, shop_domain, shopify_customer_id, shopify_customer_gid, user_id, status, created_at, updated_at)
                 VALUES ($1, $2, $3, NULL, $4, 'active', NOW(), NOW())",
            )
            .bind(realm_id)
            .bind(shop_domain)
            .bind(shopify_customer_id)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to create Shopify user binding: {e}")))?;
            Ok(())
        }
    }
}

async fn load_claimable_subscriptions(
    pool: &PgPool,
    realm_id: &str,
    shop_domain: &str,
    shopify_customer_id: &str,
) -> Result<Vec<ClaimableSubscriptionRow>, ApiError> {
    let rows = sqlx::query_as::<_, ClaimableSubscriptionRow>(
        "SELECT
            s.id AS subscription_id,
            s.user_id,
            s.plan_id,
            s.current_period_end,
            s.status
         FROM subscription s
         INNER JOIN shopify_subscription_binding b
           ON b.subscription_id = s.id
         WHERE b.realm_id = $1
           AND b.shop_domain = $2
           AND b.customer_id = $3
         ORDER BY s.created_at ASC",
    )
    .bind(realm_id)
    .bind(shop_domain)
    .bind(shopify_customer_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to load claimable subscriptions: {e}")))?;

    Ok(rows)
}

async fn assign_subscription_user(
    pool: &PgPool,
    subscription_id: Uuid,
    user_id: Uuid,
) -> Result<(), ApiError> {
    let updated_user_id = sqlx::query_scalar::<_, Option<Uuid>>(
        "UPDATE subscription
         SET user_id = $1,
             updated_at = NOW()
         WHERE id = $2
           AND user_id IS NULL
         RETURNING user_id",
    )
    .bind(user_id)
    .bind(subscription_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to claim subscription: {e}")))?;

    if updated_user_id.is_some() {
        return Ok(());
    }

    let current_user_id = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT user_id
         FROM subscription
         WHERE id = $1
         LIMIT 1",
    )
    .bind(subscription_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to reload subscription owner: {e}")))?;

    match current_user_id.flatten() {
        Some(existing_user_id) if existing_user_id == user_id => Ok(()),
        Some(_) => Err(ApiError::conflict(
            "Subscription is already claimed by another user".to_string(),
        )),
        None => Err(ApiError::not_found("Subscription not found")),
    }
}
