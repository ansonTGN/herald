use crate::application::http::realm::CreateRealmValidator;
use crate::application::http::realm::UpdateRealmValidator;
use crate::application::http::realm::validators::InitialAdminUserValidator;
use crate::tests::helpers::auth_helpers::{create_admin_session_with_user, grant_realm_admin_role};
use crate::tests::response_json;
use crate::tests::schema_test_context::SchemaTestContext;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use test_context::test_context;
use tower::ServiceExt;

// 使用 Schema 隔离的测试上下文，避免测试间数据冲突
use SchemaTestContext as RealmTestContext;

#[test_context(RealmTestContext)]
#[tokio::test]
async fn test_create_realm_success(ctx: &mut RealmTestContext) {
    // Create admin user in admin realm and grant realm-admin role (includes realms.create permission)
    let (admin_token, admin_user_id) =
        create_admin_session_with_user(ctx, "realmtest@cas.com", 1800).await;
    grant_realm_admin_role(ctx, &admin_user_id).await;

    let app = ctx.create_unified_test_router();

    // Test 1: Create realm with auto-generated UUID v7
    let payload = json!(CreateRealmValidator {
        id: None,
        name: "Test Realm Auto".to_string(),
        admin_user: InitialAdminUserValidator {
            email: "admin@test.com".to_string(),
            password: "password123".to_string(),
        },
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let realm: serde_json::Value = response_json(resp).await;
    assert_eq!(realm["name"], "Test Realm Auto");
    assert!(!realm["id"].as_str().unwrap().is_empty());
    assert!(realm["meta"].is_null());

    // Test 2: Create realm with custom ID
    let payload = json!(CreateRealmValidator {
        id: Some("customrealm".to_string()),
        name: "Custom Realm".to_string(),
        admin_user: InitialAdminUserValidator {
            email: "admin2@test.com".to_string(),
            password: "password123".to_string(),
        },
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let realm: serde_json::Value = response_json(resp).await;
    assert_eq!(realm["id"], "customrealm");
    assert_eq!(realm["name"], "Custom Realm");
}

#[test_context(RealmTestContext)]
#[tokio::test]
async fn test_list_realms(ctx: &mut RealmTestContext) {
    // Create admin user and grant realm-admin role
    let (admin_token, admin_user_id) =
        create_admin_session_with_user(ctx, "listrealms@cas.com", 1800).await;
    grant_realm_admin_role(ctx, &admin_user_id).await;

    let app = ctx.create_unified_test_router();

    // Create a test realm first
    let payload = json!(CreateRealmValidator {
        id: Some("listtest".to_string()),
        name: "List Test Realm".to_string(),
        admin_user: InitialAdminUserValidator {
            email: "listadmin@test.com".to_string(),
            password: "password123".to_string(),
        },
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // List all realms
    let req = Request::builder()
        .method("GET")
        .uri("/api/realms")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let response: serde_json::Value = response_json(resp).await;
    let realms = response["realms"].as_array().unwrap();
    assert!(!realms.is_empty());
    assert!(realms.iter().any(|r| r["id"] == "listtest"));
}

#[test_context(RealmTestContext)]
#[tokio::test]
async fn test_update_realm(ctx: &mut RealmTestContext) {
    // Create admin user and grant realm-admin role
    let (admin_token, admin_user_id) =
        create_admin_session_with_user(ctx, "updaterealm@cas.com", 1800).await;
    grant_realm_admin_role(ctx, &admin_user_id).await;

    let app = ctx.create_unified_test_router();

    // Create a test realm first
    let payload = json!(CreateRealmValidator {
        id: Some("updatetest".to_string()),
        name: "Original Name".to_string(),
        admin_user: InitialAdminUserValidator {
            email: "updateadmin@test.com".to_string(),
            password: "password123".to_string(),
        },
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let update_payload = json!(UpdateRealmValidator {
        name: "Updated Name".to_string(),
    });

    let req = Request::builder()
        .method("PUT")
        .uri("/api/realms/updatetest")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .header("content-type", "application/json")
        .body(Body::from(update_payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let realm: serde_json::Value = response_json(resp).await;
    assert_eq!(realm["id"], "updatetest");
    assert_eq!(realm["name"], "Updated Name");
}

#[test_context(RealmTestContext)]
#[tokio::test]
async fn test_create_realm_duplicate_id_fails(ctx: &mut RealmTestContext) {
    // Create admin user and grant realm-admin role
    let (admin_token, admin_user_id) =
        create_admin_session_with_user(ctx, "duplicate@cas.com", 1800).await;
    grant_realm_admin_role(ctx, &admin_user_id).await;

    let app = ctx.create_unified_test_router();

    // Create first realm
    let payload = json!(CreateRealmValidator {
        id: Some("duplicate".to_string()),
        name: "First Realm".to_string(),
        admin_user: InitialAdminUserValidator {
            email: "dupadmin1@test.com".to_string(),
            password: "password123".to_string(),
        },
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Try to create realm with same ID
    let payload = json!(CreateRealmValidator {
        id: Some("duplicate".to_string()),
        name: "Second Realm".to_string(),
        admin_user: InitialAdminUserValidator {
            email: "dupadmin2@test.com".to_string(),
            password: "password123".to_string(),
        },
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let error: serde_json::Value = response_json(resp).await;
    assert_eq!(error["code"], 400);
    assert!(error["message"].as_str().is_some());
}

#[test_context(RealmTestContext)]
#[tokio::test]
async fn test_delete_realm_not_supported(ctx: &mut RealmTestContext) {
    // Create admin user and grant realm-admin role
    let (admin_token, admin_user_id) =
        create_admin_session_with_user(ctx, "delete@cas.com", 1800).await;
    grant_realm_admin_role(ctx, &admin_user_id).await;

    let app = ctx.create_unified_test_router();

    // Create a test realm
    let payload = json!(CreateRealmValidator {
        id: Some("deletetest".to_string()),
        name: "Delete Test Realm".to_string(),
        admin_user: InitialAdminUserValidator {
            email: "deleteadmin@test.com".to_string(),
            password: "password123".to_string(),
        },
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/realms")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Attempt to delete (should fail with Method Not Allowed)
    let req = Request::builder()
        .method("DELETE")
        .uri("/api/realms/deletetest")
        .header(header::COOKIE, format!("X-Auth={}", admin_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
}
