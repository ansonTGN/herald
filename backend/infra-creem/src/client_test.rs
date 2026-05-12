// =============================================================================
// CreemClient Unit Tests
// =============================================================================
//
// Unit tests for Creem API client using wiremock for HTTP mocking
//
// =============================================================================

use super::*;
use herald_domain::common::entities::app_errors::CoreError;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

/// Helper function to create a test client
fn create_test_client(mock_server: &MockServer) -> CreemClient {
    CreemClient {
        http: reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("Failed to create test HTTP client"),
        api_key: "test_api_key".to_string(),
        base_url: mock_server.uri(),
    }
}

/// Test successful checkout session creation
#[tokio::test]
async fn test_unit_create_checkout_session_success() {
    // Arrange
    let mock_server = MockServer::start().await;
    let client = create_test_client(&mock_server);

    Mock::given(method("POST"))
        .and(path("/v1/checkout"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chk_test123",
            "url": "https://checkout.test.creem.io/abc123",
            "status": "pending"
        })))
        .mount(&mock_server)
        .await;

    let request = CreateCheckoutRequest {
        product_id: "prod_starter_monthly".to_string(),
        success_url: "https://example.com/success".to_string(),
        cancel_url: "https://example.com/cancel".to_string(),
        customer_email: "test@example.com".to_string(),
        metadata: None,
        webhook_url: None,
    };

    // Act
    let result = client.create_checkout_session(&request).await;

    // Assert
    assert!(result.is_ok());
    let session = result.unwrap();
    assert_eq!(session.id, "chk_test123");
    assert_eq!(session.url, "https://checkout.test.creem.io/abc123");
    assert_eq!(session.status, "pending");
}

/// Test checkout session with metadata
#[tokio::test]
async fn test_unit_create_checkout_session_with_metadata() {
    // Arrange
    let mock_server = MockServer::start().await;
    let client = create_test_client(&mock_server);

    Mock::given(method("POST"))
        .and(path("/v1/checkout"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chk_metadata",
            "url": "https://checkout.test.creem.io/metadata",
            "status": "pending"
        })))
        .mount(&mock_server)
        .await;

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("realm_id".to_string(), "realm_test123".to_string());
    metadata.insert("user_id".to_string(), "user_456".to_string());

    let request = CreateCheckoutRequest {
        product_id: "prod_test".to_string(),
        success_url: "https://example.com/success".to_string(),
        cancel_url: "https://example.com/cancel".to_string(),
        customer_email: "admin@example.com".to_string(),
        metadata: Some(metadata),
        webhook_url: None,
    };

    // Act
    let result = client.create_checkout_session(&request).await;

    // Assert
    assert!(result.is_ok());
    let session = result.unwrap();
    assert_eq!(session.id, "chk_metadata");
}

/// Test API authentication failure (401)
#[tokio::test]
async fn test_unit_create_checkout_session_unauthorized() {
    // Arrange
    let mock_server = MockServer::start().await;
    let client = CreemClient {
        http: reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("Failed to create test HTTP client"),
        api_key: "invalid_key".to_string(),
        base_url: mock_server.uri(),
    };

    Mock::given(method("POST"))
        .and(path("/v1/checkout"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": "Unauthorized",
            "message": "Invalid API key"
        })))
        .mount(&mock_server)
        .await;

    let request = CreateCheckoutRequest {
        product_id: "prod_test".to_string(),
        success_url: "https://example.com/success".to_string(),
        cancel_url: "https://example.com/cancel".to_string(),
        customer_email: "test@example.com".to_string(),
        metadata: None,
        webhook_url: None,
    };

    // Act
    let result = client.create_checkout_session(&request).await;

    // Assert
    assert!(result.is_err());
    match result.unwrap_err() {
        CoreError::InternalServerError(msg) => {
            assert!(msg.contains("401"));
            assert!(msg.contains("Unauthorized"));
        }
        _ => panic!("Expected InternalServerError for 401 response"),
    }
}

/// Test bad request (400)
#[tokio::test]
async fn test_unit_create_checkout_session_bad_request() {
    // Arrange
    let mock_server = MockServer::start().await;
    let client = create_test_client(&mock_server);

    Mock::given(method("POST"))
        .and(path("/v1/checkout"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "Bad Request",
            "message": "Invalid product_id"
        })))
        .mount(&mock_server)
        .await;

    let request = CreateCheckoutRequest {
        product_id: "invalid_product".to_string(),
        success_url: "https://example.com/success".to_string(),
        cancel_url: "https://example.com/cancel".to_string(),
        customer_email: "test@example.com".to_string(),
        metadata: None,
        webhook_url: None,
    };

    // Act
    let result = client.create_checkout_session(&request).await;

    // Assert
    assert!(result.is_err());
    match result.unwrap_err() {
        CoreError::InternalServerError(msg) => {
            assert!(msg.contains("400"), "actual msg: {msg}");
        }
        _ => panic!("Expected InternalServerError for 400 response"),
    }
}

/// Test server error (500)
#[tokio::test]
async fn test_unit_create_checkout_session_server_error() {
    // Arrange
    let mock_server = MockServer::start().await;
    let client = create_test_client(&mock_server);

    Mock::given(method("POST"))
        .and(path("/v1/checkout"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "Internal Server Error"
        })))
        .mount(&mock_server)
        .await;

    let request = CreateCheckoutRequest {
        product_id: "prod_test".to_string(),
        success_url: "https://example.com/success".to_string(),
        cancel_url: "https://example.com/cancel".to_string(),
        customer_email: "test@example.com".to_string(),
        metadata: None,
        webhook_url: None,
    };

    // Act
    let result = client.create_checkout_session(&request).await;

    // Assert
    assert!(result.is_err());
    match result.unwrap_err() {
        CoreError::InternalServerError(msg) => {
            assert!(msg.contains("500"));
        }
        _ => panic!("Expected InternalServerError for 500 response"),
    }
}

/// Test invalid JSON response
#[tokio::test]
async fn test_unit_create_checkout_session_invalid_json() {
    // Arrange
    let mock_server = MockServer::start().await;
    let client = create_test_client(&mock_server);

    Mock::given(method("POST"))
        .and(path("/v1/checkout"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chk_test",
            "url": "https://checkout.test.creem.io/test",
            // Missing "status" field
        })))
        .mount(&mock_server)
        .await;

    let request = CreateCheckoutRequest {
        product_id: "prod_test".to_string(),
        success_url: "https://example.com/success".to_string(),
        cancel_url: "https://example.com/cancel".to_string(),
        customer_email: "test@example.com".to_string(),
        metadata: None,
        webhook_url: None,
    };

    // Act
    let result = client.create_checkout_session(&request).await;

    // Assert
    assert!(result.is_err());
    match result.unwrap_err() {
        CoreError::InternalServerError(msg) => {
            assert!(msg.contains("Invalid Creem response"), "actual msg: {msg}");
        }
        _ => panic!("Expected InternalServerError for invalid JSON"),
    }
}

// NOTE: Low-value test removed (test_unit_checkout_request_serialization)
// This test only verified serde standard functionality without custom logic.
// Serde guarantees are covered by the library itself and integration tests.

// NOTE: Low-value test removed (test_unit_checkout_request_metadata_serialization)
// This test only verified serde standard functionality without custom logic.
// Serde guarantees are covered by the library itself and integration tests.

// NOTE: Low-value test removed (test_unit_checkout_session_deserialization)
// This test only verified serde standard functionality without custom logic.
// Serde guarantees are covered by the library itself and integration tests.

/// Test multiple concurrent requests
#[tokio::test]
#[ignore] // TODO: Fix MockServer configuration for concurrent requests
async fn test_unit_concurrent_checkout_requests() {
    // Arrange
    let mock_server = MockServer::start().await;
    let client = create_test_client(&mock_server);

    let request = CreateCheckoutRequest {
        product_id: "prod_test".to_string(),
        success_url: "https://example.com/success".to_string(),
        cancel_url: "https://example.com/cancel".to_string(),
        customer_email: "test@example.com".to_string(),
        metadata: None,
        webhook_url: None,
    };

    // Create a mock that responds multiple times
    Mock::given(method("POST"))
        .and(path("/v1/checkout"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chk_concurrent",
            "url": "https://checkout.test.creem.io/concurrent",
            "status": "pending"
        })))
        .mount(&mock_server)
        .await;

    // Act: Send multiple concurrent requests
    let mut tasks = Vec::new();
    for _ in 0..5 {
        let client_clone = client.clone();
        let request_clone = request.clone();
        tasks.push(tokio::spawn(async move {
            client_clone.create_checkout_session(&request_clone).await
        }));
    }

    // Assert: All requests succeed
    for task in tasks {
        let result = task.await.unwrap();
        assert!(result.is_ok(), "Request failed: {:?}", result);
        let session = result.unwrap();
        assert_eq!(session.id, "chk_concurrent");
    }
}
