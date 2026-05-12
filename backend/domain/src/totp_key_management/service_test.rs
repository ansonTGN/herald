// =============================================================================
// Realm TOTP Key Service Tests
// =============================================================================
//
// 测试 Realm TOTP 密钥管理服务的业务逻辑
//
// **测试目标**：
// 1. 验证密钥初始化流程
// 2. 验证密钥生成正确性
// 3. 验证错误处理
// 4. 验证日志记录
//
// **测试类型**：单元测试（使用 Mock Repository）
//
// =============================================================================

use crate::common::entities::app_errors::CoreError;
use crate::totp_key_management::ports::{MockRealmTotpKeyRepository, RealmTotpKeyService};
use crate::totp_key_management::service::RealmTotpKeyServiceImpl;
use std::sync::Arc;

// ============================================================================
// Unit Tests: RealmTotpKeyServiceImpl
// ============================================================================

#[test]
fn test_unit_realm_totp_key_service_new() {
    let mock_repo = Arc::new(MockRealmTotpKeyRepository::new());
    let _service = RealmTotpKeyServiceImpl::new(mock_repo);

    // Service is created successfully
    // (Can't inspect internal field due to privacy, but we can verify it exists)
}

// ============================================================================
// Unit Tests: init_realm_key
// ============================================================================

#[tokio::test]
async fn test_unit_init_realm_key_success() {
    let mut mock_repo = MockRealmTotpKeyRepository::new();

    mock_repo
        .expect_get_active_key()
        .returning(|_| Box::pin(async { Ok(None) }));

    mock_repo
        .expect_create_key()
        .returning(|_realm_id, _key| Box::pin(async { Ok(()) }));

    let service = RealmTotpKeyServiceImpl::new(Arc::new(mock_repo));

    let result = service.init_realm_key("test-realm").await;

    assert!(result.is_ok(), "Should successfully initialize realm key");
}

// NOTE: Over-mocked tests removed (test_unit_init_realm_key_generates_correct_key_length,
// test_unit_init_realm_key_not_all_zeros)
//
// These tests verified implementation details (key length 32 bytes, non-zero) via mocks
// instead of testing actual business behavior. They have been replaced by scenario tests:
//
// - backend/api/src/tests/scenarios/realm_totp_key_initialization_scenarios.rs
//   - test_scenario_realm_totp_key_initialization: Verifies key generation and storage
//   - test_scenario_realm_totp_key_idempotency: Verifies idempotency
//
// The new scenario tests verify:
// - Key is generated and stored in database
// - Key is exactly 32 bytes (AES-256)
// - Key is not all zeros (randomly generated)
// - Key metadata is correct (version, enabled, secret)
//
#[tokio::test]
async fn test_unit_init_realm_key_handles_database_error() {
    let mut mock_repo = MockRealmTotpKeyRepository::new();

    mock_repo
        .expect_get_active_key()
        .returning(|_| Box::pin(async { Ok(None) }));

    mock_repo.expect_create_key().returning(|_realm_id, _key| {
        Box::pin(async { Err(CoreError::InternalServerError("Database error".to_string())) })
    });

    let service = RealmTotpKeyServiceImpl::new(Arc::new(mock_repo));

    let result = service.init_realm_key("test-realm").await;

    assert!(result.is_err(), "Should return error on database failure");
    if let Err(CoreError::InternalServerError(msg)) = result {
        assert!(
            msg.contains("Database error"),
            "Error message should indicate database error"
        );
    } else {
        panic!("Expected InternalServerError");
    }
}
