// =============================================================================
// Subscription Test Helpers
// =============================================================================
//
// Helpers for subscription CRUD with the new schema (entitlement_key,
// external_price_id, provider_metadata, synced_at).
//
// =============================================================================

#![allow(dead_code)]

use crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm;
use crate::tests::schema_test_context::SchemaTestContext;
use sqlx::Row;
use uuid::Uuid;

/// Create a test subscription with all new fields via direct SQL insertion.
///
/// Returns the subscription ID.
#[allow(clippy::too_many_arguments)]
pub async fn create_test_subscription_with_entitlement_key(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    client_app_id: Uuid,
    entitlement_key: &str,
    external_price_id: &str,
    payment_provider: &str,
    status: &str,
) -> Uuid {
    let subscription_id = Uuid::now_v7();
    let external_subscription_id = format!("sub_test_{}", subscription_id);
    let external_product_id = format!("prod_{}", entitlement_key);

    // subscription.bucket_id is NOT NULL (eager binding); bind the realm's
    // legacy test bucket so direct-SQL inserts satisfy the constraint.
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;

    sqlx::query(
        "INSERT INTO subscription
            (id, realm_id, client_app_id, status, entitlement_key, external_price_id,
             external_subscription_id, external_product_id, payment_provider,
             current_period_start, current_period_end,
             cancel_at_period_end, created_at, updated_at, bucket_id)
         VALUES ($1, $2, $3, $4, $5, $6,
                 $7, $8, $9, NOW(), NOW() + INTERVAL '30 days',
                 false, NOW(), NOW(), $10)",
    )
    .bind(subscription_id)
    .bind(realm_id)
    .bind(client_app_id)
    .bind(status)
    .bind(entitlement_key)
    .bind(external_price_id)
    .bind(&external_subscription_id)
    .bind(&external_product_id)
    .bind(payment_provider)
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test subscription with entitlement_key");

    subscription_id
}

/// Create a test subscription with provider_metadata and synced_at via direct SQL.
///
/// Returns the subscription ID.
#[allow(clippy::too_many_arguments)]
pub async fn create_test_subscription_full(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    client_app_id: Uuid,
    entitlement_key: &str,
    external_price_id: &str,
    payment_provider: &str,
    status: &str,
    provider_metadata: Option<serde_json::Value>,
) -> Uuid {
    let subscription_id = Uuid::now_v7();
    let external_subscription_id = format!("sub_test_{}", subscription_id);
    let external_product_id = format!("prod_{}", entitlement_key);

    // subscription.bucket_id is NOT NULL (eager binding); bind the realm's
    // legacy test bucket so direct-SQL inserts satisfy the constraint.
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;

    sqlx::query(
        "INSERT INTO subscription
            (id, realm_id, client_app_id, status, entitlement_key, external_price_id,
             external_subscription_id, external_product_id, payment_provider,
             current_period_start, current_period_end,
             provider_metadata, synced_at,
             cancel_at_period_end, created_at, updated_at, bucket_id)
         VALUES ($1, $2, $3, $4, $5, $6,
                 $7, $8, $9, NOW(), NOW() + INTERVAL '30 days',
                 $10, NOW(),
                 false, NOW(), NOW(), $11)",
    )
    .bind(subscription_id)
    .bind(realm_id)
    .bind(client_app_id)
    .bind(status)
    .bind(entitlement_key)
    .bind(external_price_id)
    .bind(&external_subscription_id)
    .bind(&external_product_id)
    .bind(payment_provider)
    .bind(provider_metadata)
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create full test subscription");

    subscription_id
}

/// Verify that a subscription has the expected entitlement_key.
pub async fn verify_subscription_entitlement_key(
    ctx: &SchemaTestContext,
    subscription_id: Uuid,
    expected_entitlement_key: &str,
) {
    let actual: String =
        sqlx::query_scalar("SELECT entitlement_key FROM subscription WHERE id = $1")
            .bind(subscription_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .expect("Subscription not found");

    assert_eq!(
        actual, expected_entitlement_key,
        "Subscription entitlement_key mismatch"
    );
}

/// Verify that old plan fields (plan_id, tier, billing_period) do not exist
/// in the subscription table schema.
pub async fn verify_subscription_has_no_plan_fields(ctx: &SchemaTestContext) {
    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name FROM information_schema.columns
         WHERE table_name = 'subscription'
         ORDER BY ordinal_position",
    )
    .fetch_all(&ctx.app_state.pool)
    .await
    .unwrap();

    for old_col in &["plan_id", "tier", "billing_period"] {
        assert!(
            !columns.iter().any(|c| c == old_col),
            "Old column '{}' should not exist in subscription table",
            old_col
        );
    }
}

/// Verify that a subscription has the expected external_price_id.
pub async fn verify_subscription_external_price_id(
    ctx: &SchemaTestContext,
    subscription_id: Uuid,
    expected: &str,
) {
    let actual: Option<String> =
        sqlx::query_scalar("SELECT external_price_id FROM subscription WHERE id = $1")
            .bind(subscription_id)
            .fetch_optional(&ctx.app_state.pool)
            .await
            .unwrap()
            .flatten();

    assert_eq!(
        actual,
        Some(expected.to_string()),
        "Subscription external_price_id mismatch"
    );
}

/// Verify that a subscription has provider_metadata set.
pub async fn verify_subscription_has_provider_metadata(
    ctx: &SchemaTestContext,
    subscription_id: Uuid,
) {
    let actual: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT provider_metadata FROM subscription WHERE id = $1")
            .bind(subscription_id)
            .fetch_optional(&ctx.app_state.pool)
            .await
            .unwrap()
            .flatten();

    assert!(
        actual.is_some(),
        "Subscription should have provider_metadata"
    );
}

/// Verify that a subscription has synced_at set.
pub async fn verify_subscription_has_synced_at(ctx: &SchemaTestContext, subscription_id: Uuid) {
    let actual: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT synced_at FROM subscription WHERE id = $1")
            .bind(subscription_id)
            .fetch_optional(&ctx.app_state.pool)
            .await
            .unwrap()
            .flatten();

    assert!(actual.is_some(), "Subscription should have synced_at");
}

/// Get subscription history events for a subscription.
///
/// Returns Vec of (event_type, changes_json).
pub async fn get_subscription_history_events(
    ctx: &SchemaTestContext,
    subscription_id: Uuid,
) -> Vec<(String, Option<serde_json::Value>)> {
    let rows = sqlx::query(
        "SELECT event_type, changes FROM subscription_history
         WHERE subscription_id = $1
         ORDER BY created_at ASC",
    )
    .bind(subscription_id)
    .fetch_all(&ctx.app_state.pool)
    .await
    .unwrap();

    rows.into_iter()
        .map(|row| {
            let event_type: String = row.get("event_type");
            let changes: Option<serde_json::Value> = row.get("changes");
            (event_type, changes)
        })
        .collect()
}

/// Count subscriptions by entitlement_key in a realm.
pub async fn count_subscriptions_by_entitlement_key(
    ctx: &SchemaTestContext,
    realm_id: &str,
    entitlement_key: &str,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM subscription WHERE realm_id = $1 AND entitlement_key = $2",
    )
    .bind(realm_id)
    .bind(entitlement_key)
    .fetch_one(&ctx.app_state.pool)
    .await
    .unwrap()
}

/// Delete a subscription by ID (cleanup helper).
pub async fn delete_test_subscription(ctx: &SchemaTestContext, subscription_id: Uuid) {
    sqlx::query("DELETE FROM subscription WHERE id = $1")
        .bind(subscription_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
}
