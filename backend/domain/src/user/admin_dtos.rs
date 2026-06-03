// User Admin Data Transfer Objects

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request to create a user with roles
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateUserWithRolesRequest {
    pub email: String,
    pub password: String,
    pub nickname: Option<String>,
    pub role_ids: Vec<Uuid>,
    pub status: Option<i32>,
}

/// Request to update a user (admin operations)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateUserAdminRequest {
    pub nickname: Option<String>,
    pub status: Option<i32>,
}

/// Response for admin user operations
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdminUser {
    pub id: Uuid,
    pub realm_id: String,
    pub email: String,
    pub nickname: Option<String>,
    pub status: i32,
    pub created_at: String,
}

/// Role detail with policies
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RoleDetail {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

/// Policy detail
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PolicyDetail {
    pub id: Uuid,
    pub resource: String,
    pub action: String,
}

/// Permission detail (effective permissions)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PermissionDetail {
    pub resource: String,
    pub action: String,
    pub source: PermissionSource,
}

/// Source of a permission (role or direct assignment)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PermissionSource {
    Role { role_id: Uuid, role_name: String },
    Direct,
}

/// User role assignment response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserRoleAssignment {
    pub user_id: Uuid,
    pub role_id: Uuid,
    pub role_name: String,
}
