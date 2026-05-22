// OAuth router

use axum::{
    Router,
    routing::{get, post},
};
use herald_core::domain::authentication::ports::AuthenticationService;
use herald_core::domain::oauth::ports::{OAuthConfigService, OAuthRepository};
use herald_core::domain::oauth::services::OAuthService;
use herald_core::domain::user::ports::UserService;

use crate::{
    authorize::oauth_authorize,
    callback::{oauth_callback, oauth_callback_form},
    login::oauth_login,
    token::oauth_token,
};

pub fn oauth_router<R, C, U, A>() -> Router<()>
where
    R: OAuthRepository + 'static,
    C: OAuthConfigService + 'static,
    U: UserService + 'static,
    A: AuthenticationService + 'static,
{
    Router::new()
        .route("/authorize", get(oauth_authorize))
        .route("/token", post(oauth_token))
        .route("/{provider}/login", get(oauth_login))
        .route(
            "/{provider}/callback",
            get(oauth_callback).post(oauth_callback_form),
        )
}
