use crate::admin::permission::PermissionData;
use herald_api_base::application::http::server::api_entities::ApiError;

// =============================================================================
// Permission Validation Middleware
// =============================================================================
//
// 验证自定义 RBAC 策略的合法性
// 次管理员不能创建主管理员级别的策略
//
// =============================================================================

/// 验证权限策略是否合法
/// 次管理员不能创建主管理员级别的策略
///
/// 注意：此函数验证自定义 RBAC 策略（基于 PermissionService）
pub fn validate_policy_for_realm_admin(
    policy: &PermissionData,
    caller_realm_id: &str,
) -> Result<(), ApiError> {
    match policy {
        PermissionData::PoliceWrap(police) => {
            // 禁止创建 "All" 策略
            if police.resource == "All" || police.resource == "*" {
                return Err(ApiError::forbidden(
                    "Realm admin cannot create 'All' or wildcard policies",
                ));
            }

            // 禁止创建跨 Realm 策略
            // 需要精确匹配 realm_id，避免 realm-10 匹配 realm-1 的情况
            let resource_lower = police.resource.to_lowercase();
            let realm_id_lower = caller_realm_id.to_lowercase();

            // 检查是否包含精确的 realm 引用
            // 使用单词边界匹配，确保 realm-10 不会匹配 realm-1
            let has_valid_realm = if let Some(colon_pos) = resource_lower.find(':') {
                let after_colon = &resource_lower[colon_pos + 1..];
                // 提取realm部分，直到遇到非字母数字、非连字符的字符
                let realm_end = after_colon
                    .find(|c: char| !c.is_alphanumeric() && c != '-')
                    .unwrap_or(after_colon.len());
                let realm_part = &after_colon[..realm_end];
                realm_part == realm_id_lower
            } else {
                resource_lower.starts_with(&format!("/api/{}", realm_id_lower))
            };

            if !has_valid_realm {
                return Err(ApiError::forbidden(
                    "Realm admin cannot create cross-realm policies",
                ));
            }

            Ok(())
        }
        PermissionData::RoleWrap(_role) => {
            // 角色分配验证
            // 注意：role 现在是 UUID，无法通过名称判断 realm 隶属关系
            // Realm 边界检查将在数据库层执行（通过查询 roles 表的 realm_id）
            // 此处只进行基本验证：确保 role_id 是有效的 UUID（由 serde 自动处理）
            Ok(())
        }
    }
}

const SENSITIVE_PERMISSIONS: &[&str] = &["realm.create"];

/// Validates that sensitive permissions can only be created in the admin realm
///
/// # Arguments
/// * `permission_name` - The name of the permission being created
/// * `caller_realm_id` - The realm ID of the caller creating the permission
///
/// # Returns
/// * `Ok(())` if the permission can be created
/// * `Err(ApiError::Forbidden)` if the permission is sensitive and caller is not in admin realm
pub fn validate_sensitive_permission_creation(
    permission_name: &str,
    caller_realm_id: &str,
) -> Result<(), ApiError> {
    if SENSITIVE_PERMISSIONS.contains(&permission_name) && caller_realm_id != "admin" {
        return Err(ApiError::forbidden(format!(
            "Permission '{}' can only be created in admin realm",
            permission_name
        )));
    }
    Ok(())
}
