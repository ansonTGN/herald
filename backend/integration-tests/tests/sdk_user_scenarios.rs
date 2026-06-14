// =============================================================================
// SDK User Scenarios
// =============================================================================
//
// Test SDK user management API against real API.
// Covers US-TP-013: User CRUD via SDK (create, list, detail).
//
// =============================================================================

use herald_core::domain::authorization::permission_service::PermissionService;
use herald_core::domain::client_api_keys::services::ClientApiKeyService;
use herald_sdk::{Client, CreateUserSdkRequest, Error};
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
        "INSERT INTO client_api_keys (id, name, api_key_hash, realm_id, client_app_id, enabled, expires_at, created_at, last_used_at)
         VALUES ($1, $2, $3, $4, NULL, true, NULL, NOW(), NULL)",
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
// Test 1: Create user success
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-013 Scenario 1
// Covers: API Key with users:create in realm R -> create user succeeds

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_user_create_success(ctx: &mut SchemaTestContext) {
    // Given: API key in realm R with users:create permission
    let (api_key, _key_id, _realm) = setup_api_key_with_permissions(
        ctx,
        &ctx._realm_id,
        &ctx._client_id,
        &[("users", "manage")],
    )
    .await;

    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url, api_key, None);

    // When: calling create_user with valid input
    let request = CreateUserSdkRequest {
        email: format!("sdk-user-create-{}@test.com", Uuid::now_v7()),
        password: "Password123!".to_string(),
        nickname: Some("sdk-test-user".to_string()),
    };

    let result = client.create_user(&ctx._realm_id, request).await;

    // Then: user is created successfully
    if let Err(e) = &result {
        eprintln!("create_user error: {:?}", e);
    }
    assert!(
        result.is_ok(),
        "create_user should succeed: {:?}",
        result.err()
    );
    let user = result.unwrap();
    assert!(!user.id.is_empty(), "User ID should be populated");
    assert!(user.email.contains("@test.com"), "Email should match");

    handle.abort();
}

// ---------------------------------------------------------------------------
// Test 2: Create user email duplicate
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-013 Scenario 4
// Covers: Same email in same realm -> 409 conflict

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_user_create_email_duplicate(ctx: &mut SchemaTestContext) {
    // Given: API key with users:create and a user already exists in the realm
    let (api_key, _key_id, _realm) = setup_api_key_with_permissions(
        ctx,
        &ctx._realm_id,
        &ctx._client_id,
        &[("users", "manage")],
    )
    .await;

    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url.clone(), api_key.clone(), None);

    let duplicate_email = format!("dup-user-{}@test.com", Uuid::now_v7());

    // Create the first user successfully
    let first_request = CreateUserSdkRequest {
        email: duplicate_email.clone(),
        password: "Password123!".to_string(),
        nickname: None,
    };
    let first_result = client.create_user(&ctx._realm_id, first_request).await;
    if let Err(e) = &first_result {
        eprintln!("First create_user error: {:?}", e);
    }
    assert!(first_result.is_ok(), "First create_user should succeed");

    // When: attempting to create a second user with the same email
    let second_request = CreateUserSdkRequest {
        email: duplicate_email.clone(),
        password: "Password456!".to_string(),
        nickname: None,
    };
    let second_result = client.create_user(&ctx._realm_id, second_request).await;

    // Then: returns conflict error (409)
    assert!(
        second_result.is_err(),
        "Duplicate email should return error"
    );
    match second_result.unwrap_err() {
        Error::ApiError { status, message } => {
            assert_eq!(
                status, 409,
                "Expected 409 status for duplicate email, got {}: {}",
                status, message
            );
        }
        other => panic!("Expected ApiError with 409, got: {:?}", other),
    }

    handle.abort();
}

// ---------------------------------------------------------------------------
// Test 3: Create user validation error
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-013 (validation)
// Covers: Short password or empty email -> 400

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_user_create_validation_error(ctx: &mut SchemaTestContext) {
    // Given: API key with users:create permission
    let (api_key, _key_id, _realm) = setup_api_key_with_permissions(
        ctx,
        &ctx._realm_id,
        &ctx._client_id,
        &[("users", "manage")],
    )
    .await;

    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url, api_key, None);

    // When: calling create_user with a short password
    let request = CreateUserSdkRequest {
        email: format!("valid-{}@test.com", Uuid::now_v7()),
        password: "ab".to_string(), // too short
        nickname: None,
    };

    let result = client.create_user(&ctx._realm_id, request).await;

    // Then: returns a validation error (400)
    assert!(
        result.is_err(),
        "Short password should return validation error"
    );
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
// Test 4: Create user cross-realm forbidden
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-013 Scenario 5
// Covers: Key in realm A targets realm B -> 403

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_user_create_cross_realm_forbidden(ctx: &mut SchemaTestContext) {
    // Given: a second realm B exists in the database
    let other_realm_id = format!("realm-b-{}", Uuid::now_v7());
    query("INSERT INTO realm (id, name) VALUES ($1, 'Other Realm')")
        .bind(&other_realm_id)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to create second realm");

    // Given: API key in realm A (the test context's default realm) with users:create
    let (api_key, _key_id, _realm) = setup_api_key_with_permissions(
        ctx,
        &ctx._realm_id,
        &ctx._client_id,
        &[("users", "manage")],
    )
    .await;

    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url, api_key, None);

    // When: attempting to create a user in realm B (cross-realm access)
    let request = CreateUserSdkRequest {
        email: format!("cross-realm-{}@test.com", Uuid::now_v7()),
        password: "Password123!".to_string(),
        nickname: None,
    };
    let result = client.create_user(&other_realm_id, request).await;

    // Then: returns 403 Forbidden
    assert!(
        result.is_err(),
        "Cross-realm user creation should be forbidden"
    );
    match result.unwrap_err() {
        Error::Forbidden(_) => {}
        other => panic!("Expected Forbidden, got: {:?}", other),
    }

    handle.abort();
}

// ---------------------------------------------------------------------------
// Test 5: Create user missing permission
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-013 Scenario 6
// Covers: Key lacks users:create -> 403

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_user_create_missing_permission(ctx: &mut SchemaTestContext) {
    // Given: API key in realm R with users:view only (no users:create)
    let (api_key, _key_id, _realm) = setup_api_key_with_permissions(
        ctx,
        &ctx._realm_id,
        &ctx._client_id,
        &[("users", "view")], // missing users:create
    )
    .await;

    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url, api_key, None);

    // When: attempting to create a user
    let request = CreateUserSdkRequest {
        email: format!("no-perm-{}@test.com", Uuid::now_v7()),
        password: "Password123!".to_string(),
        nickname: None,
    };
    let result = client.create_user(&ctx._realm_id, request).await;

    // Then: returns 403 Forbidden
    assert!(
        result.is_err(),
        "Missing users:create permission should return error"
    );
    match result.unwrap_err() {
        Error::Forbidden(_) => {}
        other => panic!("Expected Forbidden, got: {:?}", other),
    }

    handle.abort();
}

// ---------------------------------------------------------------------------
// Test 6: List users success
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-013 Scenario 2
// Covers: Key has users:view -> returns user list

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_user_list_success(ctx: &mut SchemaTestContext) {
    // Given: API key with users:view and users:create (to seed data)
    let (api_key, _key_id, _realm) = setup_api_key_with_permissions(
        ctx,
        &ctx._realm_id,
        &ctx._client_id,
        &[("users", "view"), ("users", "manage")],
    )
    .await;

    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url.clone(), api_key.clone(), None);

    // Create a user so the list is non-empty
    let create_request = CreateUserSdkRequest {
        email: format!("list-user-{}@test.com", Uuid::now_v7()),
        password: "Password123!".to_string(),
        nickname: Some("list-test-user".to_string()),
    };
    let create_result = client.create_user(&ctx._realm_id, create_request).await;
    if let Err(e) = &create_result {
        eprintln!("Seed create_user error: {:?}", e);
    }
    assert!(create_result.is_ok(), "Seed user creation should succeed");

    // When: listing users
    let result = client.list_users(&ctx._realm_id).await;

    // Then: succeeds and returns at least one user
    if let Err(e) = &result {
        eprintln!("list_users error: {:?}", e);
    }
    assert!(
        result.is_ok(),
        "list_users should succeed: {:?}",
        result.err()
    );
    let users = result.unwrap();
    assert!(!users.is_empty(), "User list should not be empty");

    handle.abort();
}

// ---------------------------------------------------------------------------
// Test 7: Get user detail success
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-013 Scenario 3
// Covers: Key has users:view -> returns user detail

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_user_detail_success(ctx: &mut SchemaTestContext) {
    // Given: API key with users:view and users:create (to seed data)
    let (api_key, _key_id, _realm) = setup_api_key_with_permissions(
        ctx,
        &ctx._realm_id,
        &ctx._client_id,
        &[("users", "view"), ("users", "manage")],
    )
    .await;

    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url.clone(), api_key.clone(), None);

    // Create a user first
    let create_request = CreateUserSdkRequest {
        email: format!("detail-user-{}@test.com", Uuid::now_v7()),
        password: "Password123!".to_string(),
        nickname: Some("detail-test-user".to_string()),
    };
    let created = client
        .create_user(&ctx._realm_id, create_request)
        .await
        .expect("Seed user creation should succeed");

    // When: getting user detail
    let result = client.get_user(&ctx._realm_id, &created.id).await;

    // Then: succeeds and returns user info matching the created user
    if let Err(e) = &result {
        eprintln!("get_user error: {:?}", e);
    }
    assert!(
        result.is_ok(),
        "get_user should succeed: {:?}",
        result.err()
    );
    let user = result.unwrap();
    assert_eq!(user.id, created.id, "User ID should match");
    assert_eq!(user.email, created.email, "Email should match");

    handle.abort();
}

// ---------------------------------------------------------------------------
// Test 8: Get user detail not found
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-013 Scenario 5 (variant)
// Covers: Non-existent user -> 404

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_user_detail_not_found(ctx: &mut SchemaTestContext) {
    // Given: API key with users:view in realm R
    let (api_key, _key_id, _realm) =
        setup_api_key_with_permissions(ctx, &ctx._realm_id, &ctx._client_id, &[("users", "view")])
            .await;

    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url, api_key, None);

    // When: querying a non-existent user ID
    let nonexistent_id = Uuid::now_v7().to_string();
    let result = client.get_user(&ctx._realm_id, &nonexistent_id).await;

    // Then: returns 404 Not Found
    assert!(result.is_err(), "Non-existent user should return error");
    match result.unwrap_err() {
        Error::NotFound(_) => {}
        other => panic!("Expected NotFound, got: {:?}", other),
    }

    handle.abort();
}

// ---------------------------------------------------------------------------
// Test 9: Unauthenticated
// ---------------------------------------------------------------------------

// User Story: docs/user-stories/16-sdk-user-stories.md - US-TP-013 (cross-cutting)
// Covers: No API key -> 401

#[test_context(SchemaTestContext)]
#[tokio::test]
async fn test_scenario_sdk_user_unauthenticated(ctx: &mut SchemaTestContext) {
    // Given: an SDK client with an invalid API key
    let (base_url, handle) = start_test_server(ctx).await;
    let client = Client::new(base_url, "invalid-nonexistent-key".to_string(), None);

    // When: calling any user endpoint
    let result = client.list_users(&ctx._realm_id).await;

    // Then: returns 401 Unauthorized
    assert!(result.is_err(), "Invalid API key should return error");
    match result.unwrap_err() {
        Error::Unauthorized(_) => {}
        other => panic!("Expected Unauthorized, got: {:?}", other),
    }

    handle.abort();
}
