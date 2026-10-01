use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::oauth::{
    entities::ProviderType,
    http_client::{HttpClient, HttpClientRequestBuilder, HttpMethod},
    ports::OAuthProviderHandler,
    value_objects::{OAuthConfig, OAuthUserInfo},
};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, EndpointNotSet, EndpointSet, RedirectUrl,
    Scope, TokenResponse, TokenUrl, basic::BasicClient,
};
use serde::Deserialize;

// Auth and token endpoints set; the optional endpoints (device auth,
// introspection, revocation) left unset.
type EndpointConfiguredClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

pub struct DiscordOAuthProvider;

impl DiscordOAuthProvider {
    const AUTH_URL: &'static str = "https://discord.com/oauth2/authorize";
    const TOKEN_URL: &'static str = "https://discord.com/api/oauth2/token";
    const USER_API_URL: &'static str = "https://discord.com/api/users/@me";

    fn basic_client(config: &OAuthConfig) -> Result<EndpointConfiguredClient, CoreError> {
        Ok(BasicClient::new(ClientId::new(config.client_id.clone()))
            .set_client_secret(ClientSecret::new(config.client_secret.clone()))
            .set_auth_uri(AuthUrl::new(Self::AUTH_URL.to_string())?)
            .set_token_uri(TokenUrl::new(Self::TOKEN_URL.to_string())?)
            .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone())?))
    }
}

#[derive(Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    global_name: Option<String>,
    avatar: Option<String>,
    email: Option<String>,
    verified: Option<bool>,
}

impl DiscordUser {
    /// Avatar CDN URL. An `a_`-prefixed hash is animated (`.gif`), any other
    /// hash is static (`.png`); a null avatar returns None — the default
    /// avatar is deliberately not materialized.
    fn avatar_url(&self) -> Option<String> {
        self.avatar.as_ref().map(|hash| {
            let ext = if hash.starts_with("a_") { "gif" } else { "png" };
            format!(
                "https://cdn.discordapp.com/avatars/{}/{}.{}",
                self.id, hash, ext
            )
        })
    }

    fn into_user_info(self) -> Result<OAuthUserInfo, CoreError> {
        let avatar = self.avatar_url();
        // Discord only returns email/verified under the `email` scope, but
        // every Discord account is registered with a bound address — a missing
        // email therefore means the realm's scopes are misconfigured. An
        // empty or blank string is the same contract anomaly, not a usable
        // address (the account-creation path never runs #[validate(email)]),
        // so both are rejected identically and explicitly (no placeholder
        // mailbox): the message names the cause so the admin can fix the
        // config and the login self-heals.
        let email = self
            .email
            .filter(|e| !e.trim().is_empty())
            .ok_or_else(|| {
                CoreError::BadRequest(
                    "Email not provided by Discord. Check that the 'email' scope is configured for this provider."
                        .to_string(),
                )
            })?;
        // `verified` is the input to the email-verified gate in
        // find_or_create_user_by_email; Discord exposes a real per-email flag,
        // so it passes through untouched, and an absent field next to a
        // present email is a contract anomaly that fails closed — never
        // assert a proof the provider did not give.
        let verified = self.verified.unwrap_or(false);
        Ok(OAuthUserInfo {
            provider_type: ProviderType::Discord,
            provider_user_id: self.id.clone(),
            email,
            verified,
            avatar,
            name: self.global_name.or(Some(self.username)),
            union_id: None, // Discord doesn't provide UnionID
            open_id: Some(self.id),
        })
    }
}

impl OAuthProviderHandler for DiscordOAuthProvider {
    fn provider_type(&self) -> &'static str {
        "discord"
    }

    fn display_name(&self) -> &'static str {
        "Discord"
    }

    fn get_auth_url(&self, state: &str, config: &OAuthConfig) -> Result<String, CoreError> {
        let client = Self::basic_client(config)?;

        let scopes: Vec<Scope> = if config.scopes.is_empty() {
            vec![
                Scope::new("identify".to_string()),
                Scope::new("email".to_string()),
            ]
        } else {
            config
                .scopes
                .iter()
                .map(|s| Scope::new(s.clone()))
                .collect()
        };

        let (auth_url, _csrf_token) = client
            .authorize_url(|| oauth2::CsrfToken::new(state.to_string()))
            .add_scopes(scopes)
            .url();

        Ok(auth_url.to_string())
    }

    #[allow(clippy::manual_async_fn)]
    fn exchange_code_and_get_user<H>(
        &self,
        code: String,
        config: &OAuthConfig,
        http_client: &H,
    ) -> impl Future<Output = Result<OAuthUserInfo, CoreError>> + Send
    where
        H: HttpClient + Send + Sync,
    {
        async move {
            let client = Self::basic_client(config)?;

            let oauth_http_client = oauth2::reqwest::ClientBuilder::new()
                .redirect(oauth2::reqwest::redirect::Policy::none())
                .build()
                .map_err(|e| {
                    CoreError::InternalServerError(format!(
                        "Failed to build OAuth HTTP client: {}",
                        e
                    ))
                })?;

            // Token-endpoint failures (network, upstream 5xx, expired or
            // replayed code) are server-side faults: they must leave here as
            // InternalServerError so the callback emits a sanitized 500 —
            // BadRequest is reserved for the diagnosable missing-email config
            // error in into_user_info, which alone reaches the client as 400.
            let token_result = client
                .exchange_code(AuthorizationCode::new(code))
                .request_async(&oauth_http_client)
                .await
                .map_err(|e| {
                    CoreError::InternalServerError(format!("Token exchange failed: {}", e))
                })?;

            let access_token = token_result.access_token().secret();

            let response = http_client
                .request(
                    HttpClientRequestBuilder::new(Self::USER_API_URL, HttpMethod::Get)
                        .bearer_auth(access_token)
                        .build(),
                )
                .await?;

            if !response.is_success() {
                let status_code = response.status_code;
                let response_body = response.body_as_string().unwrap_or_default();
                return Err(CoreError::InternalServerError(format!(
                    "Failed to get user info from Discord: status={}, body={}",
                    status_code, response_body
                )));
            }

            let response_body = response.body_as_string()?;
            let discord_user: DiscordUser = serde_json::from_str(&response_body).map_err(|e| {
                CoreError::InternalServerError(format!("Failed to parse user info: {}", e))
            })?;

            discord_user.into_user_info()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The snowflake id is the stable identity: it must land in open_id (the
    // (realm_id, 'discord', open_id) unique-index lookup) and provider_user_id,
    // never the mutable username. global_name is the display name with the
    // handle as fallback, and Discord has no union layer to contribute.
    #[test]
    fn discord_user_json_maps_to_oauth_user_info() {
        let user: DiscordUser = serde_json::from_str(
            r#"{"id":"123456789012345678","username":"legostax","global_name":"Lego Stax","avatar":"abc","email":"lego@example.com","verified":true}"#,
        )
        .expect("sample /users/@me payload must parse");
        let info = user.into_user_info().expect("complete payload must map");
        assert_eq!(info.open_id.as_deref(), Some("123456789012345678"));
        assert_eq!(info.provider_user_id, "123456789012345678");
        assert_eq!(info.name.as_deref(), Some("Lego Stax"));
        assert_eq!(info.union_id, None);

        let fallback: DiscordUser = serde_json::from_str(
            r#"{"id":"2","username":"handle","email":"handle@example.com","verified":true}"#,
        )
        .expect("payload without global_name must parse");
        let info = fallback
            .into_user_info()
            .expect("complete payload must map");
        assert_eq!(info.name.as_deref(), Some("handle"));
    }

    // find_or_create_user_by_email trusts `verified` as the sole gate for
    // email-matched linking and auto-registration (docs/user-stories/core/
    // regular-user.md US-RU-003), and Discord exposes a real per-email flag —
    // so it must pass through unchanged. Rewriting false to true (the approach
    // for providers with no trustworthy flag) would convert an unverified
    // credential into account takeover; an absent field next to a present
    // email is a contract anomaly and must fail closed to false.
    #[test]
    fn discord_verified_flag_passes_through_fail_closed() {
        let user: DiscordUser = serde_json::from_str(
            r#"{"id":"1","username":"u","email":"u@example.com","verified":true}"#,
        )
        .expect("payload must parse");
        assert!(
            user.into_user_info()
                .expect("complete payload must map")
                .verified
        );

        let user: DiscordUser = serde_json::from_str(
            r#"{"id":"1","username":"u","email":"u@example.com","verified":false}"#,
        )
        .expect("payload must parse");
        assert!(
            !user
                .into_user_info()
                .expect("complete payload must map")
                .verified
        );

        let user: DiscordUser =
            serde_json::from_str(r#"{"id":"1","username":"u","email":"u@example.com"}"#)
                .expect("payload must parse");
        assert!(
            !user
                .into_user_info()
                .expect("complete payload must map")
                .verified
        );
    }

    // A missing email means the realm's scope list dropped `email` (every
    // Discord account has a bound address): the rejection must be an explicit
    // BadRequest naming the cause so the admin can fix the configuration — a
    // placeholder mailbox or a bare 500 would hide the misconfiguration
    // instead of letting it self-heal. A blank string is rejected the same
    // way: it is not a usable address, and the account-creation path never
    // runs #[validate(email)] on it.
    #[test]
    fn discord_missing_email_rejected_with_bad_request() {
        let user: DiscordUser =
            serde_json::from_str(r#"{"id":"1","username":"u","verified":true}"#)
                .expect("payload without email must parse");
        match user.into_user_info() {
            Err(CoreError::BadRequest(msg)) => assert!(
                msg.contains("Email not provided by Discord"),
                "unexpected message: {msg}"
            ),
            other => panic!("expected BadRequest, got {:?}", other),
        }

        for payload in [
            r#"{"id":"1","username":"u","email":"","verified":true}"#,
            r#"{"id":"1","username":"u","email":"   ","verified":true}"#,
        ] {
            let user: DiscordUser =
                serde_json::from_str(payload).expect("payload with blank email must parse");
            match user.into_user_info() {
                Err(CoreError::BadRequest(msg)) => assert!(
                    msg.contains("Email not provided by Discord"),
                    "unexpected message: {msg}"
                ),
                other => panic!("expected BadRequest, got {:?}", other),
            }
        }
    }

    // An `a_`-prefixed hash is animated: storing the `.png` extension would
    // save a dead link, because the CDN serves the animation under `.gif`
    // only. A null avatar must stay None rather than materializing a
    // default-avatar URL.
    #[test]
    fn discord_avatar_url_gif_png_and_null() {
        let animated: DiscordUser = serde_json::from_str(
            r#"{"id":"1","username":"u","avatar":"a_abc","email":"u@example.com","verified":true}"#,
        )
        .expect("payload must parse");
        assert_eq!(
            animated.avatar_url().as_deref(),
            Some("https://cdn.discordapp.com/avatars/1/a_abc.gif")
        );

        let static_png: DiscordUser = serde_json::from_str(
            r#"{"id":"1","username":"u","avatar":"abc","email":"u@example.com","verified":true}"#,
        )
        .expect("payload must parse");
        assert_eq!(
            static_png.avatar_url().as_deref(),
            Some("https://cdn.discordapp.com/avatars/1/abc.png")
        );

        let null_avatar: DiscordUser = serde_json::from_str(
            r#"{"id":"1","username":"u","avatar":null,"email":"u@example.com","verified":true}"#,
        )
        .expect("payload must parse");
        assert_eq!(null_avatar.avatar_url(), None);
    }
}
