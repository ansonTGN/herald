use serde::{Deserialize, Serialize};

/// High-level category for audit events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    UserManagement,
    Rbac,
    RealmManagement,
    Auth,
}

/// Specific action that was performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    UserCreate,
    UserUpdate,
    UserDelete,
    RoleCreate,
    RoleUpdate,
    RoleDelete,
    PermissionCreate,
    PermissionDelete,
    RoleAssign,
    RoleUnassign,
    PermissionGrant,
    PermissionRevoke,
    RealmCreate,
    RealmRbacInit,
    AuthLogin,
    AuthLogout,
    AuthLoginFailed,
}

/// Type of the target entity an audit event refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditTargetType {
    User,
    Role,
    Permission,
    Realm,
    Session,
}

/// Outcome of the audited operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditResult {
    Success,
    Failure,
}

/// What kind of actor performed the operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorType {
    User,
    Admin,
    System,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_action_serializes_to_snake_case() {
        let pairs: Vec<(AuditAction, &str)> = vec![
            (AuditAction::UserCreate, "user_create"),
            (AuditAction::UserUpdate, "user_update"),
            (AuditAction::UserDelete, "user_delete"),
            (AuditAction::RoleCreate, "role_create"),
            (AuditAction::RoleUpdate, "role_update"),
            (AuditAction::RoleDelete, "role_delete"),
            (AuditAction::PermissionCreate, "permission_create"),
            (AuditAction::PermissionDelete, "permission_delete"),
            (AuditAction::RoleAssign, "role_assign"),
            (AuditAction::RoleUnassign, "role_unassign"),
            (AuditAction::PermissionGrant, "permission_grant"),
            (AuditAction::PermissionRevoke, "permission_revoke"),
            (AuditAction::RealmCreate, "realm_create"),
            (AuditAction::RealmRbacInit, "realm_rbac_init"),
            (AuditAction::AuthLogin, "auth_login"),
            (AuditAction::AuthLogout, "auth_logout"),
            (AuditAction::AuthLoginFailed, "auth_login_failed"),
        ];

        for (variant, expected) in pairs {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(
                json.trim_matches('"'),
                expected,
                "{:?} should serialize to {}",
                variant,
                expected
            );
        }
    }

    #[test]
    fn audit_category_roundtrip() {
        let categories = vec![
            AuditCategory::UserManagement,
            AuditCategory::Rbac,
            AuditCategory::RealmManagement,
            AuditCategory::Auth,
        ];
        for c in &categories {
            let json = serde_json::to_string(c).unwrap();
            let back: AuditCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(*c, back);
        }
    }

    #[test]
    fn audit_result_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&AuditResult::Success).unwrap(),
            "\"success\""
        );
        assert_eq!(
            serde_json::to_string(&AuditResult::Failure).unwrap(),
            "\"failure\""
        );
    }

    #[test]
    fn audit_target_type_roundtrip() {
        let types = vec![
            AuditTargetType::User,
            AuditTargetType::Role,
            AuditTargetType::Permission,
            AuditTargetType::Realm,
            AuditTargetType::Session,
        ];
        for t in &types {
            let json = serde_json::to_string(t).unwrap();
            let back: AuditTargetType = serde_json::from_str(&json).unwrap();
            assert_eq!(*t, back);
        }
    }

    #[test]
    fn actor_type_roundtrip() {
        let types = vec![ActorType::User, ActorType::Admin, ActorType::System];
        for t in &types {
            let json = serde_json::to_string(t).unwrap();
            let back: ActorType = serde_json::from_str(&json).unwrap();
            assert_eq!(*t, back);
        }
    }
}
