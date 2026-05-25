// =============================================================================
// Points Package Promo CRUD Validation Scenario Tests
// =============================================================================
//
// Tests for promotional points package CRUD validation:
// 1. Creating promo packages with originalPrice, time ranges
// 2. Validation: originalPrice must be > price, standard cannot have originalPrice
// 3. Type conversion: promo <-> standard with field clearing/applying
// 4. Updating price, promo time ranges
// 5. Expired promo packages visible to admin
//
// User Story: docs/user-stories/06-billing-user-stories.md (US-PP-006, US-PP-007, US-PP-009)
//
// =============================================================================

use crate::tests::helpers::auth_helpers::{create_admin_session_with_user, grant_realm_admin_role};
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::json;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::helpers::client_helpers::create_test_api_key;
    use test_context::test_context;
    use tower::ServiceExt;

    /// ============================================================================
    /// US-PP-006 Scenario 1,3: Create promo with originalPrice > price, verify discountPercent
    /// ============================================================================
    /// User Story: docs/user-stories/06-billing-user-stories.md
    /// Covers: US-PP-006 Scenario 1,3
    ///
    /// Given: A realm admin exists
    /// When: Creating a promotional package with originalPrice > price
    /// Then: Response is 201 with correct discountPercent calculated
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_create_success(ctx: &mut TestContext) {
        let app = ctx.create_unified_test_router();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "promo-create@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", ctx._realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "promo-pack-1",
                            "title": "Promo Pack",
                            "points": 1000,
                            "price": 4990,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "promotional",
                            "originalPrice": 9990
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(body_json["packageType"], "promotional");
        assert_eq!(body_json["originalPrice"], 9990);
        assert_eq!(body_json["price"], 4990);

        // discountPercent = round((9990 - 4990) / 9990 * 100) = round(50.05) = 50
        let discount = body_json["discountPercent"]
            .as_i64()
            .expect("discountPercent should be an integer");
        assert!(
            (49..=51).contains(&discount),
            "discountPercent should be ~50, got {}",
            discount
        );
    }

    /// ============================================================================
    /// US-PP-006 Scenario 2: originalPrice == price rejected
    /// ============================================================================
    /// User Story: docs/user-stories/06-billing-user-stories.md
    /// Covers: US-PP-006 Scenario 2
    ///
    /// Given: A realm admin exists
    /// When: Creating a promotional package with originalPrice == price
    /// Then: Response is 400 with validation error
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_create_original_price_equal_rejected(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "promo-equal@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", ctx._realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "promo-equal",
                            "title": "Promo Equal",
                            "points": 500,
                            "price": 9900,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "promotional",
                            "originalPrice": 9900
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_text = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            body_text.contains("must be greater than selling price"),
            "Should reject originalPrice == price, got: {body_text}"
        );
    }

    /// ============================================================================
    /// US-PP-006 Scenario 2: originalPrice < price rejected
    /// ============================================================================
    /// User Story: docs/user-stories/06-billing-user-stories.md
    /// Covers: US-PP-006 Scenario 2
    ///
    /// Given: A realm admin exists
    /// When: Creating a promotional package with originalPrice < price
    /// Then: Response is 400 with validation error
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_create_original_price_less_rejected(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "promo-less@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", ctx._realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "promo-less",
                            "title": "Promo Less",
                            "points": 500,
                            "price": 9900,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "promotional",
                            "originalPrice": 5000
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_text = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            body_text.contains("must be greater than selling price"),
            "Should reject originalPrice < price, got: {body_text}"
        );
    }

    /// ============================================================================
    /// US-PP-006 Scenario 4: Standard package cannot have originalPrice
    /// ============================================================================
    /// User Story: docs/user-stories/06-billing-user-stories.md
    /// Covers: US-PP-006 Scenario 4
    ///
    /// Given: A realm admin exists
    /// When: Creating a standard package with originalPrice set
    /// Then: Response is 400 with validation error
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_create_standard_with_original_price_rejected(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "std-origprice@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", ctx._realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "std-with-orig",
                            "title": "Standard With OrigPrice",
                            "points": 500,
                            "price": 9900,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "standard",
                            "originalPrice": 19900
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_text = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            body_text.contains("Standard package cannot have original price"),
            "Should reject standard with originalPrice, got: {body_text}"
        );
    }

    /// ============================================================================
    /// US-PP-006 Scenario 1: Promo with start/end time range
    /// ============================================================================
    /// User Story: docs/user-stories/06-billing-user-stories.md
    /// Covers: US-PP-006 Scenario 1
    ///
    /// Given: A realm admin exists
    /// When: Creating a promotional package with valid start and end times
    /// Then: Response is 201 with promo times reflected
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_create_with_time_range_success(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "promo-time@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        let now = chrono::Utc::now();
        let start = now + chrono::Duration::hours(1);
        let end = now + chrono::Duration::days(7);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", ctx._realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "promo-timed",
                            "title": "Promo Timed",
                            "points": 2000,
                            "price": 5990,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "promotional",
                            "originalPrice": 12990,
                            "promoStartTime": start.to_rfc3339(),
                            "promoEndTime": end.to_rfc3339()
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(body_json["packageType"], "promotional");
        assert!(
            body_json["promoStartTime"].is_string(),
            "promoStartTime should be a string"
        );
        assert!(
            body_json["promoEndTime"].is_string(),
            "promoEndTime should be a string"
        );
        assert!(
            body_json["discountPercent"].is_number(),
            "discountPercent should be present"
        );
        // Promo is in the future (starts in 1h), so isExpired should be false
        // (the promo has not expired, but is also not yet active)
        assert_eq!(body_json["isExpired"], false);
    }

    /// ============================================================================
    /// Design doc validation: end time < start time rejected
    /// ============================================================================
    /// User Story: docs/user-stories/06-billing-user-stories.md
    /// Covers: US-PP-006 design doc validation
    ///
    /// Given: A realm admin exists
    /// When: Creating a promotional package where endTime < startTime
    /// Then: Response is 400 with validation error
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_create_invalid_time_range_rejected(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "promo-invalid-time@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        let now = chrono::Utc::now();
        let start = now + chrono::Duration::days(7);
        let end = now + chrono::Duration::hours(1);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", ctx._realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "promo-invalid-range",
                            "title": "Promo Invalid Range",
                            "points": 1000,
                            "price": 4990,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "promotional",
                            "originalPrice": 9990,
                            "promoStartTime": start.to_rfc3339(),
                            "promoEndTime": end.to_rfc3339()
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_text = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            body_text.contains("end time must be after start time"),
            "Should reject endTime < startTime, got: {body_text}"
        );
    }

    /// ============================================================================
    /// US-PP-007 Scenario 2: Promo -> Standard clears promo fields
    /// ============================================================================
    /// User Story: docs/user-stories/06-billing-user-stories.md
    /// Covers: US-PP-007 Scenario 2
    ///
    /// Given: A promotional package exists with originalPrice and time range
    /// When: Updating it to standard type
    /// Then: Promo fields (originalPrice, promoStartTime, promoEndTime) are cleared
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_update_to_standard_fields_cleared(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "promo-to-std@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        // Given: Create a promotional package first
        let now = chrono::Utc::now();
        let start = now - chrono::Duration::hours(1);
        let end = now + chrono::Duration::days(7);

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", ctx._realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "promo-then-std",
                            "title": "Promo Then Standard",
                            "points": 800,
                            "price": 3990,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "promotional",
                            "originalPrice": 8990,
                            "promoStartTime": start.to_rfc3339(),
                            "promoEndTime": end.to_rfc3339()
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::CREATED);
        let create_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let create_json: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
        let package_id = create_json["id"].as_str().unwrap();

        // When: Update to standard type
        let update_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!(
                        "/api/bill/{}/points-packages/{}",
                        ctx._realm_id, package_id
                    ))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "packageType": "standard"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(update_response.status(), StatusCode::OK);

        // Then: Promo fields are cleared
        let update_body = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let update_json: serde_json::Value = serde_json::from_slice(&update_body).unwrap();

        assert_eq!(update_json["packageType"], "standard");
        assert!(
            update_json["originalPrice"].is_null(),
            "originalPrice should be null after converting to standard"
        );
        assert!(
            update_json["promoStartTime"].is_null(),
            "promoStartTime should be null after converting to standard"
        );
        assert!(
            update_json["promoEndTime"].is_null(),
            "promoEndTime should be null after converting to standard"
        );
        assert!(
            update_json["discountPercent"].is_null(),
            "discountPercent should be null after converting to standard"
        );
        assert!(
            update_json["isExpired"].is_null(),
            "isExpired should be null after converting to standard"
        );
    }

    /// ============================================================================
    /// US-PP-007 Scenario 3: Standard -> Promo applies promo fields
    /// ============================================================================
    /// User Story: docs/user-stories/06-billing-user-stories.md
    /// Covers: US-PP-007 Scenario 3
    ///
    /// Given: A standard package exists
    /// When: Updating it to promotional type with originalPrice and time range
    /// Then: Promo fields are applied and discountPercent is calculated
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_update_to_promo_fields_applied(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "std-to-promo@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        // Given: Create a standard package
        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", ctx._realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "std-then-promo",
                            "title": "Standard Then Promo",
                            "points": 1500,
                            "price": 7990,
                            "currency": "CNY",
                            "enabled": true
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::CREATED);
        let create_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let create_json: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
        let package_id = create_json["id"].as_str().unwrap();

        // When: Update to promotional type with promo fields
        let now = chrono::Utc::now();
        let start = now + chrono::Duration::hours(1);
        let end = now + chrono::Duration::days(14);

        let update_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!(
                        "/api/bill/{}/points-packages/{}",
                        ctx._realm_id, package_id
                    ))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "packageType": "promotional",
                            "originalPrice": 15990,
                            "promoStartTime": start.to_rfc3339(),
                            "promoEndTime": end.to_rfc3339()
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(update_response.status(), StatusCode::OK);

        // Then: Promo fields are applied
        let update_body = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let update_json: serde_json::Value = serde_json::from_slice(&update_body).unwrap();

        assert_eq!(update_json["packageType"], "promotional");
        assert_eq!(update_json["originalPrice"], 15990);
        assert!(update_json["promoStartTime"].is_string());
        assert!(update_json["promoEndTime"].is_string());
        assert!(
            update_json["discountPercent"].is_number(),
            "discountPercent should be calculated"
        );
    }

    /// ============================================================================
    /// US-PP-007 Scenario 1: Update price, verify discountPercent recalculated
    /// ============================================================================
    /// User Story: docs/user-stories/06-billing-user-stories.md
    /// Covers: US-PP-007 Scenario 1
    ///
    /// Given: A promotional package exists with originalPrice=9990, price=4990
    /// When: Updating price to 2990
    /// Then: discountPercent is recalculated to reflect the new price
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_update_price_and_original_updated(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "promo-price-update@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        // Given: Create a promotional package
        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", ctx._realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "promo-price-change",
                            "title": "Promo Price Change",
                            "points": 1000,
                            "price": 4990,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "promotional",
                            "originalPrice": 9990
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::CREATED);
        let create_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let create_json: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
        let package_id = create_json["id"].as_str().unwrap();
        let initial_discount = create_json["discountPercent"].as_i64().unwrap();

        // When: Update price to 2990 (deeper discount)
        let update_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!(
                        "/api/bill/{}/points-packages/{}",
                        ctx._realm_id, package_id
                    ))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "price": 2990
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(update_response.status(), StatusCode::OK);

        // Then: discountPercent is higher after the price reduction
        let update_body = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let update_json: serde_json::Value = serde_json::from_slice(&update_body).unwrap();

        assert_eq!(update_json["price"], 2990);
        assert_eq!(update_json["originalPrice"], 9990);
        let new_discount = update_json["discountPercent"].as_i64().unwrap();
        assert!(
            new_discount > initial_discount,
            "New discount ({}) should be greater than initial ({})",
            new_discount,
            initial_discount
        );
    }

    /// ============================================================================
    /// US-PP-007 Scenario 4: Extend promo end time
    /// ============================================================================
    /// User Story: docs/user-stories/06-billing-user-stories.md
    /// Covers: US-PP-007 Scenario 4
    ///
    /// Given: A promotional package with a specific end time
    /// When: Extending the promo end time to a later date
    /// Then: The updated package reflects the new end time
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_update_extend_time_updated(ctx: &mut TestContext) {
        let app = ctx.create_unified_test_router();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "promo-extend@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        let now = chrono::Utc::now();
        let start = now - chrono::Duration::hours(1);
        let original_end = now + chrono::Duration::days(3);

        // Given: Create a promotional package with a 3-day promo
        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", ctx._realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "promo-extendable",
                            "title": "Promo Extendable",
                            "points": 3000,
                            "price": 6990,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "promotional",
                            "originalPrice": 14990,
                            "promoStartTime": start.to_rfc3339(),
                            "promoEndTime": original_end.to_rfc3339()
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::CREATED);
        let create_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let create_json: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
        let package_id = create_json["id"].as_str().unwrap();

        // When: Extend the promo end time by 7 more days
        let new_end = now + chrono::Duration::days(10);

        let update_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!(
                        "/api/bill/{}/points-packages/{}",
                        ctx._realm_id, package_id
                    ))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "promoEndTime": new_end.to_rfc3339()
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(update_response.status(), StatusCode::OK);

        // Then: The new end time is reflected
        let update_body = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let update_json: serde_json::Value = serde_json::from_slice(&update_body).unwrap();

        let updated_end = update_json["promoEndTime"].as_str().unwrap();
        let updated_end_dt = chrono::DateTime::parse_from_rfc3339(updated_end).unwrap();

        // The new end should be approximately 10 days from now (within 1 minute tolerance)
        let diff = updated_end_dt.timestamp() - now.timestamp();
        let expected_diff = 10 * 24 * 3600; // 10 days in seconds
        assert!(
            (diff - expected_diff).abs() < 60,
            "Updated end time should be ~10 days from now, got diff of {} seconds",
            diff
        );
    }

    /// ============================================================================
    /// US-PP-006 Scenario 5: Past endTime allowed, isExpired=true
    /// ============================================================================
    /// User Story: docs/user-stories/06-billing-user-stories.md
    /// Covers: US-PP-006 Scenario 5
    ///
    /// Given: A realm admin exists
    /// When: Creating a promotional package with endTime in the past
    /// Then: Response is 201 and isExpired is true
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_create_past_end_time_allowed(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "promo-past@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        let now = chrono::Utc::now();
        let start = now - chrono::Duration::days(14);
        let end = now - chrono::Duration::days(1);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", ctx._realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "promo-expired",
                            "title": "Promo Expired",
                            "points": 500,
                            "price": 2990,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "promotional",
                            "originalPrice": 5990,
                            "promoStartTime": start.to_rfc3339(),
                            "promoEndTime": end.to_rfc3339()
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(body_json["packageType"], "promotional");
        assert_eq!(body_json["isExpired"], true);
    }

    /// ============================================================================
    /// US-PP-009 Scenario 2: Admin list shows expired promos
    /// ============================================================================
    /// User Story: docs/user-stories/06-billing-user-stories.md
    /// Covers: US-PP-009 Scenario 2
    ///
    /// Given: A realm admin and an expired promotional package exist
    /// When: Listing packages as admin
    /// Then: The expired promo package is included in the list
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_admin_list_shows_expired(ctx: &mut TestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();
        let (token, user_id) =
            create_admin_session_with_user(ctx, "promo-list@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        // Given: Create a standard package
        let std_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "list-std-pack",
                            "title": "List Standard Pack",
                            "points": 500,
                            "price": 9900,
                            "currency": "CNY",
                            "enabled": true
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(std_response.status(), StatusCode::CREATED);

        // And: Create an expired promo package
        let now = chrono::Utc::now();
        let start = now - chrono::Duration::days(14);
        let end = now - chrono::Duration::days(1);

        let promo_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "list-promo-expired",
                            "title": "List Promo Expired",
                            "points": 1000,
                            "price": 4990,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "promotional",
                            "originalPrice": 9990,
                            "promoStartTime": start.to_rfc3339(),
                            "promoEndTime": end.to_rfc3339()
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(promo_response.status(), StatusCode::CREATED);

        // When: Admin lists packages
        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(list_response.status(), StatusCode::OK);

        let list_body = to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let list_json: serde_json::Value = serde_json::from_slice(&list_body).unwrap();
        let packages = list_json["packages"].as_array().unwrap();

        // Then: Both standard and expired promo packages are visible to admin
        assert!(
            packages.len() >= 2,
            "Admin list should show at least 2 packages (standard + expired promo), got {}",
            packages.len()
        );

        let expired_promo = packages.iter().find(|p| p["name"] == "list-promo-expired");
        assert!(
            expired_promo.is_some(),
            "Admin list should include the expired promo package"
        );
        let promo = expired_promo.unwrap();
        assert_eq!(promo["packageType"], "promotional");
        assert_eq!(promo["isExpired"], true);
    }

    // =========================================================================
    // User-Facing List Filtering & Sorting Scenario Tests (BE-T02)
    // =========================================================================
    //
    // Tests for user-facing points package list endpoint (ext API):
    // - Active promo appears first with discountPercent
    // - Expired promo filtered out
    // - Not-started promo filtered out
    // - Promo without originalPrice still visible
    // - No admin-only fields exposed
    // - Sorting: promo first, then sort_order DESC
    // - Disabled packages filtered
    //
    // User Story: docs/user-stories/billing/points-package.md (US-PP-008, US-PP-009)
    //
    // =========================================================================

    /// ============================================================================
    /// US-PP-008 Scenario 1,5: Active promo appears first, includes discountPercent
    /// ============================================================================
    /// User Story: docs/user-stories/billing/points-package.md
    /// Covers: US-PP-008 Scenario 1,5
    ///
    /// Given: An active promo package and a standard package exist
    /// When: Listing user-visible packages via ext API
    /// Then: The active promo appears first and includes discountPercent
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_user_list_active_promo_first(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();

        // Given: Admin creates packages
        let (token, user_id) =
            create_admin_session_with_user(ctx, "ulist-active@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        // Create a standard package
        let std_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "ulist-std-1",
                            "title": "Standard Pack",
                            "points": 500,
                            "price": 9900,
                            "currency": "CNY",
                            "enabled": true,
                            "sortOrder": 10
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(std_response.status(), StatusCode::CREATED);

        // Create an active promo package
        let now = chrono::Utc::now();
        let start = now - chrono::Duration::hours(1);
        let end = now + chrono::Duration::days(7);

        let promo_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "ulist-promo-active",
                            "title": "Active Promo",
                            "points": 1000,
                            "price": 4990,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "promotional",
                            "originalPrice": 9990,
                            "promoStartTime": start.to_rfc3339(),
                            "promoEndTime": end.to_rfc3339()
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(promo_response.status(), StatusCode::CREATED);

        // Create API key for ext API
        let (api_key, _entity) = create_test_api_key(ctx, "ulist-active-key", true, None).await;

        // When: User lists packages via ext API
        let (status, body_json) =
            crate::tests::helpers::billing_helpers::list_user_visible_points_packages_via_ext_api(
                &app, &realm_id, &api_key,
            )
            .await;

        // Then: Active promo is listed and includes discountPercent
        assert_eq!(status, StatusCode::OK);
        let packages = body_json["packages"]
            .as_array()
            .expect("packages should be an array");

        let promo = packages.iter().find(|p| p["name"] == "ulist-promo-active");
        assert!(promo.is_some(), "Active promo should be visible to users");
        let promo_item = promo.unwrap();
        assert_eq!(promo_item["packageType"], "promotional");
        assert!(
            promo_item["discountPercent"].is_number(),
            "Active promo should have discountPercent"
        );

        // Active promo should be listed before standard packages
        let promo_index = packages
            .iter()
            .position(|p| p["name"] == "ulist-promo-active")
            .unwrap();
        let std_index = packages
            .iter()
            .position(|p| p["name"] == "ulist-std-1")
            .unwrap();
        assert!(
            promo_index < std_index,
            "Active promo should appear before standard packages, got promo at {} and std at {}",
            promo_index,
            std_index
        );
    }

    /// ============================================================================
    /// US-PP-008 Scenario 3, US-PP-009 Scenario 1: Expired promo filtered out
    /// ============================================================================
    /// User Story: docs/user-stories/billing/points-package.md
    /// Covers: US-PP-008 Scenario 3, US-PP-009 Scenario 1
    ///
    /// Given: An expired promo package and a standard package exist
    /// When: Listing user-visible packages via ext API
    /// Then: The expired promo is not included in the response
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_user_list_expired_filtered(ctx: &mut TestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();

        // Given: Admin creates packages
        let (token, user_id) =
            create_admin_session_with_user(ctx, "ulist-expired@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        // Create a standard package
        let std_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "ulist-exp-std",
                            "title": "Standard For Expired Test",
                            "points": 500,
                            "price": 9900,
                            "currency": "CNY",
                            "enabled": true
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(std_response.status(), StatusCode::CREATED);

        // Create an expired promo package
        let now = chrono::Utc::now();
        let start = now - chrono::Duration::days(14);
        let end = now - chrono::Duration::days(1);

        let promo_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "ulist-exp-promo",
                            "title": "Expired Promo",
                            "points": 1000,
                            "price": 4990,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "promotional",
                            "originalPrice": 9990,
                            "promoStartTime": start.to_rfc3339(),
                            "promoEndTime": end.to_rfc3339()
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(promo_response.status(), StatusCode::CREATED);

        // Create API key
        let (api_key, _entity) = create_test_api_key(ctx, "ulist-expired-key", true, None).await;

        // When: User lists packages via ext API
        let (status, body_json) =
            crate::tests::helpers::billing_helpers::list_user_visible_points_packages_via_ext_api(
                &app, &realm_id, &api_key,
            )
            .await;

        // Then: Expired promo is not visible
        assert_eq!(status, StatusCode::OK);
        let packages = body_json["packages"]
            .as_array()
            .expect("packages should be an array");

        let expired_promo = packages.iter().find(|p| p["name"] == "ulist-exp-promo");
        assert!(
            expired_promo.is_none(),
            "Expired promo should NOT be visible to users, but found: {:?}",
            expired_promo
        );

        // Standard package should still be visible
        let std_pkg = packages.iter().find(|p| p["name"] == "ulist-exp-std");
        assert!(
            std_pkg.is_some(),
            "Standard package should still be visible"
        );
    }

    /// ============================================================================
    /// US-PP-008 Scenario 4: Not-started promo filtered out
    /// ============================================================================
    /// User Story: docs/user-stories/billing/points-package.md
    /// Covers: US-PP-008 Scenario 4
    ///
    /// Given: A promo package with start time in the future exists
    /// When: Listing user-visible packages via ext API
    /// Then: The not-yet-started promo is not included
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_user_list_not_started_filtered(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();

        // Given: Admin creates packages
        let (token, user_id) =
            create_admin_session_with_user(ctx, "ulist-future@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        // Create a standard package
        let std_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "ulist-future-std",
                            "title": "Standard For Future Test",
                            "points": 500,
                            "price": 9900,
                            "currency": "CNY",
                            "enabled": true
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(std_response.status(), StatusCode::CREATED);

        // Create a not-yet-started promo package
        let now = chrono::Utc::now();
        let start = now + chrono::Duration::days(7);
        let end = now + chrono::Duration::days(30);

        let promo_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "ulist-future-promo",
                            "title": "Future Promo",
                            "points": 2000,
                            "price": 5990,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "promotional",
                            "originalPrice": 12990,
                            "promoStartTime": start.to_rfc3339(),
                            "promoEndTime": end.to_rfc3339()
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(promo_response.status(), StatusCode::CREATED);

        // Create API key
        let (api_key, _entity) = create_test_api_key(ctx, "ulist-future-key", true, None).await;

        // When: User lists packages via ext API
        let (status, body_json) =
            crate::tests::helpers::billing_helpers::list_user_visible_points_packages_via_ext_api(
                &app, &realm_id, &api_key,
            )
            .await;

        // Then: Not-started promo is not visible
        assert_eq!(status, StatusCode::OK);
        let packages = body_json["packages"]
            .as_array()
            .expect("packages should be an array");

        let future_promo = packages.iter().find(|p| p["name"] == "ulist-future-promo");
        assert!(
            future_promo.is_none(),
            "Not-started promo should NOT be visible to users, but found: {:?}",
            future_promo
        );

        // Standard package should still be visible
        let std_pkg = packages.iter().find(|p| p["name"] == "ulist-future-std");
        assert!(
            std_pkg.is_some(),
            "Standard package should still be visible"
        );
    }

    /// ============================================================================
    /// US-PP-008 Scenario 2: Promo without originalPrice still visible
    /// ============================================================================
    /// User Story: docs/user-stories/billing/points-package.md
    /// Covers: US-PP-008 Scenario 2
    ///
    /// Given: A promo package without originalPrice (only time range) exists
    /// When: Listing user-visible packages via ext API
    /// Then: The promo is still visible (badge-only, no strikethrough price)
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_user_list_no_original_price_shown(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();

        // Given: Admin creates a promo without originalPrice
        let (token, user_id) =
            create_admin_session_with_user(ctx, "ulist-noorigprice@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        let now = chrono::Utc::now();
        let start = now - chrono::Duration::hours(1);
        let end = now + chrono::Duration::days(7);

        let promo_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "ulist-no-orig-price",
                            "title": "Promo No Original Price",
                            "points": 800,
                            "price": 3990,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "promotional",
                            "promoStartTime": start.to_rfc3339(),
                            "promoEndTime": end.to_rfc3339()
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(promo_response.status(), StatusCode::CREATED);

        // Create API key
        let (api_key, _entity) =
            create_test_api_key(ctx, "ulist-noorigprice-key", true, None).await;

        // When: User lists packages via ext API
        let (status, body_json) =
            crate::tests::helpers::billing_helpers::list_user_visible_points_packages_via_ext_api(
                &app, &realm_id, &api_key,
            )
            .await;

        // Then: Promo without originalPrice is still visible
        assert_eq!(status, StatusCode::OK);
        let packages = body_json["packages"]
            .as_array()
            .expect("packages should be an array");

        let promo = packages.iter().find(|p| p["name"] == "ulist-no-orig-price");
        assert!(
            promo.is_some(),
            "Promo without originalPrice should still be visible"
        );
        let promo_item = promo.unwrap();
        assert_eq!(promo_item["packageType"], "promotional");
        assert!(
            promo_item["originalPrice"].is_null(),
            "originalPrice should be null"
        );
        assert!(
            promo_item["discountPercent"].is_null(),
            "discountPercent should be null when no originalPrice"
        );
    }

    /// ============================================================================
    /// Design doc: No admin-only fields in user-facing response
    /// ============================================================================
    /// User Story: docs/user-stories/billing/points-package.md
    /// Covers: design doc (no enabled/isExpired in ext response)
    ///
    /// Given: Packages exist in the realm
    /// When: Listing user-visible packages via ext API
    /// Then: Response does not include admin-only fields (enabled, isExpired)
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_user_list_no_admin_fields_exposed(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();

        // Given: Admin creates packages
        let (token, user_id) =
            create_admin_session_with_user(ctx, "ulist-noadmin@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        // Create a standard package
        let std_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "ulist-noadmin-std",
                            "title": "Standard No Admin",
                            "points": 500,
                            "price": 9900,
                            "currency": "CNY",
                            "enabled": true
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(std_response.status(), StatusCode::CREATED);

        // Create API key
        let (api_key, _entity) = create_test_api_key(ctx, "ulist-noadmin-key", true, None).await;

        // When: User lists packages via ext API
        let (status, body_json) =
            crate::tests::helpers::billing_helpers::list_user_visible_points_packages_via_ext_api(
                &app, &realm_id, &api_key,
            )
            .await;

        // Then: No admin-only fields exposed
        assert_eq!(status, StatusCode::OK);
        let packages = body_json["packages"]
            .as_array()
            .expect("packages should be an array");

        for pkg in packages {
            assert!(
                pkg.get("enabled").is_none(),
                "User-facing response should NOT contain 'enabled' field, but found: {:?}",
                pkg
            );
            assert!(
                pkg.get("isExpired").is_none(),
                "User-facing response should NOT contain 'isExpired' field, but found: {:?}",
                pkg
            );
        }
    }

    /// ============================================================================
    /// US-PP-008 Scenario 5: Sorting: promo first, then sort_order DESC
    /// ============================================================================
    /// User Story: docs/user-stories/billing/points-package.md
    /// Covers: US-PP-008 Scenario 5
    ///
    /// Given: Multiple packages exist with different types and sort_orders
    /// When: Listing user-visible packages via ext API
    /// Then: Active promos appear first, then standard packages by sort_order DESC
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_user_list_sort_order_promo_first(
        ctx: &mut TestContext,
    ) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();

        // Given: Admin creates multiple packages
        let (token, user_id) =
            create_admin_session_with_user(ctx, "ulist-sort@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        let now = chrono::Utc::now();
        let start = now - chrono::Duration::hours(1);
        let end = now + chrono::Duration::days(7);

        // Create a standard package with sort_order 20
        let std_hi_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "ulist-sort-std-hi",
                            "title": "Standard High Sort",
                            "points": 500,
                            "price": 9900,
                            "currency": "CNY",
                            "enabled": true,
                            "sortOrder": 20
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(std_hi_response.status(), StatusCode::CREATED);

        // Create a standard package with sort_order 5
        let std_lo_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "ulist-sort-std-lo",
                            "title": "Standard Low Sort",
                            "points": 300,
                            "price": 5900,
                            "currency": "CNY",
                            "enabled": true,
                            "sortOrder": 5
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(std_lo_response.status(), StatusCode::CREATED);

        // Create an active promo package with sort_order 3
        let promo_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "ulist-sort-promo",
                            "title": "Promo Sort Test",
                            "points": 1000,
                            "price": 4990,
                            "currency": "CNY",
                            "enabled": true,
                            "packageType": "promotional",
                            "originalPrice": 9990,
                            "promoStartTime": start.to_rfc3339(),
                            "promoEndTime": end.to_rfc3339(),
                            "sortOrder": 3
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(promo_response.status(), StatusCode::CREATED);

        // Create API key
        let (api_key, _entity) = create_test_api_key(ctx, "ulist-sort-key", true, None).await;

        // When: User lists packages via ext API
        let (status, body_json) =
            crate::tests::helpers::billing_helpers::list_user_visible_points_packages_via_ext_api(
                &app, &realm_id, &api_key,
            )
            .await;

        // Then: Promo first, then standard by sort_order DESC
        assert_eq!(status, StatusCode::OK);
        let packages = body_json["packages"]
            .as_array()
            .expect("packages should be an array");

        let promo_index = packages
            .iter()
            .position(|p| p["name"] == "ulist-sort-promo")
            .expect("promo should be present");
        let std_hi_index = packages
            .iter()
            .position(|p| p["name"] == "ulist-sort-std-hi")
            .expect("std-hi should be present");
        let std_lo_index = packages
            .iter()
            .position(|p| p["name"] == "ulist-sort-std-lo")
            .expect("std-lo should be present");

        // Promo should appear before all standard packages
        assert!(
            promo_index < std_hi_index,
            "Promo (index {}) should appear before std-hi (index {})",
            promo_index,
            std_hi_index
        );
        assert!(
            promo_index < std_lo_index,
            "Promo (index {}) should appear before std-lo (index {})",
            promo_index,
            std_lo_index
        );

        // Among standard packages, higher sort_order should come first
        assert!(
            std_hi_index < std_lo_index,
            "Standard with sort_order=20 (index {}) should appear before sort_order=5 (index {})",
            std_hi_index,
            std_lo_index
        );
    }

    /// ============================================================================
    /// Design doc: Disabled packages filtered from user-facing list
    /// ============================================================================
    /// User Story: docs/user-stories/billing/points-package.md
    /// Covers: design doc (disabled packages hidden from users)
    ///
    /// Given: An enabled and a disabled standard package exist
    /// When: Listing user-visible packages via ext API
    /// Then: Only the enabled package appears
    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_scenario_points_package_promo_user_list_disabled_filtered(ctx: &mut TestContext) {
        let app = ctx.create_unified_test_router();
        let realm_id = ctx._realm_id.clone();

        // Given: Admin creates packages
        let (token, user_id) =
            create_admin_session_with_user(ctx, "ulist-disabled@test.com", 1800).await;
        grant_realm_admin_role(ctx, &user_id).await;

        // Create an enabled standard package
        let enabled_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "ulist-disabled-enabled",
                            "title": "Enabled Pack",
                            "points": 500,
                            "price": 9900,
                            "currency": "CNY",
                            "enabled": true
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(enabled_response.status(), StatusCode::CREATED);

        // Create a disabled standard package
        let disabled_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/bill/{}/points-packages", realm_id))
                    .header("Content-Type", "application/json")
                    .header("cookie", format!("X-Auth={token}"))
                    .body(Body::from(
                        json!({
                            "name": "ulist-disabled-off",
                            "title": "Disabled Pack",
                            "points": 300,
                            "price": 5900,
                            "currency": "CNY",
                            "enabled": false
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(disabled_response.status(), StatusCode::CREATED);

        // Create API key
        let (api_key, _entity) = create_test_api_key(ctx, "ulist-disabled-key", true, None).await;

        // When: User lists packages via ext API
        let (status, body_json) =
            crate::tests::helpers::billing_helpers::list_user_visible_points_packages_via_ext_api(
                &app, &realm_id, &api_key,
            )
            .await;

        // Then: Only enabled package appears
        assert_eq!(status, StatusCode::OK);
        let packages = body_json["packages"]
            .as_array()
            .expect("packages should be an array");

        let enabled_pkg = packages
            .iter()
            .find(|p| p["name"] == "ulist-disabled-enabled");
        assert!(enabled_pkg.is_some(), "Enabled package should be visible");

        let disabled_pkg = packages.iter().find(|p| p["name"] == "ulist-disabled-off");
        assert!(
            disabled_pkg.is_none(),
            "Disabled package should NOT be visible to users, but found: {:?}",
            disabled_pkg
        );
    }
}
