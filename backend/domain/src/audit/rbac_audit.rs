// =============================================================================
// RBAC 审计日志模块
// =============================================================================
//
// 记录所有角色和权限的变更操作，用于审计和安全追溯
//
// =============================================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// RBAC 审计事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacAuditEvent {
    /// 操作类型
    pub action: RbacAction,
    /// 执行操作的用户 ID
    pub actor_id: String,
    /// 操作目标的类型
    pub target_type: RbacTargetType,
    /// 操作目标的 ID
    pub target_id: String,
    /// 操作目标的名称（用于更易读的审计日志）
    pub target_name: Option<String>,
    /// Realm ID
    pub realm_id: String,
    /// 操作详情（JSON 格式）
    pub details: serde_json::Value,
    /// 操作结果
    pub result: RbacResult,
    /// 操作时间戳
    pub timestamp: DateTime<Utc>,
    /// IP 地址（可选）
    pub ip_address: Option<String>,
    /// User Agent（可选）
    pub user_agent: Option<String>,
}

/// RBAC 操作类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RbacAction {
    // 角色操作
    CreateRole,
    UpdateRole,
    DeleteRole,
    AssignRole,
    RevokeRole,

    // 权限操作
    CreatePermission,
    UpdatePermission,
    DeletePermission,
    GrantPermission,
    RevokePermission,

    // 策略操作
    CreatePolicy,
    DeletePolicy,

    // 初始化操作
    InitializeRbac,
    ReinitializeRbac,
}

/// RBAC 目标类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RbacTargetType {
    Role,
    Permission,
    Policy,
    User,
    Realm,
}

/// RBAC 操作结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RbacResult {
    Success,
    Failed,
    PartialSuccess,
}

/// 审计日志记录器
pub struct RbacAuditLogger {
    /// 是否启用审计日志
    enabled: bool,
}

impl RbacAuditLogger {
    /// 创建新的审计日志记录器
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// 记录 RBAC 操作
    pub fn log(&self, event: RbacAuditEvent) {
        if !self.enabled {
            return;
        }

        match event.result {
            RbacResult::Success => {
                info!(
                    action = %format!("{:?}", event.action),
                    actor_id = %event.actor_id,
                    target_type = %format!("{:?}", event.target_type),
                    target_id = %event.target_id,
                    target_name = ?event.target_name,
                    realm_id = %event.realm_id,
                    timestamp = %event.timestamp,
                    "RBAC audit: {} {} by user {}",
                    format!("{:?}", event.action),
                    event.target_name.as_ref().unwrap_or(&event.target_id),
                    event.actor_id
                );
            }
            RbacResult::Failed | RbacResult::PartialSuccess => {
                warn!(
                    action = %format!("{:?}", event.action),
                    actor_id = %event.actor_id,
                    target_type = %format!("{:?}", event.target_type),
                    target_id = %event.target_id,
                    target_name = ?event.target_name,
                    realm_id = %event.realm_id,
                    result = %format!("{:?}", event.result),
                    timestamp = %event.timestamp,
                    error_details = ?event.details,
                    "RBAC audit: {} {} {} by user {}",
                    format!("{:?}", event.result),
                    format!("{:?}", event.action),
                    event.target_name.as_ref().unwrap_or(&event.target_id),
                    event.actor_id
                );
            }
        }
    }

    /// 记录角色创建
    pub fn log_create_role(
        &self,
        actor_id: String,
        realm_id: String,
        role_id: String,
        role_name: String,
    ) {
        self.log(RbacAuditEvent {
            action: RbacAction::CreateRole,
            actor_id,
            target_type: RbacTargetType::Role,
            target_id: role_id.clone(),
            target_name: Some(role_name.clone()),
            realm_id,
            details: serde_json::json!({ "role_name": role_name }),
            result: RbacResult::Success,
            timestamp: Utc::now(),
            ip_address: None,
            user_agent: None,
        });
    }

    /// 记录角色删除（包括内置角色保护）
    pub fn log_delete_role(
        &self,
        actor_id: String,
        realm_id: String,
        role_id: String,
        role_name: String,
        is_builtin: bool,
        result: RbacResult,
    ) {
        let details = if is_builtin {
            serde_json::json!({ "role_name": role_name, "is_builtin": true, "reason": "Built-in role cannot be deleted" })
        } else {
            serde_json::json!({ "role_name": role_name, "is_builtin": false })
        };

        self.log(RbacAuditEvent {
            action: RbacAction::DeleteRole,
            actor_id,
            target_type: RbacTargetType::Role,
            target_id: role_id.clone(),
            target_name: Some(role_name.clone()),
            realm_id,
            details,
            result,
            timestamp: Utc::now(),
            ip_address: None,
            user_agent: None,
        });
    }

    /// 记录角色更新（包括内置角色名称保护）
    pub fn log_update_role(
        &self,
        actor_id: String,
        realm_id: String,
        role_id: String,
        old_name: String,
        new_name: String,
        is_builtin: bool,
        result: RbacResult,
    ) {
        let details = if is_builtin && old_name != new_name {
            serde_json::json!({
                "old_name": old_name,
                "new_name": new_name,
                "is_builtin": true,
                "reason": "Built-in role name cannot be changed"
            })
        } else {
            serde_json::json!({
                "old_name": old_name,
                "new_name": new_name,
                "is_builtin": is_builtin
            })
        };

        self.log(RbacAuditEvent {
            action: RbacAction::UpdateRole,
            actor_id,
            target_type: RbacTargetType::Role,
            target_id: role_id.clone(),
            target_name: Some(old_name.clone()),
            realm_id,
            details,
            result,
            timestamp: Utc::now(),
            ip_address: None,
            user_agent: None,
        });
    }

    /// 记录权限创建
    pub fn log_create_permission(
        &self,
        actor_id: String,
        realm_id: String,
        permission_id: String,
        permission_name: String,
    ) {
        self.log(RbacAuditEvent {
            action: RbacAction::CreatePermission,
            actor_id,
            target_type: RbacTargetType::Permission,
            target_id: permission_id.clone(),
            target_name: Some(permission_name.clone()),
            realm_id,
            details: serde_json::json!({ "permission_name": permission_name }),
            result: RbacResult::Success,
            timestamp: Utc::now(),
            ip_address: None,
            user_agent: None,
        });
    }

    /// 记录权限删除
    pub fn log_delete_permission(
        &self,
        actor_id: String,
        realm_id: String,
        permission_id: String,
        permission_name: String,
        is_builtin: bool,
        result: RbacResult,
    ) {
        let details = if is_builtin {
            serde_json::json!({
                "permission_name": permission_name,
                "is_builtin": true,
                "reason": "Built-in permission cannot be deleted"
            })
        } else {
            serde_json::json!({
                "permission_name": permission_name,
                "is_builtin": false
            })
        };

        self.log(RbacAuditEvent {
            action: RbacAction::DeletePermission,
            actor_id,
            target_type: RbacTargetType::Permission,
            target_id: permission_id.clone(),
            target_name: Some(permission_name.clone()),
            realm_id,
            details,
            result,
            timestamp: Utc::now(),
            ip_address: None,
            user_agent: None,
        });
    }

    /// 记录 RBAC 初始化
    pub fn log_rbac_init(
        &self,
        actor_id: String,
        realm_id: String,
        result: RbacResult,
        details: serde_json::Value,
    ) {
        self.log(RbacAuditEvent {
            action: RbacAction::InitializeRbac,
            actor_id,
            target_type: RbacTargetType::Realm,
            target_id: realm_id.clone(),
            target_name: Some(realm_id.clone()),
            realm_id: realm_id.clone(),
            details,
            result,
            timestamp: Utc::now(),
            ip_address: None,
            user_agent: None,
        });
    }

    /// 记录权限授予
    pub fn log_grant_permission(
        &self,
        actor_id: String,
        realm_id: String,
        role_id: String,
        role_name: String,
        permission_id: String,
        permission_name: String,
    ) {
        self.log(RbacAuditEvent {
            action: RbacAction::GrantPermission,
            actor_id,
            target_type: RbacTargetType::Role,
            target_id: role_id.clone(),
            target_name: Some(role_name.clone()),
            realm_id,
            details: serde_json::json!({
                "permission_id": permission_id,
                "permission_name": permission_name
            }),
            result: RbacResult::Success,
            timestamp: Utc::now(),
            ip_address: None,
            user_agent: None,
        });
    }

    /// 记录权限撤销
    pub fn log_revoke_permission(
        &self,
        actor_id: String,
        realm_id: String,
        role_id: String,
        role_name: String,
        permission_id: String,
        permission_name: String,
    ) {
        self.log(RbacAuditEvent {
            action: RbacAction::RevokePermission,
            actor_id,
            target_type: RbacTargetType::Role,
            target_id: role_id.clone(),
            target_name: Some(role_name.clone()),
            realm_id,
            details: serde_json::json!({
                "permission_id": permission_id,
                "permission_name": permission_name
            }),
            result: RbacResult::Success,
            timestamp: Utc::now(),
            ip_address: None,
            user_agent: None,
        });
    }
}

impl Default for RbacAuditLogger {
    fn default() -> Self {
        Self::_new_from_env()
    }
}

impl RbacAuditLogger {
    /// 从环境变量创建审计日志记录器
    fn _new_from_env() -> Self {
        // 默认启用审计日志
        let enabled = std::env::var("RBAC_AUDIT_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        Self::new(enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_serialization() {
        let event = RbacAuditEvent {
            action: RbacAction::CreateRole,
            actor_id: "user-123".to_string(),
            target_type: RbacTargetType::Role,
            target_id: "role-456".to_string(),
            target_name: Some("admin".to_string()),
            realm_id: "my-realm".to_string(),
            details: serde_json::json!({ "role_name": "admin" }),
            result: RbacResult::Success,
            timestamp: Utc::now(),
            ip_address: None,
            user_agent: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("create_role"));
        assert!(json.contains("user-123"));
        assert!(json.contains("admin"));
    }

    #[test]
    fn test_audit_logger_logs_event() {
        let logger = RbacAuditLogger::new(true);
        logger.log_create_role(
            "actor-1".to_string(),
            "realm-1".to_string(),
            "role-1".to_string(),
            "test-role".to_string(),
        );
        // 测试通过，没有 panic
    }

    #[test]
    fn test_audit_logger_disabled() {
        let logger = RbacAuditLogger::new(false);
        logger.log_create_role(
            "actor-1".to_string(),
            "realm-1".to_string(),
            "role-1".to_string(),
            "test-role".to_string(),
        );
        // 测试通过，没有日志输出（被禁用）
    }
}
