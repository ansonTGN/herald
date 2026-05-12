// User Admin Domain Entities

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Admin user domain entity
#[derive(Debug, Clone)]
pub struct AdminUserEntity {
    pub id: Uuid,
    pub realm_id: String,
    pub email: String,
    pub nickname: Option<String>,
    pub status: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Role domain entity
#[derive(Debug, Clone)]
pub struct RoleEntity {
    pub id: Uuid,
    pub realm_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Policy domain entity
#[derive(Debug, Clone)]
pub struct PolicyEntity {
    pub id: Uuid,
    pub realm_id: String,
    pub resource: String,
    pub action: String,
    pub policy_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User role assignment entity
#[derive(Debug, Clone)]
pub struct UserRoleEntity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub role_id: Uuid,
    pub realm_id: String,
    pub client_id: String,
    pub created_at: DateTime<Utc>,
}

/// Role policy entity
#[derive(Debug, Clone)]
pub struct RolePolicyEntity {
    pub id: Uuid,
    pub realm_id: String,
    pub role_id: Uuid,
    pub policy_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// User policy entity (direct permission assignment)
#[derive(Debug, Clone)]
pub struct UserPolicyEntity {
    pub id: Uuid,
    pub realm_id: String,
    pub user_id: Uuid,
    pub policy_id: Uuid,
    pub created_at: DateTime<Utc>,
}
