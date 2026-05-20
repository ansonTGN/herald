use axum::{
    Json,
    extract::{Extension, Path, State},
};
use herald_api_base::application::http::common::auth_utils::require_authenticated_user_in_realm;
use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::authorization::PermissionService;
use serde::Serialize;
use sqlx::Row;
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
    pub products_visible: bool,
    pub plans_visible: bool,
    pub invoices_visible: bool,
    pub subscription_history_visible: bool,
    pub points_visible: bool,
    pub points_packages_visible: bool,
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
    pub has_products: bool,
    pub has_plans: bool,
    pub has_plan_payment_mappings: bool,
    pub has_points_packages: bool,
    pub has_points_package_payment_mappings: bool,
    pub has_invoice_seller_config: bool,
    pub has_invoices: bool,
    pub has_subscription_history: bool,
}

#[derive(Debug, Clone)]
struct FeatureFacts {
    has_payment_providers: bool,
    has_products: bool,
    has_plans: bool,
    has_plan_payment_mappings: bool,
    has_user_visible_plans: bool,
    has_points_packages: bool,
    has_points_package_payment_mappings: bool,
    has_invoice_seller_config: bool,
    has_invoices: bool,
    has_user_invoices: bool,
    has_subscription_history: bool,
    has_user_subscription_history: bool,
    has_user_invoice_sources: bool,
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
    let facts = load_feature_facts(&state, &realm_id, user_id).await?;

    let admin_billing_visible = can_view_billing;
    let admin_points_visible = can_view_points;
    let user_subscription_visible =
        facts.has_user_visible_plans || facts.has_user_subscription_history;
    let user_invoices_visible = facts.has_invoice_seller_config
        && (facts.has_user_invoice_sources || facts.has_user_invoices);

    Ok(Json(FeatureAvailabilityResponse {
        admin: AdminFeatureAvailability {
            billing_visible: admin_billing_visible,
            billing_config_visible: admin_billing_visible,
            products_visible: admin_billing_visible,
            plans_visible: admin_billing_visible && facts.has_products,
            invoices_visible: admin_billing_visible
                && (facts.has_invoice_seller_config || facts.has_invoices),
            subscription_history_visible: admin_billing_visible
                && (facts.has_plans || facts.has_subscription_history),
            points_visible: admin_points_visible,
            points_packages_visible: admin_points_visible,
        },
        user: UserFeatureAvailability {
            points_visible: true,
            points_purchase_visible: facts.has_points_package_payment_mappings,
            subscription_visible: user_subscription_visible,
            invoices_visible: user_invoices_visible,
        },
        facts: FeatureAvailabilityFacts {
            has_payment_providers: facts.has_payment_providers,
            has_products: facts.has_products,
            has_plans: facts.has_plans,
            has_plan_payment_mappings: facts.has_plan_payment_mappings,
            has_points_packages: facts.has_points_packages,
            has_points_package_payment_mappings: facts.has_points_package_payment_mappings,
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

async fn load_feature_facts(
    state: &AppState,
    realm_id: &str,
    user_id: Uuid,
) -> Result<FeatureFacts, ApiError> {
    let row = sqlx::query(
        r#"
        WITH configured_providers AS (
            SELECT 'wechat' AS provider
            WHERE EXISTS (
                SELECT 1 FROM realm_config
                WHERE realm_id = $1 AND config_type = 'wechat' AND enabled = true
            )
            UNION ALL
            SELECT 'shopify'
            WHERE EXISTS (
                SELECT 1 FROM realm_config
                WHERE realm_id = $1 AND config_type = 'shopify' AND enabled = true
            )
            UNION ALL
            SELECT 'stripe'
            WHERE EXISTS (
                SELECT 1 FROM realm_config
                WHERE realm_id = $1 AND config_type = 'stripe'
                  AND config_key = 'api_key' AND enabled = true
            )
            UNION ALL
            SELECT 'creem'
            WHERE EXISTS (
                SELECT 1 FROM realm_config
                WHERE realm_id = $1 AND config_type = 'creem'
                  AND config_key = 'api_key' AND enabled = true
            )
        )
        SELECT
            EXISTS (SELECT 1 FROM configured_providers) AS has_payment_providers,
            EXISTS (SELECT 1 FROM products WHERE realm_id = $1) AS has_products,
            EXISTS (SELECT 1 FROM subscription_plan WHERE realm_id = $1) AS has_plans,
            EXISTS (
                SELECT 1
                FROM subscription_plan_payment_provider spp
                JOIN subscription_plan sp ON sp.id = spp.plan_id
                JOIN configured_providers cp ON cp.provider = spp.payment_provider
                WHERE sp.realm_id = $1 AND spp.enabled = true
            ) AS has_plan_payment_mappings,
            EXISTS (
                SELECT 1
                FROM subscription_plan sp
                JOIN subscription_plan_payment_provider spp ON spp.plan_id = sp.id
                JOIN configured_providers cp ON cp.provider = spp.payment_provider
                JOIN client_app_subscription_plan casp ON casp.plan_id = sp.id
                JOIN client_app ca ON ca.id = casp.client_app_id
                WHERE sp.realm_id = $1
                  AND sp.active = true
                  AND spp.enabled = true
                  AND casp.enabled = true
                  AND ca.enabled = true
            ) AS has_user_visible_plans,
            EXISTS (SELECT 1 FROM points_packages WHERE realm_id = $1) AS has_points_packages,
            EXISTS (
                SELECT 1
                FROM points_package_payment_providers ppp
                JOIN points_packages pp ON pp.id = ppp.points_package_id
                JOIN configured_providers cp ON cp.provider = ppp.payment_provider
                WHERE pp.realm_id = $1 AND pp.enabled = true AND ppp.enabled = true
            ) AS has_points_package_payment_mappings,
            EXISTS (SELECT 1 FROM invoice_seller_config WHERE realm_id = $1) AS has_invoice_seller_config,
            EXISTS (SELECT 1 FROM invoice WHERE realm_id = $1) AS has_invoices,
            EXISTS (SELECT 1 FROM invoice WHERE realm_id = $1 AND applicant_user_id = $2) AS has_user_invoices,
            EXISTS (SELECT 1 FROM subscription_history WHERE realm_id = $1) AS has_subscription_history,
            EXISTS (
                SELECT 1
                FROM subscription s
                LEFT JOIN subscription_history sh ON sh.subscription_id = s.id
                WHERE s.realm_id = $1
                  AND s.user_id = $2
                  AND (sh.id IS NOT NULL OR s.status IN ('active', 'trialing', 'scheduled_cancel'))
            ) AS has_user_subscription_history,
            EXISTS (
                SELECT 1 FROM payment_attempts
                WHERE realm_id = $1 AND user_id = $2 AND status = 'Succeeded'
            ) OR EXISTS (
                SELECT 1 FROM subscription
                WHERE realm_id = $1 AND user_id = $2
            ) AS has_user_invoice_sources
        "#,
    )
    .bind(realm_id)
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(
            realm_id = %realm_id,
            user_id = %user_id,
            error = %e,
            "Failed to load feature availability facts"
        );
        ApiError::internal("Failed to load feature availability")
    })?;

    Ok(FeatureFacts {
        has_payment_providers: row.get("has_payment_providers"),
        has_products: row.get("has_products"),
        has_plans: row.get("has_plans"),
        has_plan_payment_mappings: row.get("has_plan_payment_mappings"),
        has_user_visible_plans: row.get("has_user_visible_plans"),
        has_points_packages: row.get("has_points_packages"),
        has_points_package_payment_mappings: row.get("has_points_package_payment_mappings"),
        has_invoice_seller_config: row.get("has_invoice_seller_config"),
        has_invoices: row.get("has_invoices"),
        has_user_invoices: row.get("has_user_invoices"),
        has_subscription_history: row.get("has_subscription_history"),
        has_user_subscription_history: row.get("has_user_subscription_history"),
        has_user_invoice_sources: row.get("has_user_invoice_sources"),
    })
}
