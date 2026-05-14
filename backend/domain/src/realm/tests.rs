// =============================================================================
// Realm Admin Creation - Unit Tests
// =============================================================================
//
// 测试创建 Realm 时自动创建管理员的功能
//
// 测试覆盖:
// - 正常场景: 创建 Realm 并指定管理员
// - 边界场景: 不指定管理员
// - 回滚场景: 用户创建失败、角色分配失败
//
// =============================================================================

#[cfg(test)]
mod realm_admin_tests {

    use crate::audit::MockAuditEventRepository;
    use crate::authentication::Identity;
    use crate::authorization::{MockRoleRepository, MockUserRoleRepository};
    use crate::client::ports::MockClientRepository;
    use crate::common::entities::app_errors::CoreError;
    use crate::common::policies::{AllowAllRealmPolicy, RealmPolicy};
    use crate::rbac_init::MockRealmInitializationService;
    use crate::realm::{
        CreateRealmRequest, InitialAdminUser, MockRealmRepository, Realm, RealmService,
        services::RealmServiceImpl,
    };
    use crate::realm_config::ports::MockRealmConfigRepository;
    use crate::user::entities::{User, UserStatus};
    use crate::user::ports::{MockUserRepository, MockUserService};
    use chrono::Utc;
    use mockall::predicate::eq;
    use std::sync::Arc;

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    fn realm_fixture() -> Realm {
        Realm {
            id: "test-realm".to_string(),
            name: "Test Realm".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            admin_user: None,
        }
    }

    fn user_fixture() -> User {
        User {
            id: crate::common::entities::generate_uuid_v7(),
            realm_id: "test-realm".to_string(),
            email: "admin@test.com".to_string(),
            nickname: None,
            password_hash: Some("$2a$12$dummy_password_hash".to_string()),
            provider_ids: vec![],
            status: UserStatus::Normal,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn role_fixture() -> crate::authorization::Role {
        crate::authorization::Role {
            id: crate::common::entities::generate_uuid_v7(),
            realm_id: "test-realm".to_string(),
            client_id: "admin-web-console".to_string(),
            name: "realm-admin".to_string(),
            description: Some("Realm Administrator".to_string()),
            is_builtin: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn client_fixture() -> crate::client::entities::ClientApp {
        crate::client::entities::ClientApp {
            id: crate::common::entities::generate_uuid_v7(),
            realm_id: "test-realm".to_string(),
            client_id: "admin-web-console".to_string(),
            client_secret: Some("".to_string()),
            name: "Admin Web Console".to_string(),
            description: Some("Admin web console client application".to_string()),
            redirect_uris: vec![],
            enabled: true,
            icon_url: None,
            session_ttl_seconds: 1800,
            session_renewal_ttl_seconds: Some(300),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn identity_fixture() -> Identity {
        Identity::User(user_fixture())
    }

    fn policy_fixture() -> AllowAllRealmPolicy {
        AllowAllRealmPolicy
    }

    fn mock_audit_repo() -> MockAuditEventRepository {
        let mut mock = MockAuditEventRepository::new();
        mock.expect_create().returning(|_| {
            let event = crate::audit::AuditEvent {
                id: crate::common::entities::generate_uuid_v7(),
                realm_id: "test-realm".to_string(),
                category: crate::audit::AuditCategory::RealmManagement,
                action: crate::audit::AuditAction::RealmCreate,
                actor_id: "test".to_string(),
                actor_type: None,
                actor_name: None,
                target_type: crate::audit::AuditTargetType::Realm,
                target_id: "test-realm".to_string(),
                target_name: None,
                result: crate::audit::AuditResult::Success,
                details: None,
                ip_address: None,
                user_agent: None,
                trace_id: None,
                created_at: Utc::now(),
            };
            Box::pin(async move { Ok(event) })
        });
        mock
    }

    #[derive(Clone)]
    struct DenyAllRealmPolicy;

    impl RealmPolicy for DenyAllRealmPolicy {
        async fn can_create_realm(&self, _identity: Identity) -> bool {
            false
        }

        async fn can_read_realm(&self, _identity: Identity) -> bool {
            false
        }

        async fn can_update_realm(&self, _identity: Identity) -> bool {
            false
        }

        async fn can_delete_realm(&self, _identity: Identity) -> bool {
            false
        }

        async fn can_list_realms(&self, _identity: Identity) -> bool {
            false
        }
    }

    // =========================================================================
    // Scenario 1: Normal - Create Realm with Admin User
    // =========================================================================

    #[tokio::test]
    async fn test_unit_create_realm_with_admin_user_success() {
        // =====================================================================
        // Given: Mock repositories return successful responses
        // =====================================================================
        let mut mock_realm_repo = MockRealmRepository::new();
        let mut mock_user_service = MockUserService::new();
        let mut mock_role_repo = MockRoleRepository::new();
        let mut mock_user_role_repo = MockUserRoleRepository::new();
        let mut mock_client_repo = MockClientRepository::new();
        let mut mock_rbac_init = MockRealmInitializationService::new();
        let mock_policy = policy_fixture();

        // Set up expectations
        mock_realm_repo
            .expect_create_realm()
            .returning(|_| Box::pin(async { Ok(realm_fixture()) }));

        mock_client_repo
            .expect_get_client_app_by_client_id()
            .returning(|_, _| Box::pin(async { Err(CoreError::NotFound) }));

        mock_client_repo
            .expect_create_client_app()
            .returning(|_| Box::pin(async { Ok(client_fixture()) }));

        mock_rbac_init
            .expect_init_default_rbac()
            .returning(|_, _| Box::pin(async { Ok(()) }));

        mock_user_service
            .expect_create_user_without_identity_check()
            .returning(|_| Box::pin(async { Ok(user_fixture()) }));

        mock_role_repo
            .expect_find_by_name()
            .returning(|_, _, _| Box::pin(async { Ok(role_fixture()) }));

        mock_user_role_repo
            .expect_assign_role_to_user()
            .returning(|_| Box::pin(async { Ok(()) }));

        let mut mock_user_repo = MockUserRepository::new();
        mock_user_repo
            .expect_update_user()
            .returning(|_, _| Box::pin(async { Ok(user_fixture()) }));

        let mut mock_realm_config_repo = MockRealmConfigRepository::new();
        mock_realm_config_repo.expect_upsert().returning(|_, _| {
            Box::pin(async {
                Ok(crate::realm_config::RealmConfig {
                    id: crate::common::entities::generate_uuid_v7(),
                    realm_id: "test-realm".to_string(),
                    config_type: crate::realm_config::ConfigType::Registration,
                    config_key: "enabled".to_string(),
                    config_value: "false".to_string(),
                    is_secret: false,
                    enabled: true,
                    metadata: Some(serde_json::json!({})),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            })
        });
        mock_realm_config_repo
            .expect_batch_upsert()
            .returning(|_, _| Box::pin(async { Ok(vec![]) }));

        let realm_service = Arc::new(RealmServiceImpl::new(
            Arc::new(mock_realm_repo),
            Arc::new(mock_policy),
            Arc::new(mock_rbac_init),
            Arc::new(mock_client_repo),
            Arc::new(mock_user_role_repo), // Use the configured mock
            Arc::new(mock_role_repo),
            Arc::new(mock_user_repo),
            Arc::new(mock_user_service),
            Arc::new(mock_realm_config_repo),
            Arc::new(mock_audit_repo()),
        ));

        // =====================================================================
        // When: Create realm with admin user
        // =====================================================================
        let result = realm_service
            .create_realm(
                identity_fixture(),
                CreateRealmRequest {
                    id: Some("test-realm".to_string()),
                    name: "Test Realm".to_string(),
                    admin_user: InitialAdminUser {
                        email: "admin@test.com".to_string(),
                        password: "password123".to_string(),
                    },
                },
            )
            .await;

        // =====================================================================
        // Then: Realm creation succeeds
        // =====================================================================
        assert!(result.is_ok());
        let realm = result.unwrap();
        assert_eq!(realm.id, "test-realm");
        assert_eq!(realm.name, "Test Realm");
    }

    // =========================================================================
    // Scenario 2: Rollback - User Creation Failure
    // =========================================================================

    #[tokio::test]
    async fn test_unit_create_realm_rollback_on_user_creation_failure() {
        // =====================================================================
        // Given: User creation fails
        // =====================================================================
        let mut mock_realm_repo = MockRealmRepository::new();
        let mut mock_user_service = MockUserService::new();
        let mut mock_client_repo = MockClientRepository::new();
        let mut mock_rbac_init = MockRealmInitializationService::new();
        let mock_policy = policy_fixture();

        mock_realm_repo
            .expect_create_realm()
            .returning(|_| Box::pin(async { Ok(realm_fixture()) }));

        mock_client_repo
            .expect_get_client_app_by_client_id()
            .returning(|_, _| Box::pin(async { Err(CoreError::NotFound) }));

        mock_client_repo
            .expect_create_client_app()
            .returning(|_| Box::pin(async { Ok(client_fixture()) }));

        mock_rbac_init
            .expect_init_default_rbac()
            .returning(|_, _| Box::pin(async { Ok(()) }));

        // User creation fails
        mock_user_service
            .expect_create_user_without_identity_check()
            .returning(|_| {
                Box::pin(async { Err(CoreError::BadRequest("Email already exists".to_string())) })
            });

        // Expect rollback
        mock_realm_repo
            .expect_delete_realm()
            .with(eq("test-realm"))
            .returning(|_| Box::pin(async { Ok(()) }))
            .times(1);

        let mut mock_realm_config_repo = MockRealmConfigRepository::new();
        mock_realm_config_repo.expect_upsert().returning(|_, _| {
            Box::pin(async {
                Ok(crate::realm_config::RealmConfig {
                    id: crate::common::entities::generate_uuid_v7(),
                    realm_id: "test-realm".to_string(),
                    config_type: crate::realm_config::ConfigType::Registration,
                    config_key: "enabled".to_string(),
                    config_value: "false".to_string(),
                    is_secret: false,
                    enabled: true,
                    metadata: Some(serde_json::json!({})),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            })
        });
        mock_realm_config_repo
            .expect_batch_upsert()
            .returning(|_, _| Box::pin(async { Ok(vec![]) }));

        let realm_service = Arc::new(RealmServiceImpl::new(
            Arc::new(mock_realm_repo),
            Arc::new(mock_policy),
            Arc::new(mock_rbac_init),
            Arc::new(mock_client_repo),
            Arc::new(MockUserRoleRepository::new()),
            Arc::new(MockRoleRepository::new()),
            Arc::new(MockUserRepository::new()),
            Arc::new(mock_user_service),
            Arc::new(mock_realm_config_repo),
            Arc::new(mock_audit_repo()),
        ));

        // =====================================================================
        // When: Create realm with admin user, but user creation fails
        // =====================================================================
        let result = realm_service
            .create_realm(
                identity_fixture(),
                CreateRealmRequest {
                    id: Some("test-realm".to_string()),
                    name: "Test Realm".to_string(),
                    admin_user: InitialAdminUser {
                        email: "admin@test.com".to_string(),
                        password: "password123".to_string(),
                    },
                },
            )
            .await;

        // =====================================================================
        // Then: Realm creation fails, rollback is called, original error is returned
        // =====================================================================
        assert!(result.is_err());
        // The error should be the original BadRequest error, not wrapped in InternalServerError
        match result {
            Err(CoreError::BadRequest(msg)) => {
                assert!(msg.contains("Email already exists"));
            }
            _ => panic!("Expected BadRequest error to be propagated"),
        }
    }

    // =========================================================================
    // Scenario 3: Rollback - Role Not Found
    // =========================================================================

    #[tokio::test]
    async fn test_unit_create_realm_rollback_on_role_not_found() {
        // =====================================================================
        // Given: realm-admin role not found (abnormal case)
        // =====================================================================
        let mut mock_realm_repo = MockRealmRepository::new();
        let mut mock_user_service = MockUserService::new();
        let mut mock_role_repo = MockRoleRepository::new();
        let mock_user_role_repo = MockUserRoleRepository::new();
        let mut mock_client_repo = MockClientRepository::new();
        let mut mock_rbac_init = MockRealmInitializationService::new();
        let mock_policy = policy_fixture();

        mock_realm_repo
            .expect_create_realm()
            .returning(|_| Box::pin(async { Ok(realm_fixture()) }));

        mock_client_repo
            .expect_get_client_app_by_client_id()
            .returning(|_, _| Box::pin(async { Err(CoreError::NotFound) }));

        mock_client_repo
            .expect_create_client_app()
            .returning(|_| Box::pin(async { Ok(client_fixture()) }));

        mock_rbac_init
            .expect_init_default_rbac()
            .returning(|_, _| Box::pin(async { Ok(()) }));

        mock_user_service
            .expect_create_user_without_identity_check()
            .returning(|_| Box::pin(async { Ok(user_fixture()) }));

        // Role not found
        mock_role_repo
            .expect_find_by_name()
            .returning(|_, _, _| Box::pin(async { Err(CoreError::NotFound) }));

        // Expect rollback
        mock_realm_repo
            .expect_delete_realm()
            .with(eq("test-realm"))
            .returning(|_| Box::pin(async { Ok(()) }))
            .times(1);

        let mut mock_user_repo = MockUserRepository::new();
        mock_user_repo
            .expect_update_user()
            .returning(|_, _| Box::pin(async { Ok(user_fixture()) }));

        let mut mock_realm_config_repo = MockRealmConfigRepository::new();
        mock_realm_config_repo.expect_upsert().returning(|_, _| {
            Box::pin(async {
                Ok(crate::realm_config::RealmConfig {
                    id: crate::common::entities::generate_uuid_v7(),
                    realm_id: "test-realm".to_string(),
                    config_type: crate::realm_config::ConfigType::Registration,
                    config_key: "enabled".to_string(),
                    config_value: "false".to_string(),
                    is_secret: false,
                    enabled: true,
                    metadata: Some(serde_json::json!({})),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            })
        });
        mock_realm_config_repo
            .expect_batch_upsert()
            .returning(|_, _| Box::pin(async { Ok(vec![]) }));

        let realm_service = Arc::new(RealmServiceImpl::new(
            Arc::new(mock_realm_repo),
            Arc::new(mock_policy),
            Arc::new(mock_rbac_init),
            Arc::new(mock_client_repo),
            Arc::new(mock_user_role_repo),
            Arc::new(mock_role_repo),
            Arc::new(mock_user_repo),
            Arc::new(mock_user_service),
            Arc::new(mock_realm_config_repo),
            Arc::new(mock_audit_repo()),
        ));

        // =====================================================================
        // When: Create realm with admin user, but role is not found
        // =====================================================================
        let result = realm_service
            .create_realm(
                identity_fixture(),
                CreateRealmRequest {
                    id: Some("test-realm".to_string()),
                    name: "Test Realm".to_string(),
                    admin_user: InitialAdminUser {
                        email: "admin@test.com".to_string(),
                        password: "password123".to_string(),
                    },
                },
            )
            .await;

        // =====================================================================
        // Then: Realm creation fails, rollback is called
        // =====================================================================
        assert!(result.is_err());
        match result {
            Err(CoreError::InternalServerError(msg)) => {
                assert!(msg.contains("Failed to find realm-admin role"));
            }
            _ => panic!("Expected InternalServerError"),
        }
    }

    #[tokio::test]
    async fn test_unit_get_realm_allows_reading_own_realm_without_global_read_permission() {
        let mut mock_realm_repo = MockRealmRepository::new();
        mock_realm_repo
            .expect_get_realm_by_id()
            .with(eq("test-realm"))
            .returning(|_| Box::pin(async { Ok(realm_fixture()) }));

        let realm_service = Arc::new(RealmServiceImpl::new(
            Arc::new(mock_realm_repo),
            Arc::new(DenyAllRealmPolicy),
            Arc::new(MockRealmInitializationService::new()),
            Arc::new(MockClientRepository::new()),
            Arc::new(MockUserRoleRepository::new()),
            Arc::new(MockRoleRepository::new()),
            Arc::new(MockUserRepository::new()),
            Arc::new(MockUserService::new()),
            Arc::new(MockRealmConfigRepository::new()),
            Arc::new(mock_audit_repo()),
        ));

        let result = realm_service
            .get_realm(identity_fixture(), "test-realm".to_string())
            .await;

        assert!(result.is_ok(), "own realm should remain readable");
    }

    #[tokio::test]
    async fn test_unit_get_realm_denies_cross_realm_read_without_global_permission() {
        let realm_service = Arc::new(RealmServiceImpl::new(
            Arc::new(MockRealmRepository::new()),
            Arc::new(DenyAllRealmPolicy),
            Arc::new(MockRealmInitializationService::new()),
            Arc::new(MockClientRepository::new()),
            Arc::new(MockUserRoleRepository::new()),
            Arc::new(MockRoleRepository::new()),
            Arc::new(MockUserRepository::new()),
            Arc::new(MockUserService::new()),
            Arc::new(MockRealmConfigRepository::new()),
            Arc::new(mock_audit_repo()),
        ));

        let result = realm_service
            .get_realm(identity_fixture(), "other-realm".to_string())
            .await;

        match result {
            Err(CoreError::Forbidden(msg)) => {
                assert!(msg.contains("read realm"));
            }
            _ => panic!("expected forbidden for cross-realm read"),
        }
    }

    #[tokio::test]
    async fn test_unit_list_realms_requires_global_permission() {
        let realm_service = Arc::new(RealmServiceImpl::new(
            Arc::new(MockRealmRepository::new()),
            Arc::new(DenyAllRealmPolicy),
            Arc::new(MockRealmInitializationService::new()),
            Arc::new(MockClientRepository::new()),
            Arc::new(MockUserRoleRepository::new()),
            Arc::new(MockRoleRepository::new()),
            Arc::new(MockUserRepository::new()),
            Arc::new(MockUserService::new()),
            Arc::new(MockRealmConfigRepository::new()),
            Arc::new(mock_audit_repo()),
        ));

        let result = realm_service.list_realms(identity_fixture()).await;

        match result {
            Err(CoreError::Forbidden(msg)) => {
                assert!(msg.contains("list realms"));
            }
            _ => panic!("expected forbidden for realm listing"),
        }
    }
}
