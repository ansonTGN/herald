// =============================================================================
// Points Package Test Helpers
// =============================================================================
//
// Shared helpers for points package API tests.
// Provides functions for creating points packages, payment provider mappings,
// and purchase flow testing.
//
// =============================================================================

#![allow(dead_code)]

use crate::tests::schema_test_context::SchemaTestContext;
use sqlx::Row;
use uuid::Uuid; // Required for row.get() method

/// ============================================================================
/// Points Package Creation Helpers
/// ============================================================================
/// Create a points package via direct SQL insertion
///
/// Returns the package_id
pub async fn create_points_package(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    name: &str,
    title: &str,
    points: i64,
    price: i64,
    currency: &str,
    enabled: bool,
) -> Uuid {
    let package_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO points_packages (id, realm_id, name, title, description, points, price, currency, sort_order, enabled, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, $9, NOW(), NOW())"
    )
    .bind(package_id)
    .bind(realm_id)
    .bind(name)
    .bind(title)
    .bind(format!("{} description", title)) // description
    .bind(points)
    .bind(price)
    .bind(currency)
    .bind(enabled)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create points package");

    package_id
}

/// Create a points package with custom description
pub async fn create_points_package_with_description(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    name: &str,
    title: &str,
    description: &str,
    points: i64,
    price: i64,
    currency: &str,
    enabled: bool,
) -> Uuid {
    let package_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO points_packages (id, realm_id, name, title, description, points, price, currency, sort_order, enabled, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, $9, NOW(), NOW())"
    )
    .bind(package_id)
    .bind(realm_id)
    .bind(name)
    .bind(title)
    .bind(description)
    .bind(points)
    .bind(price)
    .bind(currency)
    .bind(enabled)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create points package with description");

    package_id
}

/// ============================================================================
/// Payment Provider Mapping Helpers
/// ============================================================================
/// Create a payment provider mapping via direct SQL insertion
///
/// Returns the mapping_id
pub async fn create_payment_provider_mapping(
    ctx: &mut SchemaTestContext,
    points_package_id: Uuid,
    payment_provider: &str,
    external_product_id: Option<&str>,
    enabled: bool,
) -> Uuid {
    let mapping_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO points_package_payment_providers (id, points_package_id, payment_provider, external_product_id, enabled, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, NOW(), NOW())"
    )
    .bind(mapping_id)
    .bind(points_package_id)
    .bind(payment_provider)
    .bind(external_product_id)
    .bind(enabled)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create payment provider mapping");

    mapping_id
}

/// ============================================================================
/// Query Helpers
/// ============================================================================
/// Get points package by ID
///
/// Returns (name, title, points, price, currency, enabled)
pub async fn get_points_package(
    ctx: &SchemaTestContext,
    package_id: Uuid,
) -> Option<(String, String, i64, i64, String, bool)> {
    sqlx::query(
        "SELECT name, title, points, price, currency, enabled FROM points_packages WHERE id = $1",
    )
    .bind(package_id)
    .fetch_optional(&ctx.app_state.pool)
    .await
    .unwrap()
    .map(|row| {
        (
            row.get("name"),
            row.get("title"),
            row.get("points"),
            row.get("price"),
            row.get("currency"),
            row.get("enabled"),
        )
    })
}

/// List all points packages for a realm
pub async fn list_points_packages(
    ctx: &SchemaTestContext,
    realm_id: &str,
) -> Vec<(Uuid, String, String, i64, i64, String, bool)> {
    sqlx::query(
        "SELECT id, name, title, points, price, currency, enabled FROM points_packages WHERE realm_id = $1 ORDER BY sort_order, created_at"
    )
    .bind(realm_id)
    .fetch_all(&ctx.app_state.pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (
        row.get("id"),
        row.get("name"),
        row.get("title"),
        row.get("points"),
        row.get("price"),
        row.get("currency"),
        row.get("enabled"),
    ))
    .collect()
}

/// Get payment provider mappings for a points package
pub async fn get_payment_provider_mappings(
    ctx: &SchemaTestContext,
    points_package_id: Uuid,
) -> Vec<(Uuid, String, Option<String>, bool)> {
    sqlx::query(
        "SELECT id, payment_provider, external_product_id, enabled FROM points_package_payment_providers WHERE points_package_id = $1"
    )
    .bind(points_package_id)
    .fetch_all(&ctx.app_state.pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (
        row.get("id"),
        row.get("payment_provider"),
        row.get("external_product_id"),
        row.get("enabled"),
    ))
    .collect()
}

/// ============================================================================
/// Purchase Flow Helpers
/// ============================================================================
/// Create a payment attempt via direct SQL insertion
///
/// Returns the attempt_id
pub async fn create_payment_attempt(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    user_id: Uuid,
    target_type: &str,
    target_id: Uuid,
    payment_provider: &str,
    amount: i64,
    currency: &str,
) -> Uuid {
    let attempt_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO payment_attempts (id, realm_id, user_id, target_type, target_id, payment_provider, amount, currency, status, expires_at, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'Pending', NOW() + INTERVAL '30 minutes', NOW(), NOW())"
    )
    .bind(attempt_id)
    .bind(realm_id)
    .bind(user_id)
    .bind(target_type)
    .bind(target_id)
    .bind(payment_provider)
    .bind(amount)
    .bind(currency)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create payment attempt");

    attempt_id
}

/// Create a points package purchase record
///
/// Returns the purchase_id
pub async fn create_points_package_purchase(
    ctx: &mut SchemaTestContext,
    realm_id: &str,
    user_id: Uuid,
    points_package_id: Uuid,
    points: i64,
    amount: i64,
    currency: &str,
    payment_provider: &str,
    payment_attempt_id: Uuid,
) -> Uuid {
    let purchase_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO points_package_purchases (id, realm_id, user_id, points_package_id, points, amount, currency, payment_provider, payment_attempt_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())"
    )
    .bind(purchase_id)
    .bind(realm_id)
    .bind(user_id)
    .bind(points_package_id)
    .bind(points)
    .bind(amount)
    .bind(currency)
    .bind(payment_provider)
    .bind(payment_attempt_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create points package purchase");

    purchase_id
}

/// ============================================================================
/// Cleanup Helpers
/// ============================================================================
/// Delete a points package (will fail if purchases exist)
pub async fn delete_points_package(ctx: &mut SchemaTestContext, package_id: Uuid) {
    sqlx::query("DELETE FROM points_packages WHERE id = $1")
        .bind(package_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
}

/// Delete payment provider mappings for a points package
pub async fn delete_payment_provider_mappings(
    ctx: &mut SchemaTestContext,
    points_package_id: Uuid,
) {
    sqlx::query("DELETE FROM points_package_payment_providers WHERE points_package_id = $1")
        .bind(points_package_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
}

/// Delete payment attempts for a user
pub async fn delete_payment_attempts_for_user(ctx: &mut SchemaTestContext, user_id: Uuid) {
    sqlx::query("DELETE FROM payment_attempts WHERE user_id = $1")
        .bind(user_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
}

/// Delete points package purchases for a user
pub async fn delete_purchases_for_user(ctx: &mut SchemaTestContext, user_id: Uuid) {
    sqlx::query("DELETE FROM points_package_purchases WHERE user_id = $1")
        .bind(user_id)
        .execute(&ctx.app_state.pool)
        .await
        .unwrap();
}

/// ============================================================================
/// Assertion Helpers
/// ============================================================================
/// Assert points package exists with expected values
pub async fn assert_points_package_exists(
    ctx: &SchemaTestContext,
    package_id: Uuid,
    expected_name: &str,
    expected_points: i64,
    expected_price: i64,
) {
    let package = get_points_package(ctx, package_id).await;

    assert!(
        package.is_some(),
        "Points package {} should exist",
        package_id
    );

    let (name, _title, points, price, _currency, _enabled) = package.unwrap();
    assert_eq!(name, expected_name, "Package name mismatch");
    assert_eq!(points, expected_points, "Package points mismatch");
    assert_eq!(price, expected_price, "Package price mismatch");
}

/// Assert payment provider mapping exists
pub async fn assert_payment_provider_mapping_exists(
    ctx: &SchemaTestContext,
    points_package_id: Uuid,
    payment_provider: &str,
    expected_enabled: bool,
) {
    let mappings = get_payment_provider_mappings(ctx, points_package_id).await;

    let found = mappings
        .iter()
        .any(|(_id, provider, _external_id, enabled)| {
            provider == payment_provider && *enabled == expected_enabled
        });

    assert!(
        found,
        "Payment provider mapping {} for package {} should exist with enabled={}",
        payment_provider, points_package_id, expected_enabled
    );
}

/// Assert payment attempt exists with expected status
pub async fn assert_payment_attempt_exists(
    ctx: &SchemaTestContext,
    attempt_id: Uuid,
    expected_status: &str,
) {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM payment_attempts WHERE id = $1")
            .bind(attempt_id)
            .fetch_optional(&ctx.app_state.pool)
            .await
            .unwrap();

    assert!(
        status.is_some(),
        "Payment attempt {} should exist",
        attempt_id
    );
    assert_eq!(
        status.unwrap(),
        expected_status,
        "Payment attempt status mismatch"
    );
}
