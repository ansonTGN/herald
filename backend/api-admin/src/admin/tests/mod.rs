// =============================================================================
// Admin 测试模块
// =============================================================================
//
// 使用 Schema 隔离的测试架构
// AdminTestContext 是 SchemaTestContext 的别名
//
// =============================================================================

// ⚠️ 已删除: HTTP Handler 单元测试和 RBAC 集成测试
// 删除原因: 改用场景测试（Scenario Tests）
//
// 删除的文件:
// - rbac_test.rs (RBAC 集成测试)
// - role_isolation_test.rs (角色隔离测试)
// - security_test.rs (安全测试)
//
// 替代的场景测试:
// - backend/api/src/tests/scenarios/realm_admin_scenarios.rs
// - backend/api/src/tests/scenarios/user_roles_scenarios.rs
// - backend/api/src/tests/scenarios/realm_isolation_scenarios.rs
// - backend/api/src/tests/scenarios/realm_access_scenarios.rs
// - backend/api/src/tests/scenarios/permission_system_basic_scenarios.rs

// 使用 Schema 隔离的 TestContext（每个测试独立数据库 Schema）
