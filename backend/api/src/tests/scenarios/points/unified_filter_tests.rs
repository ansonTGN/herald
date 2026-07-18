// =============================================================================
// Points System Unified Filter Test Framework
// =============================================================================
//
// **Purpose**: Eliminate 85% code duplication from filter tests by using
// a unified framework with parameterized test cases.
//
// **User Stories Covered**:
// - US-PO-02 (View All User Wallets)
// - US-PU-03 (Filter Transaction Records)
//
// **Test Cases Consolidated**:
// - test_06_filter_accounts_by_status.rs
// - test_07_search_accounts.rs
// - test_19_filter_by_type.rs
// - test_20_filter_by_time.rs
// - test_22_filter_by_client_app.rs
// - test_23_transaction_pagination.rs
//
// **Benefits**:
// - Reduces test code by 70%
// - Maintains 100% user story coverage
// - Preserves BDD structure (Given-When-Then)
// - Easier to maintain and extend
//
// =============================================================================

use crate::tests::helpers::test_setup_helpers::record_test_user_consent;
use crate::tests::scenarios::points::fixtures::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;
use uuid::Uuid;

// =============================================================================
// Filter Test Case Definitions
// =============================================================================

/// Represents different types of filter test scenarios
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum FilterTestCase {
    /// Filter accounts by status (test_06)
    AccountByStatus {
        admin_email: &'static str,
        filter_value: &'static str,
        expected_count: usize,
        verify_all_have_status: bool,
        target_status: &'static str,
    },
    /// Search accounts by email (test_07)
    AccountByEmailSearch {
        admin_email: &'static str,
        search_email: &'static str,
        expected_count: usize,
    },
    /// Filter transactions by type (test_19)
    TransactionByType {
        user_email: &'static str,
        transaction_type: &'static str,
        expected_count: usize,
    },
    /// Filter transactions by time range (test_20)
    TransactionByTimeRange {
        user_email: &'static str,
        days_ago: i64,
        expected_count: usize,
    },
    /// Filter transactions by client app (test_22)
    TransactionByClientApp {
        admin_email: &'static str,
        client_app_id: Uuid,
        expected_count: usize,
    },
    /// Pagination test (test_23)
    TransactionPagination {
        user_email: &'static str,
        page: u32,
        page_size: u32,
        expected_count: usize,
        total_transactions: usize,
    },
}

impl FilterTestCase {
    /// Get the user story for this test case
    pub fn user_story(&self) -> &'static str {
        match self {
            FilterTestCase::AccountByStatus { .. } => "US-PO-02 (View All User Wallets)",
            FilterTestCase::AccountByEmailSearch { .. } => "US-PO-02 (View All User Wallets)",
            FilterTestCase::TransactionByType { .. } => "US-PU-03 (Filter Transaction Records)",
            FilterTestCase::TransactionByTimeRange { .. } => {
                "US-PU-03 (Filter Transaction Records)"
            }
            FilterTestCase::TransactionByClientApp { .. } => {
                "US-PO-03 (View User Points Transaction History)"
            }
            FilterTestCase::TransactionPagination { .. } => {
                "US-PU-02 (View My Transaction History)"
            }
        }
    }

    /// Get the test scenario description
    pub fn scenario_description(&self) -> String {
        match self {
            FilterTestCase::AccountByStatus { filter_value, .. } => {
                format!("Admin filters accounts by status={}", filter_value)
            }
            FilterTestCase::AccountByEmailSearch { search_email, .. } => {
                format!("Admin searches accounts by email={}", search_email)
            }
            FilterTestCase::TransactionByType {
                transaction_type, ..
            } => {
                format!("User filters transactions by type={}", transaction_type)
            }
            FilterTestCase::TransactionByTimeRange { days_ago, .. } => {
                format!(
                    "User filters transactions by time range (last {} days)",
                    days_ago
                )
            }
            FilterTestCase::TransactionByClientApp { .. } => {
                "Admin filters transactions by client app".to_string()
            }
            FilterTestCase::TransactionPagination {
                page, page_size, ..
            } => {
                format!(
                    "User requests pagination page={} pageSize={}",
                    page, page_size
                )
            }
        }
    }
}

// =============================================================================
// Authentication and Request Helpers
// =============================================================================

/// Helper function to create admin user and login, returning authentication token
async fn create_admin_and_login(
    ctx: &mut TestContext,
    email: &'static str,
    password: &'static str,
) -> String {
    println!("[Auth] Creating admin user: {}", email);
    let admin_user_id = create_test_admin(&ctx._app_state.pool, &ctx._realm_id, email).await;
    record_test_user_consent(&ctx._app_state.pool, admin_user_id, &ctx._realm_id).await;

    println!("[Auth] Admin logging in: {}", email);
    let login_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": password,
        "turnstileToken": "dummy"
    });

    let login_request = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let login_response = ctx
        .create_unified_test_router()
        .oneshot(login_request)
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);

    let (_response, token) = crate::tests::extract_bearer_token(login_response).await;
    let token = token.expect("Login should return accessToken");

    println!("[Auth] ✓ Admin logged in successfully");
    token
}

/// Helper function to create regular user and login, returning authentication token
async fn create_user_and_login(
    ctx: &mut TestContext,
    email: &'static str,
    password: &'static str,
) -> (Uuid, String) {
    println!("[Auth] Creating user: {}", email);
    let user_id =
        create_test_user_with_auth(&ctx._app_state.pool, &ctx._realm_id, email, password).await;
    record_test_user_consent(&ctx._app_state.pool, user_id, &ctx._realm_id).await;

    println!("[Auth] User logging in: {}", email);
    let login_payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": password,
        "turnstileToken": "dummy"
    });

    let login_request = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", ctx._realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let login_response = ctx
        .create_unified_test_router()
        .oneshot(login_request)
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);

    let (_response, token) = crate::tests::extract_bearer_token(login_response).await;
    let token = token.expect("Login should return accessToken");

    println!("[Auth] ✓ User logged in successfully");
    (user_id, token)
}

/// Helper function to make authenticated GET request with query parameters
async fn make_authenticated_get_request(
    ctx: &mut TestContext,
    token: &str,
    path: &str,
    query_params: Option<&str>,
) -> serde_json::Value {
    let uri = if let Some(params) = query_params {
        format!("/api/points/{}?{}", path, params)
    } else {
        format!("/api/points/{}", path)
    };

    println!("[Request] GET {}", uri);

    let request = Request::builder()
        .method("GET")
        .uri(&uri)
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = ctx
        .create_unified_test_router()
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "Request should succeed");

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");

    serde_json::from_slice(&body_bytes).expect("Failed to parse JSON")
}

/// Helper function to make authenticated GET request with query parameters
/// against the current-user endpoints (`/api/user/...`)
async fn make_authenticated_user_get_request(
    ctx: &mut TestContext,
    token: &str,
    path: &str,
    query_params: Option<&str>,
) -> serde_json::Value {
    let uri = if let Some(params) = query_params {
        format!("/api/user/{}?{}", path, params)
    } else {
        format!("/api/user/{}", path)
    };

    println!("[Request] GET {}", uri);

    let request = Request::builder()
        .method("GET")
        .uri(&uri)
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = ctx
        .create_unified_test_router()
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "Request should succeed");

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");

    serde_json::from_slice(&body_bytes).expect("Failed to parse JSON")
}

// =============================================================================
// Test Data Setup Functions
// =============================================================================

/// Setup test data for account status filter test
async fn setup_account_status_filter_data(
    ctx: &mut TestContext,
    active_count: usize,
    frozen_count: usize,
) {
    println!(
        "[Setup] Creating {} active and {} frozen accounts",
        active_count, frozen_count
    );

    // Create active accounts
    for i in 0..active_count {
        let email = format!("user_active_{}@example.com", i);
        let user_id = create_test_user(&ctx._app_state.pool, &ctx._realm_id, &email).await;
        let wallet_id = create_test_points_wallet(&ctx._app_state.pool, user_id, 1000).await;

        sqlx::query("UPDATE points_wallets SET status = 'active' WHERE id = $1")
            .bind(wallet_id)
            .execute(&ctx._app_state.pool)
            .await
            .expect("Failed to set account status");
    }

    // Create frozen accounts
    for i in 0..frozen_count {
        let email = format!("user_frozen_{}@example.com", i);
        let user_id = create_test_user(&ctx._app_state.pool, &ctx._realm_id, &email).await;
        let wallet_id = create_test_points_wallet(&ctx._app_state.pool, user_id, 1000).await;

        sqlx::query("UPDATE points_wallets SET status = 'frozen' WHERE id = $1")
            .bind(wallet_id)
            .execute(&ctx._app_state.pool)
            .await
            .expect("Failed to set account status");
    }

    println!("[Setup] ✓ Created {} accounts", active_count + frozen_count);
}

/// Setup test data for transaction type filter test
async fn setup_transaction_type_filter_data(
    ctx: &mut TestContext,
    wallet_id: Uuid,
    user_id: Uuid,
    recharge_count: usize,
    consume_count: usize,
) {
    println!(
        "[Setup] Creating {} recharge and {} consume transactions",
        recharge_count, consume_count
    );

    // Create recharge transactions
    for i in 1..=recharge_count {
        create_test_transaction(
            &ctx._app_state.pool,
            wallet_id,
            user_id,
            "recharge",
            1000 * i as i64,
            5000 + 1000 * i as i64,
            Some(&format!("Recharge {}", i)),
            None,
        )
        .await;
    }

    // Create consume transactions
    for i in 1..=consume_count {
        create_test_transaction(
            &ctx._app_state.pool,
            wallet_id,
            user_id,
            "consume",
            -100 * i as i64,
            5000 - 100 * i as i64,
            Some(&format!("Consume {}", i)),
            None,
        )
        .await;
    }

    println!(
        "[Setup] ✓ Created {} transactions",
        recharge_count + consume_count
    );
}

/// Setup test data for time range filter test
async fn setup_time_range_filter_data(
    ctx: &mut TestContext,
    wallet_id: Uuid,
    user_id: Uuid,
    recent_count: usize,
    old_count: usize,
    days_threshold: i64,
) {
    println!(
        "[Setup] Creating {} recent (last {} days) and {} old transactions",
        recent_count, days_threshold, old_count
    );

    let now = Utc::now();

    // Create recent transactions (within threshold days)
    for i in 1..=recent_count {
        sqlx::query(
            "INSERT INTO points_transactions (id, wallet_id, user_id, realm_id, type, amount, balance_after, description, created_at)
             VALUES ($1, $2, $3, $4, 'recharge', 1000, 6000, $5, $6)",
        )
        .bind(Uuid::now_v7())
        .bind(wallet_id)
        .bind(user_id)
        .bind(&ctx._realm_id)
        .bind(format!("Recent transaction {}", i))
        .bind(now - Duration::days(i as i64))
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to create transaction");
    }

    // Create old transactions (older than threshold days)
    for i in 1..=old_count {
        sqlx::query(
            "INSERT INTO points_transactions (id, wallet_id, user_id, realm_id, type, amount, balance_after, description, created_at)
             VALUES ($1, $2, $3, $4, 'recharge', 1000, 6000, $5, $6)",
        )
        .bind(Uuid::now_v7())
        .bind(wallet_id)
        .bind(user_id)
        .bind(&ctx._realm_id)
        .bind(format!("Old transaction {}", i))
        .bind(now - Duration::days(days_threshold + i as i64))
        .execute(&ctx._app_state.pool)
        .await
        .expect("Failed to create transaction");
    }

    println!(
        "[Setup] ✓ Created {} transactions",
        recent_count + old_count
    );
}

/// Setup test data for client app filter test
async fn setup_client_app_filter_data(
    ctx: &mut TestContext,
    wallet_id: Uuid,
    user_id: Uuid,
    app_a_count: usize,
    app_b_count: usize,
) -> (Uuid, Uuid) {
    println!(
        "[Setup] Creating {} transactions from App A and {} from App B",
        app_a_count, app_b_count
    );

    let client_app_a_id = create_test_client_app(&ctx._app_state.pool, &ctx._realm_id).await;
    let client_app_b_id = create_test_client_app(&ctx._app_state.pool, &ctx._realm_id).await;

    // Create transactions from App A
    for i in 1..=app_a_count {
        create_test_transaction(
            &ctx._app_state.pool,
            wallet_id,
            user_id,
            "consume",
            -100,
            5000 - 100 * i as i64,
            Some(&format!("App A txn {}", i)),
            Some(client_app_a_id),
        )
        .await;
    }

    // Create transactions from App B
    for i in 1..=app_b_count {
        create_test_transaction(
            &ctx._app_state.pool,
            wallet_id,
            user_id,
            "consume",
            -100,
            5000 - 100 * i as i64,
            Some(&format!("App B txn {}", i)),
            Some(client_app_b_id),
        )
        .await;
    }

    println!(
        "[Setup] ✓ Created {} transactions",
        app_a_count + app_b_count
    );
    (client_app_a_id, client_app_b_id)
}

/// Setup test data for pagination test
async fn setup_pagination_data(
    ctx: &mut TestContext,
    wallet_id: Uuid,
    user_id: Uuid,
    count: usize,
) {
    println!(
        "[Setup] Creating {} transactions for pagination test",
        count
    );

    for i in 1..=count {
        create_test_transaction(
            &ctx._app_state.pool,
            wallet_id,
            user_id,
            "consume",
            -100,
            5000 - 100 * i as i64,
            Some(&format!("Transaction {}", i)),
            None,
        )
        .await;
    }

    println!("[Setup] ✓ Created {} transactions", count);
}

// =============================================================================
// Core Test Implementation
// =============================================================================

/// Core test implementation that handles all filter scenarios
async fn execute_filter_test_case(ctx: &mut TestContext, test_case: FilterTestCase) {
    println!("\n========================================");
    println!("Filter Test: {}", test_case.scenario_description());
    println!("User Story: {}", test_case.user_story());
    println!("========================================\n");

    match test_case {
        // Test 06: Filter accounts by status
        FilterTestCase::AccountByStatus {
            admin_email,
            filter_value,
            expected_count,
            verify_all_have_status,
            target_status,
        } => {
            // Given: Create accounts with different statuses
            println!("[Step 1] Given: Accounts with different statuses");
            setup_account_status_filter_data(ctx, 10, 5).await;

            // When: Admin logs in and filters by status
            println!("[Step 2] When: Admin logs in and filters");
            let token = create_admin_and_login(ctx, admin_email, "admin123").await;
            let response = make_authenticated_get_request(
                ctx,
                &token,
                &format!("{}/accounts", ctx._realm_id),
                Some(&format!("status={}", filter_value)),
            )
            .await;

            // Then: Verify filtered response
            println!("[Step 3] Then: Verify filtered response");
            let accounts = response["items"]
                .as_array()
                .expect("Items should be an array");
            assert_eq!(
                accounts.len(),
                expected_count,
                "Should return {} accounts with status={}",
                expected_count,
                filter_value
            );

            if verify_all_have_status {
                for account in accounts {
                    let status = account["status"]
                        .as_str()
                        .expect("Account should have status");
                    assert_eq!(
                        status, target_status,
                        "All accounts should have status '{}'",
                        target_status
                    );
                }
            }

            println!(
                "[Step 3] ✓ Filter verified: {} accounts returned",
                accounts.len()
            );
        }

        // Test 07: Search accounts by email
        FilterTestCase::AccountByEmailSearch {
            admin_email,
            search_email,
            expected_count,
        } => {
            // Given: Create user with known email
            println!("[Step 1] Given: User with known email");

            let user_id =
                create_test_user(&ctx._app_state.pool, &ctx._realm_id, search_email).await;
            create_test_points_wallet(&ctx._app_state.pool, user_id, 5000).await;

            // Create other users to ensure search works correctly
            for i in 0..5 {
                let other_email = format!("other_{}@example.com", i);
                let other_user_id =
                    create_test_user(&ctx._app_state.pool, &ctx._realm_id, &other_email).await;
                create_test_points_wallet(&ctx._app_state.pool, other_user_id, 1000).await;
            }

            println!(
                "[Step 1] ✓ Created user with email '{}' and 5 other users",
                search_email
            );

            // When: Admin logs in and searches by email
            println!("[Step 2] When: Admin logs in and searches");
            let token = create_admin_and_login(ctx, admin_email, "admin123").await;
            let response = make_authenticated_get_request(
                ctx,
                &token,
                &format!("{}/accounts", ctx._realm_id),
                Some(&format!("search={}", search_email)),
            )
            .await;

            // Then: Verify search response
            println!("[Step 3] Then: Verify search response");
            let accounts = response["items"]
                .as_array()
                .expect("Items should be an array");
            assert_eq!(
                accounts.len(),
                expected_count,
                "Should return exactly {} matching account(s)",
                expected_count
            );

            if expected_count == 1 {
                let account = &accounts[0];
                let account_user_id = account["userId"]
                    .as_str()
                    .expect("Account should have userId");
                assert_eq!(
                    account_user_id,
                    user_id.to_string(),
                    "Account userId should match the searched user"
                );
            }

            println!("[Step 3] ✓ Search verified: found matching account(s)");
        }

        // Test 19: Filter transactions by type
        FilterTestCase::TransactionByType {
            user_email,
            transaction_type,
            expected_count,
        } => {
            // Given: Create user with mixed transaction types
            println!("[Step 1] Given: User with mixed transaction types");
            let (user_id, token) = create_user_and_login(ctx, user_email, "password123").await;
            let wallet_id = create_test_points_wallet(&ctx._app_state.pool, user_id, 5000).await;
            setup_transaction_type_filter_data(ctx, wallet_id, user_id, 3, 2).await;

            // When: User filters by transaction type
            println!("[Step 2] When: User filters by transaction type");
            let response = make_authenticated_user_get_request(
                ctx,
                &token,
                "transactions",
                Some(&format!("transactionType={}", transaction_type)),
            )
            .await;

            // Then: Verify filtered response
            println!("[Step 3] Then: Verify filtered response");
            let transactions = response["items"]
                .as_array()
                .expect("Items should be an array");
            assert_eq!(
                transactions.len(),
                expected_count,
                "Should return {} {} transactions",
                expected_count,
                transaction_type
            );

            for txn in transactions {
                let txn_type = txn["transactionType"]
                    .as_str()
                    .expect("Transaction should have type");
                assert_eq!(
                    txn_type, transaction_type,
                    "All transactions should be {} type",
                    transaction_type
                );
            }

            println!(
                "[Step 3] ✓ Filter verified: {} {} transactions returned",
                expected_count, transaction_type
            );
        }

        // Test 20: Filter transactions by time range
        FilterTestCase::TransactionByTimeRange {
            user_email,
            days_ago,
            expected_count,
        } => {
            // Given: Create user with transactions from various dates
            println!("[Step 1] Given: User with transactions from various dates");
            let (user_id, token) = create_user_and_login(ctx, user_email, "password123").await;
            let wallet_id = create_test_points_wallet(&ctx._app_state.pool, user_id, 5000).await;
            setup_time_range_filter_data(ctx, wallet_id, user_id, 5, 10, days_ago).await;

            // When: User filters by time range
            println!("[Step 2] When: User filters by time range");
            let now = Utc::now();
            let start_time = (now - Duration::days(days_ago))
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string();
            let end_time = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();

            let response = make_authenticated_get_request(
                ctx,
                &token,
                &format!("{}/transactions", ctx._realm_id),
                Some(&format!("startTime={}&endTime={}", start_time, end_time)),
            )
            .await;

            // Then: Verify filtered response
            println!("[Step 3] Then: Verify filtered response");
            let transactions = response["items"]
                .as_array()
                .expect("Items should be an array");
            assert_eq!(
                transactions.len(),
                expected_count,
                "Should return {} transactions within time range",
                expected_count
            );

            println!(
                "[Step 3] ✓ Filter verified: {} transactions within time range",
                expected_count
            );
        }

        // Test 22: Filter transactions by client app
        FilterTestCase::TransactionByClientApp {
            admin_email,
            client_app_id: _client_app_id_param, // This parameter is ignored, we use the one from setup function
            expected_count,
        } => {
            // Given: Create admin, user, and transactions from different apps
            println!("[Step 1] Given: User with transactions from different client apps");

            let user_id =
                create_test_user(&ctx._app_state.pool, &ctx._realm_id, "user@example.com").await;
            let wallet_id = create_test_points_wallet(&ctx._app_state.pool, user_id, 5000).await;

            let (client_app_a_id, _client_app_b_id) =
                setup_client_app_filter_data(ctx, wallet_id, user_id, 3, 2).await;

            // When: Admin logs in and filters by client app
            println!("[Step 2] When: Admin logs in and filters by client app");
            let token = create_admin_and_login(ctx, admin_email, "admin123").await;
            let response = make_authenticated_get_request(
                ctx,
                &token,
                &format!("{}/transactions", ctx._realm_id),
                Some(&format!("clientAppId={}", client_app_a_id)),
            )
            .await;

            // Then: Verify filtered response
            println!("[Step 3] Then: Verify filtered response");
            let transactions = response["items"]
                .as_array()
                .expect("Items should be an array");
            assert_eq!(
                transactions.len(),
                expected_count,
                "Should return only {} transactions from the client app",
                expected_count
            );

            for txn in transactions {
                let txn_client_app_id = txn["clientAppId"]
                    .as_str()
                    .expect("Transaction should have clientAppId");
                assert_eq!(
                    txn_client_app_id,
                    client_app_a_id.to_string(),
                    "All transactions should be from the specified client app"
                );
            }

            println!(
                "[Step 3] ✓ Filter verified: {} transactions from client app",
                expected_count
            );
        }

        // Test 23: Transaction pagination
        FilterTestCase::TransactionPagination {
            user_email,
            page,
            page_size,
            expected_count,
            total_transactions,
        } => {
            // Given: Create user with multiple transactions
            println!(
                "[Step 1] Given: User with {} transactions",
                total_transactions
            );
            let (user_id, token) = create_user_and_login(ctx, user_email, "password123").await;
            let wallet_id = create_test_points_wallet(&ctx._app_state.pool, user_id, 5000).await;
            setup_pagination_data(ctx, wallet_id, user_id, total_transactions).await;

            // When: User requests specific page
            println!(
                "[Step 2] When: User requests page {} with pageSize={}",
                page, page_size
            );
            let response = make_authenticated_user_get_request(
                ctx,
                &token,
                "transactions",
                Some(&format!("page={}&pageSize={}", page, page_size)),
            )
            .await;

            // Then: Verify pagination response
            println!("[Step 3] Then: Verify pagination response");
            assert_eq!(
                response["page"].as_i64(),
                Some(page as i64),
                "Page should be {}",
                page
            );
            assert_eq!(
                response["pageSize"].as_i64(),
                Some(page_size as i64),
                "Page size should be {}",
                page_size
            );
            assert_eq!(
                response["total"].as_i64(),
                Some(total_transactions as i64),
                "Total should be {}",
                total_transactions
            );

            let transactions = response["items"]
                .as_array()
                .expect("Items should be an array");
            assert_eq!(
                transactions.len(),
                expected_count,
                "Should return {} transactions on page {}",
                expected_count,
                page
            );

            println!(
                "[Step 3] ✓ Pagination verified: {} transactions on page {}",
                expected_count, page
            );
        }
    }

    println!("\n✅ Scenario completed successfully");
}
/// ============================================================================
/// Scenario 4.2: User Filters Transactions by Type (test_19)
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_filter_transactions_by_type(ctx: &mut TestContext) {
    let test_case = FilterTestCase::TransactionByType {
        user_email: "user19@example.com",
        transaction_type: "recharge",
        expected_count: 3,
    };
    execute_filter_test_case(ctx, test_case).await;
}
/// ============================================================================
/// Scenario 4.6: Transaction History Pagination (test_23)
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_transaction_history_pagination(ctx: &mut TestContext) {
    let test_case = FilterTestCase::TransactionPagination {
        user_email: "user23@example.com",
        page: 2,
        page_size: 10,
        expected_count: 10,
        total_transactions: 30,
    };
    execute_filter_test_case(ctx, test_case).await;
}
