// User Admin Domain Errors

use crate::common::entities::app_errors::CoreError;

/// User Admin domain-specific errors
#[derive(Debug, thiserror::Error)]
pub enum UserAdminError {
    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Role not found: {0}")]
    RoleNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid role assignment: {0}")]
    InvalidRoleAssignment(String),

    #[error("Duplicate email: {0}")]
    DuplicateEmail(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<UserAdminError> for CoreError {
    fn from(err: UserAdminError) -> Self {
        match err {
            UserAdminError::UserNotFound(msg) => {
                tracing::debug!("User not found: {}", msg);
                CoreError::NotFound
            }
            UserAdminError::RoleNotFound(msg) => {
                tracing::debug!("Role not found: {}", msg);
                CoreError::NotFound
            }
            UserAdminError::PermissionDenied(msg) => CoreError::Forbidden(msg),
            UserAdminError::InvalidRoleAssignment(msg) => CoreError::BadRequest(msg),
            UserAdminError::DuplicateEmail(msg) => CoreError::BadRequest(msg),
            UserAdminError::DatabaseError(msg) => CoreError::InternalServerError(msg),
            UserAdminError::InternalError(msg) => CoreError::InternalServerError(msg),
        }
    }
}

pub type UserAdminResult<T> = Result<T, UserAdminError>;
