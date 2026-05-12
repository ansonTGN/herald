// Herald API Admin Module
// Admin handlers, role definitions, permission management

pub mod admin;
pub mod permission;
pub mod role_definitions;

// Re-export router functions for use by the main api crate
pub use admin::admin_router_with_middleware;

/// OpenAPI specification for admin module
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        crate::admin::admin_users::list_users,
        crate::admin::admin_users::create_user,
        crate::admin::admin_users::get_user,
        crate::admin::admin_users::update_user,
        crate::admin::admin_users::delete_user,
        crate::admin::admin_users::reset_user_password,
        crate::admin::admin_users::roles::get_user_roles,
        crate::admin::admin_users::roles::update_user_roles,
        crate::admin::permission::create_permission,
        crate::admin::permission::list_permission,
        crate::admin::permission::delete_permission,
        crate::admin::permission::check_permission,
        crate::role_definitions::list_roles,
        crate::role_definitions::create_role,
        crate::role_definitions::get_role,
        crate::role_definitions::update_role,
        crate::role_definitions::delete_role,
        crate::role_definitions::assign_permission_to_role,
        crate::role_definitions::get_role_permissions,
        crate::role_definitions::remove_permission_from_role,
        crate::admin::permission_definitions::list_permission_definitions,
        crate::admin::permission_definitions::create_permission_definition,
        crate::admin::permission_definitions::get_permission_definition,
        crate::admin::permission_definitions::update_permission_definition,
        crate::admin::permission_definitions::delete_permission_definition,
    ),
    components(schemas(
        crate::role_definitions::types::RoleCreateRequest,
        crate::role_definitions::types::RoleUpdateRequest,
        crate::role_definitions::types::RoleResponse,
        crate::role_definitions::types::AssignPermissionRequest,
        crate::role_definitions::types::PermissionResponse,
        crate::admin::permission_definitions::types::PermissionCreateRequest,
        crate::admin::permission_definitions::types::PermissionUpdateRequest,
        crate::admin::permission_definitions::types::PermissionResponse,
        crate::admin::admin_users::types::UserRoleDetail,
        crate::admin::admin_users::types::UserRolesResponse,
        crate::admin::admin_users::types::UpdateUserRolesRequest,
        crate::admin::permission::Role,
        crate::admin::permission::Police,
        crate::admin::permission::PermissionData,
    ))
)]
pub struct ApiDoc;
