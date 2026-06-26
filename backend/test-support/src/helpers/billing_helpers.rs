// =============================================================================
// Billing Test Helpers
// =============================================================================
//
// Shared helpers for billing-related API tests.
// Adapted for product_reduce: subscription uses entitlement_key instead of
// plan_id/tier/billing_period; Product/Plan helpers removed.
//
// =============================================================================

#![allow(dead_code)]

use crate::schema_test_context::SchemaTestContext as TestContext;
use herald_core::domain::authentication::Identity;
use herald_core::domain::user::entities::{User, UserStatus};
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
/// Entitlement Mapping Test Data Creation Helpers
/// ============================================================================
///
/// Create a test entitlement mapping via direct SQL insertion.
///
/// Returns the mapping ID.
pub async fn setup_test_entitlement_mapping(
    ctx: &mut TestContext,
    realm_id: &str,
    payment_provider: &str,
    external_product_id: &str,
    entitlement_key: &str,
) -> Uuid {
    let mapping_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO provider_entitlement_mappings
            (id, realm_id, payment_provider, external_product_id, entitlement_key,
             grant_on_subscribe, enabled, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, false, false, NOW(), NOW())",
    )
    .bind(mapping_id)
    .bind(realm_id)
    .bind(payment_provider)
    .bind(external_product_id)
    .bind(entitlement_key)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test entitlement mapping");

    mapping_id
}

/// Create a test entitlement mapping with full points policy via direct SQL insertion.
///
/// Returns the mapping ID.
#[allow(clippy::too_many_arguments)]
pub async fn setup_test_entitlement_mapping_with_points(
    ctx: &mut TestContext,
    realm_id: &str,
    payment_provider: &str,
    external_product_id: &str,
    entitlement_key: &str,
    points_per_period: i64,
    grant_on_subscribe: bool,
    enabled: bool,
) -> Uuid {
    let mapping_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO provider_entitlement_mappings
            (id, realm_id, payment_provider, external_product_id, entitlement_key,
             points_per_period, grant_on_subscribe, enabled, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())",
    )
    .bind(mapping_id)
    .bind(realm_id)
    .bind(payment_provider)
    .bind(external_product_id)
    .bind(entitlement_key)
    .bind(points_per_period)
    .bind(grant_on_subscribe)
    .bind(enabled)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test entitlement mapping with points");

    mapping_id
}

/// Ensure the realm has a legacy credit bucket and return its id.
///
/// `subscription.bucket_id` is NOT NULL (eager binding), so direct-SQL
/// subscription inserts must bind a real bucket. This find-or-creates the
/// realm's legacy test bucket (deterministic slug, idempotent). Mirrors
/// `points_helpers::ensure_test_bucket_for_realm` in the api crate, which
/// test-support cannot depend on.
async fn ensure_test_bucket_for_realm(pool: &sqlx::PgPool, realm_id: &str) -> Uuid {
    use sqlx::Row;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    realm_id.hash(&mut hasher);
    let slug = format!("legacy-{:016x}", hasher.finish());

    sqlx::query(
        r#"INSERT INTO credit_buckets
             (id, realm_id, bucket_key, name, display_order, enabled,
              receives_registration_credits, created_at, updated_at)
           VALUES ($1, $2, $3, 'Legacy Test Bucket', 0, true, false, NOW(), NOW())
           ON CONFLICT (realm_id, bucket_key) DO NOTHING"#,
    )
    .bind(Uuid::now_v7())
    .bind(realm_id)
    .bind(&slug)
    .execute(pool)
    .await
    .expect("Failed to ensure legacy credit bucket");

    let row = sqlx::query("SELECT id FROM credit_buckets WHERE realm_id = $1 AND bucket_key = $2")
        .bind(realm_id)
        .bind(&slug)
        .fetch_one(pool)
        .await
        .expect("Failed to fetch legacy credit bucket");
    row.get::<Uuid, _>("id")
}

/// Create a test entitlement mapping at **price granularity** via direct SQL
/// insertion (`provider_entitlement_mappings` moved from
/// product to price key).
///
/// Binds `external_price_id`, `points_per_period`, `grant_on_subscribe`,
/// `enabled`, AND the NOT-NULL `bucket_id` (the legacy
/// `setup_test_entitlement_mapping*` helpers omit `bucket_id` and
/// `external_price_id`, so they cannot seed a row the price-aware webhook
/// resolver would actually hit). The bucket is the realm's registration-pool
/// bucket (caller must have run `ensure_registration_pool_bucket` first).
///
/// Returns the mapping ID.
#[allow(clippy::too_many_arguments)]
pub async fn setup_test_price_mapping(
    ctx: &mut TestContext,
    realm_id: &str,
    payment_provider: &str,
    external_product_id: &str,
    external_price_id: Option<&str>,
    entitlement_key: &str,
    points_per_period: i64,
    grant_on_subscribe: bool,
    enabled: bool,
    bucket_id: Uuid,
) -> Uuid {
    let mapping_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO provider_entitlement_mappings
            (id, realm_id, payment_provider, external_product_id, external_price_id,
             bucket_id, entitlement_key, points_per_period,
             grant_on_subscribe, enabled, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW())",
    )
    .bind(mapping_id)
    .bind(realm_id)
    .bind(payment_provider)
    .bind(external_product_id)
    .bind(external_price_id)
    .bind(bucket_id)
    .bind(entitlement_key)
    .bind(points_per_period)
    .bind(grant_on_subscribe)
    .bind(enabled)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create price-granularity entitlement mapping");

    mapping_id
}

/// Create a test subscription with entitlement_key via direct SQL insertion.
/// Uses the new schema (entitlement_key, external_price_id, provider_metadata).
/// Returns the subscription ID.
pub async fn create_test_subscription_with_entitlement(
    ctx: &mut TestContext,
    realm_id: &str,
    client_app_id: Uuid,
    entitlement_key: &str,
    external_price_id: &str,
) -> Uuid {
    let subscription_id = Uuid::now_v7();
    let external_subscription_id = format!("sub_test_{}", subscription_id);

    // subscription.bucket_id is NOT NULL (eager binding); bind the realm's
    // legacy test bucket so the direct-SQL insert satisfies the constraint.
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;

    sqlx::query(
        "INSERT INTO subscription
            (id, realm_id, client_app_id, status, entitlement_key, external_price_id,
             external_subscription_id, external_product_id, payment_provider,
             current_period_start, current_period_end,
             cancel_at_period_end, created_at, updated_at, bucket_id)
         VALUES ($1, $2, $3, 'active', $4, $5,
                 $6, $7, 'creem', NOW(), NOW() + INTERVAL '30 days',
                 false, NOW(), NOW(), $8)",
    )
    .bind(subscription_id)
    .bind(realm_id)
    .bind(client_app_id)
    .bind(entitlement_key)
    .bind(external_price_id)
    .bind(&external_subscription_id)
    .bind(format!("prod_{}", subscription_id))
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test subscription with entitlement");

    subscription_id
}

/// Ensure the realm has a **registration-pool** credit bucket and return its id.
///
/// Distinct from `ensure_test_bucket_for_realm` (which sets
/// `receives_registration_credits = false`): the provider-product sync service
/// resolves draft mappings' `bucket_id` (NOT NULL) through
/// `RegistrationPoolResolver::resolve_registration_pool_bucket`, which selects
/// the single bucket flagged `receives_registration_credits = true`. The
/// template schema seeds no such bucket, so any sync that creates a new draft
/// mapping would otherwise fail with `BadRequest`. This helper is the test-side
/// equivalent of an operator configuring the realm's registration pool.
pub async fn ensure_registration_pool_bucket(ctx: &mut TestContext, realm_id: &str) -> Uuid {
    sqlx::query(
        r#"INSERT INTO credit_buckets
             (id, realm_id, bucket_key, name, display_order, enabled,
              receives_registration_credits, created_at, updated_at)
           VALUES ($1, $2, $3, 'Registration Pool', 0, true, true, NOW(), NOW())
           ON CONFLICT (realm_id, bucket_key) DO NOTHING"#,
    )
    .bind(Uuid::now_v7())
    .bind(realm_id)
    .bind("registration-pool")
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to ensure registration-pool credit bucket");

    use sqlx::Row;
    let row = sqlx::query("SELECT id FROM credit_buckets WHERE realm_id = $1 AND bucket_key = $2")
        .bind(realm_id)
        .bind("registration-pool")
        .fetch_one(&ctx.app_state.pool)
        .await
        .expect("Failed to fetch registration-pool credit bucket");
    row.get::<Uuid, _>("id")
}

/// Build an `Identity::User` for the given user id + the context's realm.
///
/// Used by scenario tests that drive a domain service directly (bypassing the
/// HTTP layer) and therefore need a real `Identity` for the policy + realm
/// access checks. `status = Normal` and a placeholder email keep it valid; the
/// load-bearing field is `realm_id` (must equal the test realm so
/// `has_access_to_realm` passes).
pub fn build_admin_identity(ctx: &TestContext, user_id: Uuid) -> Identity {
    Identity::User(User {
        id: user_id,
        realm_id: ctx._realm_id.clone(),
        email: "sync-operator@test.local".to_string(),
        nickname: None,
        password_hash: None,
        provider_ids: vec![],
        status: UserStatus::Normal,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    })
}

/// Create an `active` subscription bound to a SPECIFIC price mapping's
/// `(realm, provider, external_product_id, external_price_id)` triple.
///
/// Unlike `create_test_subscription_with_entitlement` (which synthesizes its
/// own `external_product_id` and uses `client_app_id`), this helper binds the
/// subscription to the exact product/price the batch active-subscription lock
/// (`batch_update_mappings` step 3) counts against, and uses a NULL
/// `client_app_id` to avoid the `uq_subscription_client_app` unique constraint
/// (so multiple protected mappings can coexist in one realm). The mapping's
/// `bucket_id` is reused so the NOT-NULL `subscription.bucket_id` is satisfied
/// without inventing a second bucket.
///
/// Returns the subscription ID.
#[allow(clippy::too_many_arguments)]
pub async fn create_active_subscription_for_price_mapping(
    ctx: &mut TestContext,
    realm_id: &str,
    payment_provider: &str,
    external_product_id: &str,
    external_price_id: Option<&str>,
    entitlement_key: &str,
    bucket_id: Uuid,
) -> Uuid {
    let subscription_id = Uuid::now_v7();
    let external_subscription_id = format!("sub_protected_{}", subscription_id);

    // NULL client_app_id avoids uq_subscription_client_app; the active-sub
    // count query filters by (realm, provider, product, price), NOT by app.
    sqlx::query(
        "INSERT INTO subscription \
            (id, realm_id, client_app_id, status, entitlement_key, external_price_id, \
             external_subscription_id, external_product_id, payment_provider, \
             current_period_start, current_period_end, cancel_at_period_end, \
             created_at, updated_at, bucket_id) \
         VALUES ($1, $2, NULL, 'active', $3, $4, $5, $6, $7, \
                 NOW(), NOW() + INTERVAL '30 days', false, NOW(), NOW(), $8)",
    )
    .bind(subscription_id)
    .bind(realm_id)
    .bind(entitlement_key)
    .bind(external_price_id)
    .bind(&external_subscription_id)
    .bind(external_product_id)
    .bind(payment_provider)
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create active subscription bound to price mapping");

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
