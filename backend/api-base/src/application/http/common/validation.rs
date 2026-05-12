// =============================================================================
// 验证辅助模块
// =============================================================================
//
// 提供通用的输入验证函数。
//
// =============================================================================

/// Check if a database error is a duplicate key/unique constraint violation
///
/// # Arguments
/// * `error` - The database error to check
///
/// # Returns
/// * `true` if the error is a duplicate key violation
/// * `false` otherwise
///
/// # Example
/// ```ignore
/// match user_role.insert(db).await {
///     Ok(_) => Ok(()),
///     Err(e) => {
///         if is_duplicate_key_error(&e) {
///             Ok(()) // Idempotent - already exists
///         } else {
///             Err(e)
///         }
///     }
/// }
/// ```
pub fn is_duplicate_key_error(error: &dyn std::error::Error) -> bool {
    let error_str = error.to_string().to_lowercase();
    error_str.contains("duplicate key") || error_str.contains("unique constraint")
}
