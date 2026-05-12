// OAuth router

use axum::{Router, routing::get};
use herald_core::domain::authentication::ports::AuthenticationService;
use herald_core::domain::oauth::services::OAuthService;
use herald_core::domain::oauth::ports::{OAuthConfigService, OAuthRepository};
use herald_core::domain::user::ports::UserService;

use crate::{authorize::oauth_authorize, callback::oauth_callback, login::oauth_login};

pub fn oauth_router<R, C, U, A>() -> Router<()>
where
    R: OAuthRepository + 'static,
    C: OAuthConfigService + 'static,
    U: UserService + 'static,
    A: AuthenticationService + 'static,
{
    Router::new()
        .route("/authorize", get(oauth_authorize))
        .route("/{provider}/login", get(oauth_login))
        .route("/{provider}/callback", get(oauth_callback))
}
