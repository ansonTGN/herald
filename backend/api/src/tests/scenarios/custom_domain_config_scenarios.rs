use crate::application::http::server::create_api_routes;
use crate::tests::helpers::*;
use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use herald_core::domain::realm_config::ConfigType;
use serde_json::{Value, json};
use test_context::test_context;
use tower::ServiceExt;

const CUSTOM_DOMAIN_PATH: &str = "/api/realms/{realm}/config/custom-domain";

fn custom_domain_uri(realm_id: &str, suffix: &str) -> String {
    format!(
        "{}{}",
        CUSTOM_DOMAIN_PATH.replace("{realm}", realm_id),
        suffix
    )
}

fn authed_request(method: &str, uri: String, token: &str, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, format!("X-Auth={token}"));

    let body = if let Some(body) = body {
        builder = builder.header("content-type", "application/json");
        Body::from(body.to_string())
    } else {
        Body::empty()
    };

    builder.body(body).unwrap()
}

/// Insert a `custom_domain` row into `realm_config` for the context's realm.
///
/// Mirrors the white-label in-file helper but persists
/// `ConfigType::CustomDomain` (`config_type = 'custom_domain'`). The hostname
/// is stored already-normalized so the uniqueness check (handler scans
/// `realm_config` for a `"hostname":"<value>"` JSON substring) sees a
/// well-formed value.
async fn insert_custom_domain_config(ctx: &TestContext, config_key: &str, hostname: Option<&str>) {
    let config_value = json!({ "hostname": hostname }).to_string();
    sqlx::query(
        "INSERT INTO realm_config (realm_id, config_type, config_key, config_value, is_secret, enabled, metadata, created_at, updated_at)
         VALUES ($1, 'custom_domain', $2, $3, false, true, '{}'::jsonb, NOW(), NOW())
         ON CONFLICT (realm_id, config_type, config_key)
         DO UPDATE SET config_value = EXCLUDED.config_value, enabled = EXCLUDED.enabled, updated_at = NOW()",
    )
    .bind(&ctx._realm_id)
    .bind(config_key)
    .bind(config_value)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to upsert custom-domain config");
}

/// Fetch a raw `custom_domain` `realm_config.config_value` for the context's realm.
async fn fetch_custom_domain_config(ctx: &TestContext, config_key: &str) -> Option<String> {
    sqlx::query_scalar(
        "SELECT config_value FROM realm_config
         WHERE realm_id = $1 AND config_type = 'custom_domain' AND config_key = $2",
    )
    .bind(&ctx._realm_id)
    .bind(config_key)
    .fetch_optional(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch custom-domain config")
}

/// Insert a published `custom_domain_mapping` row for an arbitrary realm.
///
/// `enabled` defaults true (the unified request-time effectiveness predicate,
/// design §5.1「生效判定」). `cname_verified`/`tls_ready` are surface-only and
/// default false. Used to seed cross-realm hostname occupation (draft 409) and
/// to simulate a Caddy-issued mapping for the ask endpoint.
async fn insert_custom_domain_mapping(
    ctx: &TestContext,
    realm_id: &str,
    hostname: &str,
    enabled: bool,
) {
    sqlx::query(
        "INSERT INTO custom_domain_mapping (realm_id, hostname, enabled, cname_verified, tls_ready, created_at, updated_at)
         VALUES ($1, $2, $3, false, false, NOW(), NOW())
         ON CONFLICT (hostname)
         DO UPDATE SET realm_id = EXCLUDED.realm_id, enabled = EXCLUDED.enabled, updated_at = NOW()",
    )
    .bind(realm_id)
    .bind(hostname)
    .bind(enabled)
    .execute(&ctx._app_state.pool)
    .await
    .expect("Failed to upsert custom-domain mapping");
}

/// User Story: US-CD-001 — Realm Admin custom-domain settings must not be routed to another config type.
/// Covers: design §4.2.3 (`ConfigType::CustomDomain` string mappings)
#[test]
fn custom_domain_config_type_string_mappings_are_registered() {
    assert_eq!(
        ConfigType::try_from_str("custom_domain"),
        Ok(ConfigType::CustomDomain)
    );
    assert_eq!(String::from(ConfigType::CustomDomain), "custom_domain");
    assert_eq!(ConfigType::CustomDomain.as_ref(), "custom_domain");
}

/// User Story: US-CD-001 — Realm Admin opens custom-domain settings before any configuration exists.
/// Covers: design §4.2.2 (GET management state defaults)
#[test_context(TestContext)]
#[tokio::test]
async fn custom_domain_get_returns_empty_state_when_unconfigured(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (token, user_id) =
        create_admin_session_with_user(ctx, "custom-domain-empty@test.com", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    let req = authed_request("GET", custom_domain_uri(&ctx._realm_id, ""), &token, None);

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = crate::tests::response_json(resp).await;
    // published is CustomDomainConfig::default() — a non-null object with a null hostname.
    assert_eq!(body["published"]["hostname"], Value::Null);
    assert!(body["draft"].is_null());
    assert_eq!(body["hasPrevious"], false);
    // cnameTarget is a global config string (empty in the test context) but must be present.
    assert!(body.get("cnameTarget").is_some());
    assert!(body["status"].is_null());
}

/// User Story: US-CD-001 — Realm Admin saves an unpublished custom-domain draft.
/// Covers: design §4.2.2 (PUT draft), §4.5 (`settings.manage`)
#[test_context(TestContext)]
#[tokio::test]
async fn custom_domain_draft_requires_manage_and_does_not_publish(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (plain_token, _plain_user_id) =
        create_admin_session_with_user(ctx, "custom-domain-draft-plain@test.com", 1800).await;
    let draft = json!({ "hostname": "login.example.com" });

    // Without settings.manage → 403.
    let forbidden_req = authed_request(
        "PUT",
        custom_domain_uri(&ctx._realm_id, "/draft"),
        &plain_token,
        Some(draft.clone()),
    );
    let forbidden_resp = app.clone().oneshot(forbidden_req).await.unwrap();
    assert_eq!(forbidden_resp.status(), StatusCode::FORBIDDEN);

    let (admin_token, admin_user_id) =
        create_admin_session_with_user(ctx, "custom-domain-draft-admin@test.com", 1800).await;
    grant_realm_admin_role(ctx, &admin_user_id).await;

    let req = authed_request(
        "PUT",
        custom_domain_uri(&ctx._realm_id, "/draft"),
        &admin_token,
        Some(draft),
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = crate::tests::response_json(resp).await;
    assert_eq!(body["message"], "Custom-domain draft saved");
    assert_eq!(body["draft"]["hostname"], "login.example.com");

    // Draft must be persisted but NOT published: no settings row, no mapping.
    assert!(fetch_custom_domain_config(ctx, "draft").await.is_some());
    assert!(
        fetch_custom_domain_config(ctx, "settings").await.is_none(),
        "Saving draft must not publish settings"
    );
    let mapping_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM custom_domain_mapping WHERE realm_id = $1")
            .bind(&ctx._realm_id)
            .fetch_one(&ctx._app_state.pool)
            .await
            .expect("Failed to count mappings");
    assert_eq!(
        mapping_count, 0,
        "Saving a draft must not write the host→realm mapping"
    );
}

/// User Story: US-CD-001 — Regular users cannot view Realm Admin custom-domain state.
/// Covers: design §4.2.2 (GET management state), §4.5 (`settings.view`)
#[test_context(TestContext)]
#[tokio::test]
async fn custom_domain_get_requires_view_and_forbids_plain_user(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (token, _user_id) =
        create_admin_session_with_user(ctx, "custom-domain-view-plain@test.com", 1800).await;

    let req = authed_request("GET", custom_domain_uri(&ctx._realm_id, ""), &token, None);

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

/// User Story: US-CD-001 / US-CD-004 — Custom-domain settings are isolated to the user's own Realm.
/// Covers: design §4.5 (cross-Realm access forbidden)
#[test_context(TestContext)]
#[tokio::test]
async fn custom_domain_cross_realm_access_is_forbidden(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (token, user_id) =
        create_admin_session_with_user(ctx, "custom-domain-cross-realm@test.com", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;
    let other_realm_id = uuid::Uuid::now_v7().to_string();

    let get_req = authed_request("GET", custom_domain_uri(&other_realm_id, ""), &token, None);
    let get_resp = app.clone().oneshot(get_req).await.unwrap();
    assert_eq!(get_resp.status(), StatusCode::FORBIDDEN);

    let put_req = authed_request(
        "PUT",
        custom_domain_uri(&other_realm_id, "/draft"),
        &token,
        Some(json!({ "hostname": "login.other-realm.com" })),
    );
    let put_resp = app.oneshot(put_req).await.unwrap();
    assert_eq!(put_resp.status(), StatusCode::FORBIDDEN);
}

/// User Story: US-CD-001 / US-CD-004 — Realm Admin cannot save an unsafe or malformed hostname.
/// Covers: design §4.5 (hostname normalization & validation), §4.2.2 (PUT draft 400)
#[test_context(TestContext)]
#[tokio::test]
async fn custom_domain_draft_rejects_invalid_hostname(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (token, user_id) =
        create_admin_session_with_user(ctx, "custom-domain-validation@test.com", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    // Wildcard hostnames are rejected.
    let wildcard_req = authed_request(
        "PUT",
        custom_domain_uri(&ctx._realm_id, "/draft"),
        &token,
        Some(json!({ "hostname": "*.example.com" })),
    );
    let wildcard_resp = app.clone().oneshot(wildcard_req).await.unwrap();
    assert_eq!(wildcard_resp.status(), StatusCode::BAD_REQUEST);

    // Hostnames with a port are rejected.
    let port_req = authed_request(
        "PUT",
        custom_domain_uri(&ctx._realm_id, "/draft"),
        &token,
        Some(json!({ "hostname": "login.example.com:8443" })),
    );
    let port_resp = app.clone().oneshot(port_req).await.unwrap();
    assert_eq!(port_resp.status(), StatusCode::BAD_REQUEST);

    // A scheme-prefixed URL is rejected (caller pasted a full URL, not a hostname).
    let scheme_req = authed_request(
        "PUT",
        custom_domain_uri(&ctx._realm_id, "/draft"),
        &token,
        Some(json!({ "hostname": "https://login.example.com" })),
    );
    let scheme_resp = app.clone().oneshot(scheme_req).await.unwrap();
    assert_eq!(scheme_resp.status(), StatusCode::BAD_REQUEST);

    // A valid mixed-case hostname with a trailing dot is accepted and normalized
    // (lowercased, trailing dot stripped) per design §4.5.
    let normalized_req = authed_request(
        "PUT",
        custom_domain_uri(&ctx._realm_id, "/draft"),
        &token,
        Some(json!({ "hostname": "Login.Example.COM." })),
    );
    let normalized_resp = app.oneshot(normalized_req).await.unwrap();
    assert_eq!(normalized_resp.status(), StatusCode::OK);

    let body: Value = crate::tests::response_json(normalized_resp).await;
    assert_eq!(body["draft"]["hostname"], "login.example.com");
}

/// User Story: US-CD-001 — A custom-domain hostname is globally unique across all Realms.
/// Covers: design §4.2.2 (PUT draft 409 global uniqueness)
#[test_context(TestContext)]
#[tokio::test]
async fn custom_domain_draft_409_on_hostname_taken_across_realms(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (token, user_id) =
        create_admin_session_with_user(ctx, "custom-domain-conflict@test.com", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    // Another realm already occupies the hostname via a published mapping row.
    let other_realm_id = uuid::Uuid::now_v7().to_string();
    insert_custom_domain_mapping(ctx, &other_realm_id, "taken.example.com", true).await;

    let req = authed_request(
        "PUT",
        custom_domain_uri(&ctx._realm_id, "/draft"),
        &token,
        Some(json!({ "hostname": "taken.example.com" })),
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

/// User Story: US-CD-003 — Realm Admin discards drafts without changing published branding.
/// Covers: design §4.2.2 (DELETE draft idempotence)
#[test_context(TestContext)]
#[tokio::test]
async fn custom_domain_delete_draft_is_idempotent_and_preserves_published(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (token, user_id) =
        create_admin_session_with_user(ctx, "custom-domain-delete-draft@test.com", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;
    let published_hostname = "published.example.com";
    insert_custom_domain_config(ctx, "settings", Some(published_hostname)).await;
    insert_custom_domain_config(ctx, "draft", Some("draft.example.com")).await;

    for _ in 0..2 {
        let req = authed_request(
            "DELETE",
            custom_domain_uri(&ctx._realm_id, "/draft"),
            &token,
            None,
        );
        let resp = app.clone().oneshot(req).await.unwrap();
        // DELETE draft is idempotent: 200 on every call whether or not a draft existed.
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = crate::tests::response_json(resp).await;
        assert_eq!(body["message"], "Custom-domain draft discarded");
    }

    // Published settings survive repeated deletes; the draft is gone.
    let settings = fetch_custom_domain_config(ctx, "settings")
        .await
        .expect("published settings must survive draft deletion");
    assert!(settings.contains(published_hostname));
    assert!(fetch_custom_domain_config(ctx, "draft").await.is_none());
}

/// User Story: US-CD-003 — Realm Admin publishes a draft while keeping one-step rollback available.
/// Covers: design §4.2.2 (POST publish), §5.1 (mapping write on publish)
#[test_context(TestContext)]
#[tokio::test]
async fn custom_domain_publish_writes_mapping_and_clears_draft(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (token, user_id) =
        create_admin_session_with_user(ctx, "custom-domain-publish@test.com", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;
    let old_hostname = "old.example.com";
    let new_hostname = "new.example.com";
    insert_custom_domain_config(ctx, "settings", Some(old_hostname)).await;
    insert_custom_domain_config(ctx, "draft", Some(new_hostname)).await;

    let req = authed_request(
        "POST",
        custom_domain_uri(&ctx._realm_id, "/publish"),
        &token,
        None,
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = crate::tests::response_json(resp).await;
    assert_eq!(body["message"], "Custom-domain configuration published");
    assert_eq!(body["hasPrevious"], true);
    // publish writes the mapping with status pending (enabled, not yet verified).
    assert_eq!(body["status"]["cnameVerified"], false);
    assert_eq!(body["status"]["tlsReady"], false);

    // settings now holds the new hostname; the draft is cleared; previous holds the old one.
    let settings = fetch_custom_domain_config(ctx, "settings")
        .await
        .expect("settings row must exist after publish");
    assert!(settings.contains(new_hostname));
    assert!(
        !settings.contains(old_hostname),
        "published settings must reflect the newly published hostname"
    );
    let previous = fetch_custom_domain_config(ctx, "previous_settings")
        .await
        .expect("previous_settings snapshot must exist after publish");
    assert!(previous.contains(old_hostname));
    assert!(fetch_custom_domain_config(ctx, "draft").await.is_none());

    // The host→realm mapping reflects the published hostname in its pending state.
    let row: Option<(bool, bool)> = sqlx::query_as(
        "SELECT cname_verified, tls_ready FROM custom_domain_mapping
         WHERE realm_id = $1 AND hostname = $2",
    )
    .bind(&ctx._realm_id)
    .bind(new_hostname)
    .fetch_optional(&ctx._app_state.pool)
    .await
    .expect("Failed to fetch mapping");
    let (cname_verified, tls_ready) =
        row.expect("publish must write a mapping row for the new hostname");
    assert!(
        !cname_verified && !tls_ready,
        "freshly published mapping must start pending (cname_verified=false, tls_ready=false)"
    );
}

/// User Story: US-CD-003 — Publishing requires an existing draft with a hostname.
/// Covers: design §4.2.2 (POST publish bad request)
#[test_context(TestContext)]
#[tokio::test]
async fn custom_domain_publish_without_draft_or_body_returns_bad_request(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (token, user_id) =
        create_admin_session_with_user(ctx, "custom-domain-publish-empty@test.com", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    // No draft exists at all → 400.
    let req = authed_request(
        "POST",
        custom_domain_uri(&ctx._realm_id, "/publish"),
        &token,
        None,
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// User Story: US-CD-003 — Realm Admin can restore the previous published custom-domain settings.
/// Covers: design §4.2.2 (POST restore), §5.1 (mapping rollback on restore)
#[test_context(TestContext)]
#[tokio::test]
async fn custom_domain_restore_swaps_settings_and_rolls_back_mapping(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (token, user_id) =
        create_admin_session_with_user(ctx, "custom-domain-restore@test.com", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;
    let current_hostname = "broken.example.com";
    let previous_hostname = "stable.example.com";
    insert_custom_domain_config(ctx, "settings", Some(current_hostname)).await;
    insert_custom_domain_config(ctx, "previous_settings", Some(previous_hostname)).await;
    // Simulate the current published mapping the restore must roll back from.
    insert_custom_domain_mapping(ctx, &ctx._realm_id, current_hostname, true).await;

    let req = authed_request(
        "POST",
        custom_domain_uri(&ctx._realm_id, "/restore"),
        &token,
        None,
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = crate::tests::response_json(resp).await;
    assert_eq!(
        body["message"],
        "Previous custom-domain configuration restored"
    );
    assert_eq!(body["hasPrevious"], true);

    // settings ↔ previous_settings are swapped: settings holds the previous hostname,
    // previous_settings holds what was just replaced.
    let settings = fetch_custom_domain_config(ctx, "settings")
        .await
        .expect("settings must exist after restore");
    assert!(settings.contains(previous_hostname));
    let previous = fetch_custom_domain_config(ctx, "previous_settings")
        .await
        .expect("previous_settings must exist after restore");
    assert!(previous.contains(current_hostname));

    // Mapping is rolled back: the restored (previous) hostname is the realm's
    // effective published mapping.
    let restored_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM custom_domain_mapping WHERE realm_id = $1 AND hostname = $2",
    )
    .bind(&ctx._realm_id)
    .bind(previous_hostname)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to count restored mapping");
    assert_eq!(
        restored_count, 1,
        "restore must (re)publish the previous hostname as the realm's mapping"
    );

    // The superseded current hostname mapping row is removed on rollback
    // (design §4.2.2 restore: 「被替换下的 hostname 行删除」 — the replaced
    // hostname row is deleted, not just disabled). `upsert_for_realm` enforces
    // the at-most-one-enabled-row-per-realm invariant by deleting every other
    // hostname for the realm; assert that rollback actually achieved it.
    let superseded_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM custom_domain_mapping WHERE realm_id = $1 AND hostname = $2",
    )
    .bind(&ctx._realm_id)
    .bind(current_hostname)
    .fetch_one(&ctx._app_state.pool)
    .await
    .expect("Failed to count superseded mapping");
    assert_eq!(
        superseded_count, 0,
        "restore must delete the superseded current hostname mapping row"
    );
}

/// User Story: US-CD-003 — Restore without a previous snapshot is rejected.
/// Covers: design §4.2.2 (POST restore bad request)
#[test_context(TestContext)]
#[tokio::test]
async fn custom_domain_restore_without_previous_returns_bad_request(ctx: &mut TestContext) {
    let app = ctx.create_unified_test_router();
    let (token, user_id) =
        create_admin_session_with_user(ctx, "custom-domain-restore-empty@test.com", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;

    // No previous_settings snapshot exists → 400.
    let req = authed_request(
        "POST",
        custom_domain_uri(&ctx._realm_id, "/restore"),
        &token,
        None,
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// User Story: US-CD-002 — Publishing a custom-domain config must write the
/// hostname into the `custom_domain_mapping` table so Caddy On-Demand TLS
/// issuance (the ask endpoint) and future per-realm lookups reflect it.
/// Covers: design §4.2.2 publish side-effect (mapping write).
///
/// Note: the public host→realmId resolve endpoint and per-domain URL generation
/// were reverted; the retained read surface for the mapping table is the Caddy
/// ask endpoint (`GET /api/internal/custom-domain/authorize`), which we use here
/// to confirm the published hostname was committed and is effective.
#[test_context(TestContext)]
#[tokio::test]
async fn custom_domain_publish_published_link_uses_custom_domain_host(ctx: &mut TestContext) {
    // ask-key-gated router: the authorize endpoint needs a non-empty configured
    // key to return 200 (the default test context key is empty → always 401).
    let mut state = (*ctx._app_state).clone();
    state.custom_domain_ask_key = "test-ask-shared-secret".to_string();
    let ask_key = state.custom_domain_ask_key.clone();
    let app = create_api_routes(std::sync::Arc::new(state.clone())).with_state(state);

    let (token, user_id) =
        create_admin_session_with_user(ctx, "custom-domain-publish-mapping@test.com", 1800).await;
    grant_realm_admin_role(ctx, &user_id).await;
    let hostname = "login.publish-mapping-example.com";

    insert_custom_domain_config(ctx, "draft", Some(hostname)).await;
    let publish_req = authed_request(
        "POST",
        custom_domain_uri(&ctx._realm_id, "/publish"),
        &token,
        None,
    );
    let publish_resp = app.clone().oneshot(publish_req).await.unwrap();
    assert_eq!(publish_resp.status(), StatusCode::OK);

    // The Caddy ask endpoint (retained read surface) must now authorize the
    // just-published hostname — proving the publish wrote the mapping row.
    let ask_uri = format!("/api/internal/custom-domain/authorize?host={hostname}");
    let ask_req = Request::builder()
        .method("GET")
        .uri(ask_uri)
        .header("x-herald-ask-key", &ask_key)
        .body(Body::empty())
        .unwrap();
    let ask_resp = app.oneshot(ask_req).await.unwrap();
    assert_eq!(ask_resp.status(), StatusCode::OK);

    let body: Value = crate::tests::response_json(ask_resp).await;
    assert_eq!(body["authorized"], true);
}
