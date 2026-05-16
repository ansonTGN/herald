// =============================================================================
// Billing Test Helpers
// =============================================================================
//
// Shared helpers for billing-related API tests (subscription_scenarios, plan_scenarios)
//
// =============================================================================

#![allow(dead_code)]

use crate::schema_test_context::SchemaTestContext as TestContext;
use uuid::Uuid;

/// ============================================================================
/// Billing Test Setup Helpers
/// ============================================================================
///
/// Setup admin session for billing tests
pub async fn setup_billing_admin_session(ctx: &mut TestContext, email: &str) -> String {
    let (admin_token, user_id) =
        crate::helpers::create_admin_session_with_user(ctx, email, 1800).await;

    // Grant Realm Admin role
    crate::helpers::grant_realm_admin_role(ctx, &user_id).await;

    admin_token
}

/// ============================================================================
/// Test Data Creation Helpers
/// ============================================================================
///
/// Ensure a default product exists for the given realm and return its ID.
///
/// Creates a default product if one does not already exist for the realm.
/// This is needed because the plan table has a NOT NULL product_id FK column.
pub async fn ensure_default_product(ctx: &mut TestContext, realm_id: &str) -> Uuid {
    // Check if default product already exists for this realm
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM products WHERE realm_id = $1 AND code = 'default' LIMIT 1",
    )
    .bind(realm_id)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .unwrap();

    if let Some(id) = existing {
        return id;
    }

    // Create default product
    let product_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO products (id, realm_id, code, title, description, enabled, created_at, updated_at)
         VALUES ($1, $2, 'default', 'Default Product', 'Default test product', true, NOW(), NOW())"
    )
    .bind(product_id)
    .bind(realm_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create default product");

    product_id
}

/// Create a test plan via direct SQL insertion
///
/// Returns plan_id
pub async fn create_test_plan(ctx: &mut TestContext, realm_id: &str, name: &str) -> Uuid {
    create_test_plan_with_attrs(ctx, realm_id, name, "monthly", 2500).await
}

/// Create a test plan with custom attributes via direct SQL insertion
pub async fn create_test_plan_with_attrs(
    ctx: &mut TestContext,
    realm_id: &str,
    name: &str,
    plan_type: &str,
    price_cents: i64,
) -> Uuid {
    let plan_id = Uuid::now_v7();
    let product_id = ensure_default_product(ctx, realm_id).await;

    sqlx::query(
        "INSERT INTO plan (id, realm_id, name, description, title, type, price, currency,
                          active, trial_days, sort_order, product_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW(), NOW())",
    )
    .bind(plan_id)
    .bind(realm_id)
    .bind(name)
    .bind(format!("{} description", name))
    .bind(name) // title
    .bind(plan_type) // type
    .bind(price_cents) // price in cents
    .bind("USD") // currency
    .bind(true)
    .bind(0)
    .bind(1)
    .bind(product_id) // product_id
    .execute(&ctx.app_state.pool)
    .await
    .unwrap();

    plan_id
}

/// Create a test subscription via direct SQL insertion
///
/// Returns subscription_id
pub async fn create_test_subscription(
    ctx: &mut TestContext,
    realm_id: &str,
    client_app_id: Uuid,
    plan_id: Uuid,
    billing_period: &str,
) -> Uuid {
    let subscription_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO subscription (id, realm_id, plan_id, client_app_id, status, billing_period,
                                 external_subscription_id, external_product_id, payment_provider,
                                 current_period_start, current_period_end,
                                 cancel_at_period_end, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'active', $5,
                 'ext_sub_test', 'test_product', 'creem', NOW(), NOW() + INTERVAL '30 days',
                 false, NOW(), NOW())",
    )
    .bind(subscription_id)
    .bind(realm_id)
    .bind(plan_id)
    .bind(client_app_id)
    .bind(billing_period)
    .execute(&ctx.app_state.pool)
    .await
    .unwrap();

    subscription_id
}

/// Delete a subscription via SQL (for cleanup)
pub async fn delete_test_subscription(ctx: &mut TestContext, subscription_id: Uuid) {
    sqlx::query("DELETE FROM subscription WHERE id = $1")
        .bind(subscription_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
}

/// Delete subscriptions by client app ID (for cleanup)
pub async fn delete_subscriptions_by_client_app(ctx: &mut TestContext, client_app_id: Uuid) {
    sqlx::query("DELETE FROM subscription WHERE client_app_id = $1")
        .bind(client_app_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
}

/// ============================================================================
/// Client App Creation Helper
/// ============================================================================
///
/// JSON payload for creating a basic client app
pub fn client_app_create_json(client_id: &str, name: &str, redirect_uris: &[&str]) -> String {
    use serde_json::json;

    let payload = json!({
        "clientId": client_id,
        "name": name,
        "redirectUris": redirect_uris,
        "enabled": true
    });

    payload.to_string()
}
