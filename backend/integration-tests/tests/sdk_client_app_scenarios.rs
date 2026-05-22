// =============================================================================
// SDK Client App Scenarios
// =============================================================================
//
// Test SDK client app management API against real API.
// Covers US-TP-014: Client App CRUD via SDK (create, list, detail).
//
// =============================================================================

use herald_core::domain::authorization::permission_service::PermissionService;
use herald_core::domain::client_api_keys::services::ClientApiKeyService;
use herald_sdk::{Client, CreateClientAppSdkRequest, Error};
use herald_test_support::SchemaTestContext;
use sqlx::query;
use test_context::test_context;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create an API key in a specific realm, granting specific permissions
/// through a dedicated role bound to the API key principal.
///
/// Returns (api_key_plaintext, api_key_entity_id, realm_id).
async fn setup_api_key_with_permissions(
    ctx: &SchemaTestContext,
    realm_id: &str,
    client_id: &str,
    permissions: &[(&str, &str)],
) -> (String, String, String) {
    // 1. Create a role
    let role_id = Uuid::now_v7();
    query(
        "INSERT INTO roles (id, name, realm_id, client_id, is_builtin)
         VALUES ($1, $2, $3, $4, false)",
    )
    .bind(role_id)
    .bind(format!("test-role-{}", Uuid::now_v7()))
    .bind(realm_id)
    .bind(client_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create role");

    // 2. Grant permissions to the role
    for (resource, action) in permissions {
        query(
            "INSERT INTO role_policies (id, role_id, realm_id, resource, action)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::now_v7())
        .bind(role_id)
        .bind(realm_id)
        .bind(resource)
        .bind(action)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to insert role policy");
    }

    // 3. Create the API key
    let api_key_id = Uuid::now_v7();
    let api_key_plaintext = ClientApiKeyService::generate_api_key();
    let api_key_hash = ClientApiKeyService::hash_api_key(&api_key_plaintext);

    query(
        "INSERT INTO client_api_keys (id, name, api_key_hash, realm_id, client_app_id, enabled, expires_at, created_at, last_used_at, usage_count)
         VALUES ($1, $2, $3, $4, NULL, true, NULL, NOW(), NULL, 0)",
    )
    .bind(api_key_id)
    .bind(format!("test-key-{}", Uuid::now_v7()))
    .bind(&api_key_hash)
    .bind(realm_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to create API key");

    // 4. Bind role to the API key principal via user_roles
    query(
        "INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id, principal_type, principal_id)
         VALUES ($1, $2, $3, $4, $5, 'api_key', $2::text)",
    )
    .bind(api_key_id)
    .bind(api_key_id)
    .bind(role_id)
    .bind(realm_id)
    .bind(client_id)
    .execute(&ctx.app_state.pool)
    .await
    .expect("Failed to bind role to API key principal");

    // 5. Invalidate permission cache so the new binding takes effect
    let _ = ctx
        .app_state
        .permission_checker
        .invalidate_user_role_cache(realm_id, &api_key_id.to_string())
        .await;

    (
        api_key_plaintext,
        api_key_id.to_string(),
        realm_id.to_string(),
    )
}

/// Start a test server on an ephemeral port and return (base_url, abort_handle).
async fn start_test_server(ctx: &SchemaTestContext) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    let router = ctx.create_unified_test_router();
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    (base_url, handle)
}

// ---------------------------------------------------------------------------
// Test 1: Create client app success
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-014 Scenario 1
// Covers: API Key with clients:create in realm R -> create succeeds, returns client_id, client_secret, name

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_client_app_create_success(ctx: &mut SchemaTestContext) {
    // Given: API key in realm R with clients:create permission
    let (api_key, _key_id, _realm) = setup_api_key_with_permissions(
        ctx,
        &ctx._realm_id,
        &ctx._client_id,
        &[("clients", "create")],
    )
    .await;

    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url, api_key, None);

    // When: calling create_client_app with valid input
    let request = CreateClientAppSdkRequest {
        name: "test-client-app".to_string(),
        description: Some("Created by scenario test".to_string()),
        redirect_uris: vec!["https://example.com/callback".to_string()],
    };

    let result = client.create_client_app(&ctx._realm_id, request).await;

    // Then: client app is created successfully with client_id, client_secret, name
    if let Err(e) = &result {
        eprintln!("create_client_app error: {:?}", e);
    }
    assert!(
        result.is_ok(),
        "create_client_app should succeed: {:?}",
        result.err()
    );
    let app = result.unwrap();
    assert!(!app.client_id.is_empty(), "client_id should be populated");
    assert!(
        app.client_secret.is_some(),
        "client_secret should be present in create response"
    );
    assert_eq!(app.name, "test-client-app");

    handle.abort();
}

// ---------------------------------------------------------------------------
// Test 2: Create client app validation error
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-014 Scenario 4
// Covers: Empty name -> 400 validation error

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_client_app_create_validation_error(ctx: &mut SchemaTestContext) {
    // Given: API key with clients:create permission
    let (api_key, _key_id, _realm) = setup_api_key_with_permissions(
        ctx,
        &ctx._realm_id,
        &ctx._client_id,
        &[("clients", "create")],
    )
    .await;

    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url, api_key, None);

    // When: calling create_client_app with an empty name
    let request = CreateClientAppSdkRequest {
        name: "".to_string(),
        description: None,
        redirect_uris: vec![],
    };

    let result = client.create_client_app(&ctx._realm_id, request).await;

    // Then: returns a validation error (400)
    assert!(result.is_err(), "Empty name should return validation error");
    match result.unwrap_err() {
        Error::ApiError { status, message } => {
            assert_eq!(
                status, 400,
                "Expected 400 status for validation error, got {}: {}",
                status, message
            );
        }
        other => panic!("Expected ApiError with 400, got: {:?}", other),
    }

    handle.abort();
}

// ---------------------------------------------------------------------------
// Test 3: Create client app cross-realm forbidden
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-014 (cross-cutting)
// Covers: Key in realm A targets realm B -> 403

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_client_app_create_cross_realm_forbidden(ctx: &mut SchemaTestContext) {
    // Given: a second realm B exists in the database
    let other_realm_id = format!("realm-b-{}", Uuid::now_v7());
    query("INSERT INTO realm (id, name) VALUES ($1, 'Other Realm')")
        .bind(&other_realm_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to create second realm");

    // Given: API key in realm A with clients:create
    let (api_key, _key_id, _realm) = setup_api_key_with_permissions(
        ctx,
        &ctx._realm_id,
        &ctx._client_id,
        &[("clients", "create")],
    )
    .await;

    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url, api_key, None);

    // When: attempting to create a client app in realm B (cross-realm access)
    let request = CreateClientAppSdkRequest {
        name: "cross-realm-app".to_string(),
        description: None,
        redirect_uris: vec![],
    };
    let result = client.create_client_app(&other_realm_id, request).await;

    // Then: returns 403 Forbidden
    assert!(
        result.is_err(),
        "Cross-realm client app creation should be forbidden"
    );
    match result.unwrap_err() {
        Error::Forbidden(_) => {}
        other => panic!("Expected Forbidden, got: {:?}", other),
    }

    handle.abort();
}

// ---------------------------------------------------------------------------
// Test 4: Create client app missing permission
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-014 Scenario 6
// Covers: Key lacks clients:create -> 403

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_client_app_create_missing_permission(ctx: &mut SchemaTestContext) {
    // Given: API key in realm R with clients:view only (no clients:create)
    let (api_key, _key_id, _realm) = setup_api_key_with_permissions(
        ctx,
        &ctx._realm_id,
        &ctx._client_id,
        &[("clients", "view")], // missing clients:create
    )
    .await;

    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url, api_key, None);

    // When: attempting to create a client app
    let request = CreateClientAppSdkRequest {
        name: "no-perm-app".to_string(),
        description: None,
        redirect_uris: vec![],
    };
    let result = client.create_client_app(&ctx._realm_id, request).await;

    // Then: returns 403 Forbidden
    assert!(
        result.is_err(),
        "Missing clients:create permission should return error"
    );
    match result.unwrap_err() {
        Error::Forbidden(_) => {}
        other => panic!("Expected Forbidden, got: {:?}", other),
    }

    handle.abort();
}

// ---------------------------------------------------------------------------
// Test 5: List client apps success
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-014 Scenario 2
// Covers: Key has clients:view -> returns list with id, client_id, name, enabled, created_at

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_client_app_list_success(ctx: &mut SchemaTestContext) {
    // Given: API key with clients:view and clients:create (to seed data)
    let (api_key, _key_id, _realm) = setup_api_key_with_permissions(
        ctx,
        &ctx._realm_id,
        &ctx._client_id,
        &[("clients", "view"), ("clients", "create")],
    )
    .await;

    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url.clone(), api_key.clone(), None);

    // Create a client app so the list is non-empty
    let create_request = CreateClientAppSdkRequest {
        name: "list-test-app".to_string(),
        description: Some("App for list test".to_string()),
        redirect_uris: vec!["https://example.com/cb".to_string()],
    };
    let create_result = client
        .create_client_app(&ctx._realm_id, create_request)
        .await;
    if let Err(e) = &create_result {
        eprintln!("Seed create_client_app error: {:?}", e);
    }
    assert!(
        create_result.is_ok(),
        "Seed client app creation should succeed"
    );

    // When: listing client apps
    let result = client.list_client_apps(&ctx._realm_id).await;

    // Then: succeeds and returns at least one client app with expected fields
    if let Err(e) = &result {
        eprintln!("list_client_apps error: {:?}", e);
    }
    assert!(
        result.is_ok(),
        "list_client_apps should succeed: {:?}",
        result.err()
    );
    let apps = result.unwrap();
    assert!(!apps.is_empty(), "Client app list should not be empty");

    let first = &apps[0];
    assert!(!first.id.is_empty(), "Item id should be populated");
    assert!(
        !first.client_id.is_empty(),
        "Item client_id should be populated"
    );
    assert!(!first.name.is_empty(), "Item name should be populated");

    handle.abort();
}

// ---------------------------------------------------------------------------
// Test 6: Get client app detail success
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-014 Scenario 3
// Covers: Key has clients:view -> returns detail with redirect_uris

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_client_app_detail_success(ctx: &mut SchemaTestContext) {
    // Given: API key with clients:view and clients:create (to seed data)
    let (api_key, _key_id, _realm) = setup_api_key_with_permissions(
        ctx,
        &ctx._realm_id,
        &ctx._client_id,
        &[("clients", "view"), ("clients", "create")],
    )
    .await;

    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url.clone(), api_key.clone(), None);

    // Create a client app first
    let create_request = CreateClientAppSdkRequest {
        name: "detail-test-app".to_string(),
        description: Some("App for detail test".to_string()),
        redirect_uris: vec!["https://example.com/callback".to_string()],
    };
    let created = client
        .create_client_app(&ctx._realm_id, create_request)
        .await
        .expect("Seed client app creation should succeed");

    // When: getting client app detail
    let result = client.get_client_app(&ctx._realm_id, &created.id).await;

    // Then: succeeds and returns full client app info with redirect_uris
    if let Err(e) = &result {
        eprintln!("get_client_app error: {:?}", e);
    }
    assert!(
        result.is_ok(),
        "get_client_app should succeed: {:?}",
        result.err()
    );
    let app = result.unwrap();
    assert_eq!(app.id, created.id, "App ID should match");
    assert_eq!(app.name, "detail-test-app");
    assert!(
        !app.redirect_uris.is_empty(),
        "redirect_uris should be present"
    );
    assert!(
        app.redirect_uris
            .iter()
            .any(|u| u == "https://example.com/callback"),
        "redirect_uris should contain the created URI"
    );

    handle.abort();
}

// ---------------------------------------------------------------------------
// Test 7: Get client app detail not found
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-014 Scenario 5
// Covers: Non-existent client app -> 404

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_client_app_detail_not_found(ctx: &mut SchemaTestContext) {
    // Given: API key with clients:view in realm R
    let (api_key, _key_id, _realm) = setup_api_key_with_permissions(
        ctx,
        &ctx._realm_id,
        &ctx._client_id,
        &[("clients", "view")],
    )
    .await;

    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url, api_key, None);

    // When: querying a non-existent client app ID
    let nonexistent_id = Uuid::now_v7().to_string();
    let result = client.get_client_app(&ctx._realm_id, &nonexistent_id).await;

    // Then: returns 404 Not Found
    assert!(
        result.is_err(),
        "Non-existent client app should return error"
    );
    match result.unwrap_err() {
        Error::NotFound(_) => {}
        other => panic!("Expected NotFound, got: {:?}", other),
    }

    handle.abort();
}

// ---------------------------------------------------------------------------
// Test 8: Unauthenticated
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-014 (cross-cutting)
// Covers: No API key -> 401

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_client_app_unauthenticated(ctx: &mut SchemaTestContext) {
    // Given: an SDK client with an invalid API key
    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url, "invalid-nonexistent-key".to_string(), None);

    // When: calling any client app endpoint
    let result = client.list_client_apps(&ctx._realm_id).await;

    // Then: returns 401 Unauthorized
    assert!(result.is_err(), "Invalid API key should return error");
    match result.unwrap_err() {
        Error::Unauthorized(_) => {}
        other => panic!("Expected Unauthorized, got: {:?}", other),
    }

    handle.abort();
}
