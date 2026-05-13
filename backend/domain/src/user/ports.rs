use crate::authentication::Identity;
use crate::common::entities::app_errors::CoreError;
use crate::user::{
    entities::{Profile, User},
    value_objects::{CreateUserRequest, LoginRequest, RegisterRequest, UpdateUserRequest},
};
use std::future::Future;
use uuid::Uuid;

// ============================================================================
// Repository Ports (Traits)
// ============================================================================

#[cfg_attr(test, mockall::automock)]
pub trait UserRepository: Send + Sync {
    fn create_user(
        &self,
        request: CreateUserRequest,
        password_hash: String,
    ) -> impl Future<Output = Result<User, CoreError>> + Send;

    fn get_user_by_id(&self, id: Uuid) -> impl Future<Output = Result<User, CoreError>> + Send;

    fn get_user_by_email(
        &self,
        realm_id: &str,
        email: &str,
    ) -> impl Future<Output = Result<User, CoreError>> + Send;

    fn get_user_by_email_or_username(
        &self,
        realm_id: &str,
        email: Option<String>,
        username: Option<String>,
    ) -> impl Future<Output = Result<Option<(Uuid, Option<String>, i16)>, CoreError>> + Send;

    fn change_password(
        &self,
        realm_id: &str,
        user_id: Uuid,
        new_password_hash: String,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn update_user_status(
        &self,
        user_id: Uuid,
        status: i16,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn list_users(
        &self,
        realm_id: &str,
        page: u64,
        page_size: u64,
        email: Option<String>,
    ) -> impl Future<Output = Result<(Vec<User>, i64), CoreError>> + Send;

    fn update_user(
        &self,
        id: Uuid,
        request: UpdateUserRequest,
    ) -> impl Future<Output = Result<User, CoreError>> + Send;

    fn delete_user(&self, id: Uuid) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn create_profile(
        &self,
        profile: Profile,
    ) -> impl Future<Output = Result<Profile, CoreError>> + Send;

    fn get_profile(&self, user_id: Uuid)
    -> impl Future<Output = Result<Profile, CoreError>> + Send;

    fn update_profile(
        &self,
        user_id: Uuid,
        nickname: Option<String>,
    ) -> impl Future<Output = Result<Profile, CoreError>> + Send;
}

#[cfg_attr(test, mockall::automock)]
pub trait UserVerificationRepository: Send + Sync {
    fn create_verification_code(
        &self,
        email: &str,
        code_type: &str,
        code: &str,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn verify_code(
        &self,
        email: &str,
        code_type: &str,
        code: &str,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    fn consume_code(&self, code: &str) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Get email address by verification code
    fn get_email_by_code(
        &self,
        code: &str,
    ) -> impl Future<Output = Result<Option<String>, CoreError>> + Send;

    /// Delete verification codes by type for a specific email
    fn delete_code_by_type(
        &self,
        email: &str,
        code_type: &str,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}

// ============================================================================
// Service Ports (Traits)
// ============================================================================

#[cfg_attr(test, mockall::automock)]
pub trait UserService: Send + Sync {
    fn create_user(
        &self,
        identity: Identity,
        request: CreateUserRequest,
    ) -> impl Future<Output = Result<User, CoreError>> + Send;

    fn get_user(
        &self,
        identity: Identity,
        id: Uuid,
    ) -> impl Future<Output = Result<User, CoreError>> + Send;

    fn list_users(
        &self,
        identity: Identity,
        realm_id: String,
        page: u64,
        page_size: u64,
        email: Option<String>,
    ) -> impl Future<Output = Result<(Vec<User>, i64), CoreError>> + Send;

    fn update_user(
        &self,
        identity: Identity,
        id: Uuid,
        request: UpdateUserRequest,
    ) -> impl Future<Output = Result<User, CoreError>> + Send;

    fn delete_user(
        &self,
        identity: Identity,
        id: Uuid,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn verify_email(
        &self,
        code: &str,
        realm_id: &str,
    ) -> impl Future<Output = Result<User, CoreError>> + Send;

    fn verify_email_trigger(
        &self,
        realm_id: &str,
        email: &str,
        code_type: &str,
    ) -> impl Future<Output = Result<String, CoreError>> + Send;

    fn login(&self, request: LoginRequest) -> impl Future<Output = Result<User, CoreError>> + Send;

    fn register(
        &self,
        request: RegisterRequest,
    ) -> impl Future<Output = Result<User, CoreError>> + Send;

    /// Create user without password (for OAuth)
    fn create_user_without_password(
        &self,
        request: CreateUserRequest,
    ) -> impl Future<Output = Result<User, CoreError>> + Send;

    /// Create user without identity/realm boundary checks
    /// For internal use by system operations (e.g., realm initialization)
    fn create_user_without_identity_check(
        &self,
        request: CreateUserRequest,
    ) -> impl Future<Output = Result<User, CoreError>> + Send;

    fn change_password(
        &self,
        realm_id: &str,
        user_id: Uuid,
        old_password: String,
        new_password: String,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn reset_password_request(
        &self,
        realm_id: &str,
        email: &str,
        code_type: &str,
    ) -> impl Future<Output = Result<String, CoreError>> + Send;

    fn reset_password_confirm(
        &self,
        code: &str,
        new_password: String,
        realm_id: &str,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Activate user account (for realms without email verification)
    fn activate_user(&self, user_id: Uuid) -> impl Future<Output = Result<(), CoreError>> + Send;
}
