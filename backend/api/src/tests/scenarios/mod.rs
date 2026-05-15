// Audit log query scenarios
pub mod audit_scenarios;

// Audit event collection scenarios (verify events are recorded for core operations)
pub mod audit_collection_scenarios;

// Tests for self-implemented permission system
pub mod admin_init_scenarios;
pub mod admin_role_definitions_scenarios;
pub mod builtin_protection_scenarios;
pub mod change_email_scenarios;
pub mod client_app_scenarios;
pub mod login_flow_scenarios;
pub mod permission_regression_scenarios;
pub mod permission_security_scenarios;
pub mod realm_access_scenarios;
pub mod realm_admin_creation_scenarios;
pub mod realm_isolation_scenarios;
pub mod realm_totp_config_scenarios;
pub mod role_policies_scenarios;
pub mod user_list_scenarios;
pub mod user_register_test;
pub mod user_roles_scenarios;

// Billing scenarios
pub mod plan_scenarios;
pub mod shopify_webhook_scenarios;
pub mod subscription_scenarios;

// Billing security tests (permission checks + webhook verification)
pub mod billing;

// Realm creation permission scenarios
pub mod realm_creation_permission_test;

// Client API scenarios
pub mod client_api_scenarios;

// TOTP scenarios
pub mod realm_totp_key_initialization_scenarios;
pub mod user_totp_disable_scenarios;
pub mod user_totp_scenarios;

// Public config scenarios
pub mod public_config_scenarios;

// Unified OAuth scenarios
pub mod unified_oauth_scenarios;

// Device code authorization scenarios
pub mod device_code_scenarios;

// Unified permission hierarchy scenarios
pub mod unified_permission_hierarchy_scenarios;

// Points system scenarios
pub mod points;

// Realm management scenarios
// (realm_config_update_scenarios and realm_delete_scenarios removed - see .ai/future/backend_test_delete.md)
