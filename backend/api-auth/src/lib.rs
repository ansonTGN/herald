// Herald API Auth Module
// Authentication handlers (login, register, password reset, TOTP, email verification)

pub mod change_email;
pub mod login;
pub mod logout;
pub mod register;
pub mod registration_status;
pub mod reset_password;
pub mod status;
pub mod turnstile_status;
pub mod user_totp;
pub mod verify_email;
pub mod verify_totp;

#[cfg(test)]
mod util_test;

use axum::routing::get;
use axum::{Router, routing::post};
use herald_api_base::application::http::state::AppState;

// Re-export shared auth utilities from api-base for backward compatibility
pub mod util {
    pub use herald_api_base::application::http::auth::util::*;
}
pub mod identity_middleware {
    pub use herald_api_base::application::http::auth::identity_middleware::*;
}
pub mod error {
    pub use herald_api_base::application::http::auth::error::*;
}

// Re-export commonly used types and functions
pub use login::{LoginRequestPayload, LoginResponse};

// Re-export utoipa path markers
pub use change_email::__path_confirm as __path_change_email_confirm;
pub use change_email::__path_request as __path_change_email_request;
pub use login::__path_login;
pub use logout::__path_logout;
pub use register::__path_register;
pub use reset_password::__path_confirm as __path_reset_password_confirm;
pub use reset_password::__path_request as __path_reset_password_request;
pub use status::__path_status;
pub use turnstile_status::__path_get_turnstile_status;
pub use user_totp::__path_handle_disable_totp;
pub use user_totp::__path_handle_enable_totp;
pub use user_totp::__path_handle_get_totp_status;
pub use user_totp::__path_handle_regenerate_totp;
pub use user_totp::__path_handle_verify_totp_setup;
pub use verify_email::__path_confirm as __path_verify_email_confirm;
pub use verify_email::__path_trigger as __path_verify_email_trigger;
pub use verify_totp::__path_handle_verify_totp as __path_verify_totp;

/// OpenAPI specification for auth module
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        crate::login::login,
        crate::register::register,
        crate::logout::logout,
        crate::status::status,
        crate::turnstile_status::get_turnstile_status,
        crate::verify_email::trigger,
        crate::verify_email::confirm,
        crate::reset_password::request,
        crate::reset_password::confirm,
        crate::change_email::request,
        crate::change_email::confirm,
        crate::verify_totp::handle_verify_totp,
        crate::user_totp::handle_enable_totp,
        crate::user_totp::handle_verify_totp_setup,
        crate::user_totp::handle_disable_totp,
        crate::user_totp::handle_regenerate_totp,
        crate::user_totp::handle_get_totp_status,
    ),
    components(schemas(
        crate::login::LoginRequestPayload,
        crate::login::LoginResponse,
        crate::register::RegisterRequest,
        crate::register::RegisterResponse,
        crate::status::StatusResponse,
        crate::turnstile_status::TurnstileStatusResponse,
        crate::verify_email::VerifyEmailTriggerRequest,
        crate::verify_email::VerifyEmailConfirmResponse,
        crate::reset_password::ResetPasswordRequestRequest,
        crate::reset_password::ResetPasswordRequestResponse,
        crate::reset_password::ResetPasswordConfirmRequest,
        crate::reset_password::ResetPasswordConfirmResponse,
        crate::change_email::ChangeEmailRequest,
        crate::change_email::ChangeEmailResponse,
        crate::verify_totp::VerifyTotpRequest,
        crate::verify_totp::VerifyTotpResponse,
        crate::user_totp::EnableTotpRequest,
        crate::user_totp::EnableTotpResponse,
        crate::user_totp::VerifyTotpSetupRequest,
        crate::user_totp::VerifyTotpSetupResponse,
        crate::user_totp::DisableTotpRequest,
        crate::user_totp::DisableTotpResponse,
        crate::user_totp::RegenerateTotpRequest,
        crate::user_totp::RegenerateTotpResponse,
        crate::user_totp::TotpStatusResponse,
        crate::user_totp::BackupCodeStatsResponse,
    ))
)]
pub struct ApiDoc;

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register::register))
        .route("/login", post(login::login))
        .route("/login/verify-totp", post(verify_totp::handle_verify_totp))
        .route("/logout", get(logout::logout).post(logout::logout))
        .route("/status", get(status::status))
        .route(
            "/turnstile/status",
            post(turnstile_status::get_turnstile_status),
        )
        .route(
            "/registration/status",
            post(registration_status::get_registration_status),
        )
        .route("/verify_email/trigger", post(verify_email::trigger))
        .route(
            "/verify_email/confirm/{email_verification_code}",
            get(verify_email::confirm),
        )
        .route("/reset_password/request", post(reset_password::request))
        .route(
            "/reset_password/confirm/{reset_code}",
            post(reset_password::confirm),
        )
        .route("/change_email/request", post(change_email::request))
        .route(
            "/change_email/confirm/{change_code}",
            get(change_email::confirm),
        )
}
