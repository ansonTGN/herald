// End-to-end tests for DELETE /api/user (BE-D07).
// Covers design §6.1「账户注销」and US-RU-014.

use crate::tests::helpers::points_helpers::ensure_test_bucket_for_realm;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use herald_core::domain::legal::entities::AgreementType;
use herald_core::domain::{
    authentication::BrowserTokenService, client::ports::ClientService, user::UserRepository,
};
use herald_core::infrastructure::authentication::RedisBrowserTokenService;
use serde_json::json;
use sqlx::Row;
use test_context::test_context;
use tower::ServiceExt;
use uuid::Uuid;

/// Create a normal user with a bcrypt-hashed password, a username, provider_ids
/// and a profile nickname. Returns (user_id, email).
async fn seed_normal_user_with_password(
    ctx: &TestContext,
    realm_id: &str,
    password: &str,
) -> (Uuid, String) {
    let user_id = Uuid::now_v7();
    let email = format!("self-delete-{}@test.com", user_id.simple());
    let username = format!("user_{}", user_id.simple());
    let password_hash =
        bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash password");
    let provider_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, username, provider_ids, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, 1, NOW(), NOW())",
    )
    .bind(user_id)
    .bind(realm_id)
    .bind(&email)
    .bind(&password_hash)
    .bind(&username)
    .bind(vec![provider_id])
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test user");

    sqlx::query(
        "INSERT INTO profile (id, realm_id, nickname, created_at, updated_at)
         VALUES ($1, $2, $3, NOW(), NOW())",
    )
    .bind(user_id)
    .bind(realm_id)
    .bind(format!("nick_{}", user_id.simple()))
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create test profile");

    // Seed consent records so the user can log in past the BE-D08 consent gate.
    // The helper resolves the current effective versions (realm custom if any,
    // otherwise the platform default templates) and writes one row per type.
    let tos_id = ctx
        .app_state
        .legal_service
        .current_effective(realm_id, AgreementType::TermsOfService)
        .await
        .expect("Failed to resolve effective ToS")
        .map(|v| v.id)
        .expect("No effective ToS version exists");
    let pp_id = ctx
        .app_state
        .legal_service
        .current_effective(realm_id, AgreementType::PrivacyPolicy)
        .await
        .expect("Failed to resolve effective PrivacyPolicy")
        .map(|v| v.id)
        .expect("No effective PrivacyPolicy version exists");

    sqlx::query(
        "INSERT INTO user_agreement_consent (user_id, realm_id, agreement_type, consented_version_id)
         VALUES ($1, $2, 'terms_of_service', $3),
                ($1, $2, 'privacy_policy', $4)
         ON CONFLICT (user_id, agreement_type) DO NOTHING",
    )
    .bind(user_id)
    .bind(realm_id)
    .bind(tos_id)
    .bind(pp_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to seed user consent");

    (user_id, email)
}

/// Insert a TOTP config (plus one backup code) for the user.
async fn seed_totp_config(ctx: &TestContext, user_id: Uuid) {
    let config_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO user_totp_config (id, user_id, realm_id, secret_hash, enabled, verified_at, created_at, updated_at)
         VALUES ($1, $2, $3, $4, true, NOW(), NOW(), NOW())",
    )
    .bind(config_id)
    .bind(user_id)
    .bind(&ctx._realm_id)
    .bind("JBSWY3DPEHPK3PXP")
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to seed TOTP config");

    sqlx::query(
        "INSERT INTO user_totp_backup_codes (user_totp_config_id, code_hash, used, created_at)
         VALUES ($1, $2, false, NOW())",
    )
    .bind(config_id)
    .bind("dummy_hash")
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to seed TOTP backup code");
}

/// Insert an in-effect subscription row bound to the test realm's bucket.
async fn seed_active_subscription(
    ctx: &TestContext,
    realm_id: &str,
    user_id: Uuid,
    status: &str,
) -> Uuid {
    let sub_id = Uuid::now_v7();
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;
    // Each subscription needs a distinct client_app_id because the column is UNIQUE.
    let client_app_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO subscription
            (id, realm_id, user_id, client_app_id, external_subscription_id, external_product_id,
             payment_provider, status, entitlement_key, external_price_id,
             current_period_start, current_period_end, cancel_at_period_end, created_at, updated_at, bucket_id)
         VALUES ($1, $2, $3, $4, $5, $6, 'creem', $7, $8, $9,
                 NOW(), NOW() + INTERVAL '30 days', false, NOW(), NOW(), $10)",
    )
    .bind(sub_id)
    .bind(realm_id)
    .bind(user_id)
    .bind(client_app_id)
    .bind(format!("sub_{}", sub_id.simple()))
    .bind(format!("prod_{}", sub_id.simple()))
    .bind(status)
    .bind(format!("plan_{}", sub_id.simple()))
    .bind(format!("price_{}", sub_id.simple()))
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to seed active subscription");

    sub_id
}

/// Insert a succeeded one-time points-package purchase row.
async fn seed_one_time_purchase(ctx: &TestContext, realm_id: &str, user_id: Uuid) -> Uuid {
    let attempt_id = Uuid::now_v7();
    let bucket_id = ensure_test_bucket_for_realm(&ctx.app_state.pool, realm_id).await;

    sqlx::query(
        "INSERT INTO payment_attempts
            (id, realm_id, user_id, payment_provider, target_type, target_id, bucket_id,
             amount, currency, status, expires_at, created_at, updated_at)
         VALUES ($1, $2, $3, 'stripe', 'entitlement_mapping', $4, $5,
                 999, 'USD', 'Succeeded', NOW() + INTERVAL '2 hours', NOW(), NOW())",
    )
    .bind(attempt_id)
    .bind(realm_id)
    .bind(user_id)
    .bind(Uuid::now_v7())
    .bind(bucket_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to seed one-time purchase");

    attempt_id
}

/// Perform a password login and return the raw response.
async fn login_with_credentials(
    ctx: &TestContext,
    realm_id: &str,
    email: &str,
    password: &str,
) -> axum::response::Response {
    let app = ctx.create_unified_test_router();
    let payload = json!({
        "clientId": ctx._client_id,
        "email": email,
        "password": password,
        "turnstileToken": "dummy"
    });

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/auth/{}/login", realm_id))
        .header("content-type", "application/json")
        .header("x-forwarded-for", "3.3.3.3")
        .body(Body::from(payload.to_string()))
        .unwrap();

    app.oneshot(req).await.unwrap()
}

/// Login and extract the Bearer access token.
async fn login_and_get_token(
    ctx: &TestContext,
    realm_id: &str,
    email: &str,
    password: &str,
) -> String {
    let resp = login_with_credentials(ctx, realm_id, email, password).await;
    assert!(
        !resp.headers().contains_key(header::SET_COOKIE),
        "Browser-token login must not set a cookie"
    );
    let body: serde_json::Value = crate::tests::response_json(resp).await;
    body["accessToken"]
        .as_str()
        .expect("Login should return accessToken")
        .to_owned()
}

/// Call DELETE /api/user with the given Bearer token and reauthentication token.
async fn call_delete_account(
    ctx: &TestContext,
    token: &str,
    reauth_token: &str,
) -> axum::response::Response {
    let app = ctx.create_unified_test_router();
    let payload = json!({ "reauthToken": reauth_token });

    let req = Request::builder()
        .method("DELETE")
        .uri("/api/user")
        .header("content-type", "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(payload.to_string()))
        .unwrap();

    app.oneshot(req).await.unwrap()
}

/// Create an additional FirstParty browser-token family for the user.
async fn create_extra_session(ctx: &TestContext, user_id: Uuid) -> String {
    let user = ctx
        .app_state
        .user_repository
        .get_user_by_id(user_id)
        .await
        .expect("Failed to load test user");
    let client_app = ctx
        .app_state
        .service
        .client_service()
        .get_client_app_by_client_id(&ctx._realm_id, &ctx._client_id)
        .await
        .expect("Failed to load test client app");
    RedisBrowserTokenService::new(ctx.app_state.redis_manager.clone())
        .create_first_party_token_family(&user, &client_app, None, None)
        .await
        .expect("Failed to create extra token family")
        .access_token
}

/// Read the account row fields that matter for anonymization assertions.
async fn read_account_row(
    ctx: &TestContext,
    user_id: Uuid,
) -> (i16, String, Option<String>, Option<String>, Vec<Uuid>) {
    let row = sqlx::query(
        "SELECT status, email, password, username, provider_ids FROM account WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .expect("Failed to read account row");

    (
        row.get("status"),
        row.get("email"),
        row.get("password"),
        row.get("username"),
        row.get("provider_ids"),
    )
}

/// Read the profile nickname (None if profile row is absent).
async fn read_profile_nickname(ctx: &TestContext, user_id: Uuid) -> Option<String> {
    sqlx::query_scalar::<_, Option<String>>("SELECT nickname FROM profile WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .unwrap()
        .flatten()
}

/// True if a TOTP config still exists for the user.
async fn read_totp_exists(ctx: &TestContext, user_id: Uuid) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM user_totp_config WHERE user_id = $1)",
    )
    .bind(user_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .unwrap()
}

/// Read current subscription status.
async fn read_subscription_status(ctx: &TestContext, sub_id: Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT status FROM subscription WHERE id = $1")
        .bind(sub_id)
        .fetch_optional(&ctx.app_state.pool)
        .await
        .unwrap()
}

/// Read all audit events where the user is actor or target.
async fn read_audit_events_for(
    ctx: &TestContext,
    user_id: Uuid,
) -> Vec<(String, String, Option<serde_json::Value>)> {
    sqlx::query(
        "SELECT category, action, details FROM audit_events
         WHERE actor_id = $1 OR target_id = $1
         ORDER BY created_at DESC",
    )
    .bind(user_id.to_string())
    .fetch_all(&ctx.app_state.pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.get("category"), row.get("action"), row.get("details")))
    .collect()
}

/// Hit a protected endpoint with a token and return the status.
async fn protected_endpoint_status(ctx: &TestContext, token: &str) -> StatusCode {
    let app = ctx.create_unified_test_router();
    let req = Request::builder()
        .method("GET")
        .uri("/api/user/profile")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    app.oneshot(req).await.unwrap().status()
}

// =============================================================================
// Scenario Tests
// =============================================================================

/// ============================================================================
/// User Story: US-RU-014
/// Covers: Design §4.2.2 — wrong password returns 401, account unchanged
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_delete_account_wrong_password_returns_401(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let password = "CorrectPassword123!";
    let (user_id, email) = seed_normal_user_with_password(ctx, &realm_id, password).await;
    let token = login_and_get_token(ctx, &realm_id, &email, password).await;

    let before = read_account_row(ctx, user_id).await;

    let resp = call_delete_account(ctx, &token, "wrong-password").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let after = read_account_row(ctx, user_id).await;
    assert_eq!(
        after.0, 1,
        "Account status must remain Normal after failed delete"
    );
    assert_eq!(
        after.1, before.1,
        "Email must not change after failed delete"
    );
    assert!(
        after.2.is_some(),
        "Password hash must remain after failed delete"
    );
}

/// ============================================================================
/// User Story: US-RU-014
/// Covers: Design §4.2.2 / §5.2 — a second delete attempt on an already-deleted
///         account is rejected. Because self-delete revokes ALL sessions
///         (including the caller's), the reused token is invalidated by the
///         auth layer before the endpoint can observe the account state, so the
///         observable status is 401 rather than 409.
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_delete_account_already_deleted_returns_401(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let password = "CorrectPassword123!";
    let (user_id, email) = seed_normal_user_with_password(ctx, &realm_id, password).await;
    let token = login_and_get_token(ctx, &realm_id, &email, password).await;

    let first = call_delete_account(ctx, &token, password).await;
    assert_eq!(first.status(), StatusCode::NO_CONTENT);

    let second = call_delete_account(ctx, &token, password).await;
    assert_eq!(
        second.status(),
        StatusCode::UNAUTHORIZED,
        "reusing the revoked session after self-delete must be rejected with 401"
    );

    let (status, _, _, _, _) = read_account_row(ctx, user_id).await;
    assert_eq!(status, 4, "Account must remain Deleted");
}

/// ============================================================================
/// User Story: US-RU-014
/// Covers: Design §5.2 — successful deletion anonymizes PII (status=4,
///         derived email, NULL password/username, empty provider_ids,
///         NULL profile nickname)
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_delete_account_success_anonymizes_pii(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let password = "CorrectPassword123!";
    let (user_id, email) = seed_normal_user_with_password(ctx, &realm_id, password).await;
    let token = login_and_get_token(ctx, &realm_id, &email, password).await;

    let resp = call_delete_account(ctx, &token, password).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let (status, email_after, password_after, username_after, provider_ids_after) =
        read_account_row(ctx, user_id).await;

    assert_eq!(status, 4, "Account status must be Deleted");
    assert_eq!(
        email_after,
        format!("deleted+{}@anonymized.local", user_id),
        "Email must be derived from account id"
    );
    assert!(password_after.is_none(), "Password must be NULL");
    assert!(username_after.is_none(), "Username must be NULL");
    assert!(provider_ids_after.is_empty(), "provider_ids must be empty");

    let nickname_after = read_profile_nickname(ctx, user_id).await;
    assert!(nickname_after.is_none(), "Profile nickname must be NULL");
}

/// ============================================================================
/// User Story: US-RU-014
/// Covers: Design §5.2 — TOTP config and backup codes are wiped on deletion
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_delete_account_deletes_totp(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let password = "CorrectPassword123!";
    let (user_id, _email) = seed_normal_user_with_password(ctx, &realm_id, password).await;
    seed_totp_config(ctx, user_id).await;
    assert!(
        read_totp_exists(ctx, user_id).await,
        "TOTP config must exist before delete"
    );

    // Create a session directly: this test covers TOTP deletion, not the login
    // flow with TOTP enabled (which is explicitly out of scope for the consent
    // gate item and would return `requires_totp` without a session cookie).
    let token = create_extra_session(ctx, user_id).await;
    let resp = call_delete_account(ctx, &token, password).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    assert!(
        !read_totp_exists(ctx, user_id).await,
        "TOTP config must be deleted after account deletion"
    );
}

/// ============================================================================
/// User Story: US-RU-014
/// Covers: Design §5.2 — in-effect subscriptions are cancelled immediately
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_delete_account_cancels_active_subscriptions(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let password = "CorrectPassword123!";
    let (user_id, email) = seed_normal_user_with_password(ctx, &realm_id, password).await;

    let active_sub = seed_active_subscription(ctx, &realm_id, user_id, "active").await;
    let trialing_sub = seed_active_subscription(ctx, &realm_id, user_id, "trialing").await;

    let token = login_and_get_token(ctx, &realm_id, &email, password).await;
    let resp = call_delete_account(ctx, &token, password).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    assert_eq!(
        read_subscription_status(ctx, active_sub).await.as_deref(),
        Some("canceled"),
        "Active subscription must be canceled"
    );
    assert_eq!(
        read_subscription_status(ctx, trialing_sub).await.as_deref(),
        Some("canceled"),
        "Trialing subscription must be canceled"
    );
}

/// ============================================================================
/// User Story: US-RU-014
/// Covers: Design §5.2/§6.3 — delete_user_sessions revokes the caller's
///         current session and any other session for the user
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_delete_account_revokes_all_sessions(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let password = "CorrectPassword123!";
    let (user_id, email) = seed_normal_user_with_password(ctx, &realm_id, password).await;

    let current_token = login_and_get_token(ctx, &realm_id, &email, password).await;
    let extra_token = create_extra_session(ctx, user_id).await;

    // Sanity: both tokens can access a protected endpoint before deletion.
    assert_eq!(
        protected_endpoint_status(ctx, &current_token).await,
        StatusCode::OK
    );
    assert_eq!(
        protected_endpoint_status(ctx, &extra_token).await,
        StatusCode::OK
    );

    let resp = call_delete_account(ctx, &current_token, password).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    assert_eq!(
        protected_endpoint_status(ctx, &current_token).await,
        StatusCode::UNAUTHORIZED,
        "Current session must be revoked after self-delete"
    );
    assert_eq!(
        protected_endpoint_status(ctx, &extra_token).await,
        StatusCode::UNAUTHORIZED,
        "Extra session must also be revoked after self-delete"
    );
}

/// ============================================================================
/// User Story: US-RU-014
/// Covers: Design §5.2 — user.delete audit event with method=self_service
///         and anonymized=true is recorded under Compliance category
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_delete_account_writes_audit_self_service(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let password = "CorrectPassword123!";
    let (user_id, email) = seed_normal_user_with_password(ctx, &realm_id, password).await;
    let token = login_and_get_token(ctx, &realm_id, &email, password).await;

    let resp = call_delete_account(ctx, &token, password).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let events = read_audit_events_for(ctx, user_id).await;
    let delete_event = events
        .iter()
        .find(|(_, action, _)| action == "user.delete")
        .expect("user.delete audit event must exist");

    assert_eq!(delete_event.0, "compliance", "Category must be Compliance");
    let details = delete_event
        .2
        .as_ref()
        .expect("Audit details must be present");
    assert_eq!(details["method"], "self_service");
    assert_eq!(details["anonymized"], true);
}

/// ============================================================================
/// User Story: US-RU-014
/// Covers: Design §3.1/§5.2 — deleted account cannot log back in
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_login_fails_after_delete(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let password = "CorrectPassword123!";
    let (user_id, email) = seed_normal_user_with_password(ctx, &realm_id, password).await;
    let token = login_and_get_token(ctx, &realm_id, &email, password).await;

    let resp = call_delete_account(ctx, &token, password).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let login_resp = login_with_credentials(ctx, &realm_id, &email, password).await;
    assert_eq!(
        login_resp.status(),
        StatusCode::FORBIDDEN,
        "Login with original credentials after deletion must be forbidden"
    );

    let (status, _, _, _, _) = read_account_row(ctx, user_id).await;
    assert_eq!(status, 4);
}

/// ============================================================================
/// User Story: US-RU-014
/// Covers: Design §5.2/§6.1 — one-time purchases are not refunded on
///         self-delete (no refund audit, no change to payment attempt)
/// ============================================================================
#[test_context(TestContext)]
#[tokio::test]
async fn test_delete_account_does_not_refund_one_time_purchase(ctx: &mut TestContext) {
    let realm_id = ctx._realm_id.clone();
    let password = "CorrectPassword123!";
    let (user_id, email) = seed_normal_user_with_password(ctx, &realm_id, password).await;
    let attempt_id = seed_one_time_purchase(ctx, &realm_id, user_id).await;

    let token = login_and_get_token(ctx, &realm_id, &email, password).await;
    let resp = call_delete_account(ctx, &token, password).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // No refund-related audit event should be recorded for this user.
    let events = read_audit_events_for(ctx, user_id).await;
    let refund_audit = events
        .iter()
        .any(|(_, action, _)| action.contains("refund"));
    assert!(!refund_audit, "No refund audit event must be written");

    // The original payment attempt must remain unchanged.
    let attempt: Option<(String, i64)> =
        sqlx::query_as("SELECT status, amount FROM payment_attempts WHERE id = $1")
            .bind(attempt_id)
            .fetch_optional(&ctx.app_state.pool)
            .await
            .unwrap();

    let (status, amount) = attempt.expect("Payment attempt row must still exist");
    assert_eq!(status, "Succeeded");
    assert_eq!(amount, 999);
}
