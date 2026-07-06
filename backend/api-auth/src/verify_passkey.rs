use axum::{
    Json,
    extract::{Path, State},
    http::header::SET_COOKIE,
    response::{IntoResponse, Response},
};
use axum_valid::Valid;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use herald_api_base::application::http::auth::util::{
    ClientIp, SessionData, build_set_cookie, epoch_seconds, rate_limit_hit,
    renewal_ttl_seconds_from_i32, store_session, verify_turnstile_for_realm,
};
use herald_api_base::application::http::server::api_entities::ApiError;
pub use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::client::ports::ClientService;
use herald_core::domain::realm_config::ConfigType;
use herald_core::domain::security_constants::{
    LOGIN_IDENTIFIER_RATE_LIMIT, LOGIN_IP_RATE_LIMIT, OAUTH_STATE_TTL_SECONDS,
    TOTP_LOCKOUT_SECONDS, TOTP_MAX_FAILURES, TOTP_VERIFY_IP_RATE_LIMIT,
    TOTP_VERIFY_USER_RATE_LIMIT,
};
use herald_core::domain::user::ports::UserRepository;
use herald_core::domain::user_passkey::{PasskeyError, PasskeyLoginState, UserPasskeyService};
use herald_core::infrastructure::user_passkey::{
    PostgresUserPasskeyRepository, RedisPasskeyChallengeStore,
};

use crate::consent_gate::AuthConsentAgreement;

const PASSKEY_VERIFY_FAILED: &str = "Passkey 验证失败";

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyOptionsRequest {
    #[validate(length(min = 1, max = 64))]
    pub client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turnstile_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth: Option<PasskeyOAuthRequest>,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyOAuthRequest {
    #[validate(length(min = 1, max = 128))]
    pub client_id: String,
    #[validate(length(min = 1, max = 2048))]
    pub redirect_uri: String,
    #[validate(length(min = 1, max = 512))]
    pub state: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyOptionsResponse {
    pub auth_token: String,
    pub options: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyVerifyRequest {
    #[validate(length(min = 1, max = 128))]
    pub auth_token: String,
    pub assertion: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(required = false)]
    pub agreements: Option<Vec<AuthConsentAgreement>>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyVerifyResponse {
    pub message: String,
    pub user_id: String,
    pub token: String,
    pub expires_in_seconds: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consent_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agreements: Option<Vec<herald_core::domain::legal::LegalAgreementSummary>>,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Passkey2faOptionsRequest {
    #[validate(length(min = 1, max = 128))]
    pub temp_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Passkey2faOptionsResponse {
    pub auth_token: String,
    pub options: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Passkey2faVerifyRequest {
    #[validate(length(min = 1, max = 128))]
    pub temp_token: String,
    #[validate(length(min = 1, max = 128))]
    pub auth_token: String,
    pub assertion: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(required = false)]
    pub agreements: Option<Vec<AuthConsentAgreement>>,
}

#[derive(Serialize, Deserialize)]
struct TempSessionData {
    user_id: String,
    realm_id: String,
    client_id: String,
    client_ip: String,
    #[serde(default)]
    oauth_client_id: Option<String>,
    #[serde(default)]
    redirect_uri: Option<String>,
    #[serde(default)]
    state: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/auth/{realmId}/login/passkey/options",
    tag = "auth",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = PasskeyOptionsRequest,
    responses(
        (status = 200, description = "Passkey login challenge created", body = PasskeyOptionsResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 404, description = "Passkey is not enabled for this realm", body = ErrorResponse),
        (status = 429, description = "Too many requests", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
pub async fn handle_passkey_options(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    ClientIp(client_ip): ClientIp,
    Valid(Json(req)): Valid<Json<PasskeyOptionsRequest>>,
) -> Result<impl IntoResponse, ApiError> {
    verify_turnstile_for_realm(
        &state,
        &realm_id,
        req.turnstile_token.as_deref(),
        Some(&client_ip),
    )
    .await?;
    rate_limit_passkey_start(&state, &client_ip, &req.client_id).await?;
    ensure_passkey_enabled(&state, &realm_id).await?;
    ensure_client_exists(&state, &realm_id, &req.client_id).await?;

    let login_state = PasskeyLoginState {
        realm_id: realm_id.clone(),
        client_id: req.client_id,
        client_ip,
        oauth_client_id: req.oauth.as_ref().map(|oauth| oauth.client_id.clone()),
        redirect_uri: req.oauth.as_ref().map(|oauth| oauth.redirect_uri.clone()),
        state: req.oauth.as_ref().map(|oauth| oauth.state.clone()),
    };

    let service = passkey_service(&state)?;
    let (options, auth_token) = service
        .begin_login_first_factor(&realm_id, login_state)
        .await
        .map_err(map_passkey_begin_error)?;
    let options =
        serde_json::to_value(options).map_err(|_| ApiError::internal("Internal server error"))?;

    Ok(Json(PasskeyOptionsResponse {
        auth_token,
        options,
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/{realmId}/login/passkey/verify",
    tag = "auth",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = PasskeyVerifyRequest,
    responses(
        (status = 200, description = "Passkey login successful", body = PasskeyVerifyResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Passkey verification failed", body = ErrorResponse),
        (status = 429, description = "Too many requests", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
pub async fn handle_passkey_verify(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    ClientIp(client_ip): ClientIp,
    Valid(Json(req)): Valid<Json<PasskeyVerifyRequest>>,
) -> Result<impl IntoResponse, ApiError> {
    rate_limit_hit(
        &state,
        format!("rl:passkey:verify:ip:{client_ip}"),
        TOTP_VERIFY_IP_RATE_LIMIT.0,
        TOTP_VERIFY_IP_RATE_LIMIT.1,
    )
    .await?;

    let service = passkey_service(&state)?;
    let (user_id, _credential, login_state) = service
        .finish_login_first_factor(&req.auth_token, &req.assertion)
        .await
        .map_err(map_passkey_verify_error)?;

    if login_state.realm_id != realm_id {
        return Err(ApiError::unauthorized(PASSKEY_VERIFY_FAILED));
    }

    clear_fail_count(&state, user_id).await?;
    finish_login(
        &state,
        &realm_id,
        user_id,
        login_state,
        req.agreements.as_deref(),
        client_ip,
    )
    .await
}

#[utoipa::path(
    post,
    path = "/api/auth/{realmId}/login/passkey/2fa/options",
    tag = "auth",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = Passkey2faOptionsRequest,
    responses(
        (status = 200, description = "Passkey second-factor challenge created", body = Passkey2faOptionsResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Invalid or expired temporary token", body = ErrorResponse),
        (status = 429, description = "Too many requests", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
pub async fn handle_passkey_2fa_options(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    ClientIp(client_ip): ClientIp,
    Valid(Json(req)): Valid<Json<Passkey2faOptionsRequest>>,
) -> Result<impl IntoResponse, ApiError> {
    let temp_session = load_temp_session(
        &state,
        &req.temp_token,
        "Invalid or expired temporary token",
    )
    .await?;
    if temp_session.realm_id != realm_id {
        return Err(ApiError::unauthorized("Invalid or expired temporary token"));
    }
    rate_limit_passkey_second_factor(&state, &client_ip, &temp_session.user_id).await?;
    let user_id = parse_temp_user_id(&temp_session)?;

    let login_state = temp_session.to_login_state();
    let service = passkey_service(&state)?;
    let (options, auth_token) = service
        .begin_second_factor(&login_state, user_id)
        .await
        .map_err(map_passkey_verify_error)?;
    let options =
        serde_json::to_value(options).map_err(|_| ApiError::internal("Internal server error"))?;

    Ok(Json(Passkey2faOptionsResponse {
        auth_token,
        options,
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/{realmId}/login/passkey/2fa/verify",
    tag = "auth",
    params(
        ("realmId" = String, Path, description = "Realm ID")
    ),
    request_body = Passkey2faVerifyRequest,
    responses(
        (status = 200, description = "Passkey second-factor verification successful", body = PasskeyVerifyResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Passkey verification failed", body = ErrorResponse),
        (status = 429, description = "Too many requests", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
pub async fn handle_passkey_2fa_verify(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    ClientIp(client_ip): ClientIp,
    Valid(Json(req)): Valid<Json<Passkey2faVerifyRequest>>,
) -> Result<impl IntoResponse, ApiError> {
    let temp_session = load_temp_session(&state, &req.temp_token, PASSKEY_VERIFY_FAILED).await?;
    if temp_session.realm_id != realm_id {
        return Err(ApiError::unauthorized(PASSKEY_VERIFY_FAILED));
    }
    rate_limit_passkey_second_factor(&state, &client_ip, &temp_session.user_id).await?;
    let user_id = parse_temp_user_id(&temp_session)?;
    check_fail_count(&state, user_id).await?;

    let service = passkey_service(&state)?;
    let credential = match service
        .finish_second_factor(&req.auth_token, &req.assertion)
        .await
    {
        Ok(credential) => credential,
        Err(err) => {
            record_fail_count(&state, user_id).await?;
            delete_temp_session(&state, &req.temp_token).await?;
            return Err(map_passkey_verify_error(err));
        }
    };

    if credential.user_id != user_id || credential.realm_id != temp_session.realm_id {
        record_fail_count(&state, user_id).await?;
        delete_temp_session(&state, &req.temp_token).await?;
        return Err(ApiError::unauthorized(PASSKEY_VERIFY_FAILED));
    }

    delete_temp_session(&state, &req.temp_token).await?;
    clear_fail_count(&state, user_id).await?;

    finish_login(
        &state,
        &realm_id,
        user_id,
        temp_session.to_login_state(),
        req.agreements.as_deref(),
        client_ip,
    )
    .await
}

impl TempSessionData {
    fn to_login_state(&self) -> PasskeyLoginState {
        PasskeyLoginState {
            realm_id: self.realm_id.clone(),
            client_id: self.client_id.clone(),
            client_ip: self.client_ip.clone(),
            oauth_client_id: self.oauth_client_id.clone(),
            redirect_uri: self.redirect_uri.clone(),
            state: self.state.clone(),
        }
    }
}

async fn finish_login(
    state: &AppState,
    realm_id: &str,
    user_id: Uuid,
    login_state: PasskeyLoginState,
    agreements: Option<&[AuthConsentAgreement]>,
    client_ip: String,
) -> Result<Response, ApiError> {
    let client_app = state
        .service
        .client_service()
        .get_client_app_by_client_id(&login_state.realm_id, &login_state.client_id)
        .await
        .map_err(|_| ApiError::internal("Client app lookup failed"))?;
    let session_ttl = client_app.session_ttl_seconds as usize;
    let renewal_ttl_seconds = renewal_ttl_seconds_from_i32(client_app.session_renewal_ttl_seconds)?;

    let user = state.user_repository.get_user_by_id(user_id).await?;
    if let Some(summaries) = crate::consent_gate::evaluate_login_consent_gate(
        state,
        &user,
        &login_state.realm_id,
        agreements,
        Some(client_ip.clone()),
        None,
    )
    .await
    {
        return Ok(Json(PasskeyVerifyResponse {
            message: "consent required".to_string(),
            user_id: user_id.to_string(),
            token: String::new(),
            expires_in_seconds: 0,
            redirect_to: None,
            consent_required: Some(true),
            agreements: Some(summaries),
        })
        .into_response());
    }

    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Redis connection error"))?;

    let has_oauth = login_state.oauth_client_id.is_some()
        && login_state.redirect_uri.is_some()
        && login_state.state.is_some();
    if has_oauth {
        let oauth_client_id = login_state.oauth_client_id.as_ref().expect("checked above");
        let redirect_uri = login_state.redirect_uri.as_ref().expect("checked above");
        let state_param = login_state.state.as_ref().expect("checked above");

        let state_key = format!("oauth:state:{state_param}");
        let state_json: Option<String> = redis::cmd("GETDEL")
            .arg(&state_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Redis GETDEL failed for OAuth state");
                ApiError::internal("Redis operation error")
            })?;

        let state_json = state_json.ok_or_else(|| {
            ApiError::bad_request(
                "OAuth state not found or already used. Please restart the authorization flow.",
            )
        })?;

        let state_data: serde_json::Value = serde_json::from_str(&state_json).map_err(|e| {
            tracing::error!(error = %e, "Failed to parse OAuth state JSON");
            ApiError::internal("Redis operation error")
        })?;

        let stored_client_id = state_data["client_id"].as_str().unwrap_or("");
        let stored_realm_id = state_data["realm_id"].as_str().unwrap_or("");
        let stored_redirect_uri = state_data["redirect_uri"].as_str().unwrap_or("");

        if stored_client_id != oauth_client_id {
            return Err(ApiError::bad_request("OAuth state client_id mismatch"));
        }
        if stored_realm_id != login_state.realm_id {
            return Err(ApiError::bad_request("OAuth state realm_id mismatch"));
        }
        if stored_redirect_uri != redirect_uri {
            return Err(ApiError::bad_request("OAuth state redirect_uri mismatch"));
        }

        let auth_code = format!("ac_{}", Uuid::now_v7());
        let code_challenge = state_data["code_challenge"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let code_key = format!("oauth:code:{auth_code}");
        let code_value = serde_json::json!({
            "code_challenge": code_challenge,
            "client_id": oauth_client_id,
            "redirect_uri": redirect_uri,
            "user_id": user_id.to_string(),
            "realm_id": login_state.realm_id,
        })
        .to_string();

        let _: () = conn
            .set_ex(&code_key, code_value, OAUTH_STATE_TTL_SECONDS)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to store OAuth authorization code");
                ApiError::internal("Redis operation error")
            })?;

        let redirect_to = format!("{redirect_uri}?code={auth_code}&state={state_param}");
        return Ok(Json(PasskeyVerifyResponse {
            message: "ok".to_string(),
            user_id: user_id.to_string(),
            token: String::new(),
            expires_in_seconds: 0,
            redirect_to: Some(redirect_to),
            consent_required: None,
            agreements: None,
        })
        .into_response());
    }

    let token = format!("{}_{}_{}", realm_id, user_id, epoch_seconds());
    let session_data = SessionData {
        realm_id: login_state.realm_id,
        client_id: login_state.client_id,
        user_id: user_id.to_string(),
        client_ip,
        renewal_ttl_seconds,
    };

    store_session(state, &token, &session_data, session_ttl).await?;

    let cookie_header = build_set_cookie(
        "X-Auth",
        &token,
        session_ttl as i64,
        state.app_env == "production",
    );
    Ok((
        [(SET_COOKIE, cookie_header)],
        Json(PasskeyVerifyResponse {
            message: "ok".to_string(),
            user_id: user_id.to_string(),
            token,
            expires_in_seconds: session_ttl as i64,
            redirect_to: None,
            consent_required: None,
            agreements: None,
        }),
    )
        .into_response())
}

async fn ensure_client_exists(
    state: &AppState,
    realm_id: &str,
    client_id: &str,
) -> Result<(), ApiError> {
    state
        .service
        .client_service()
        .get_client_app_by_client_id(realm_id, client_id)
        .await
        .map(|_| ())
        .map_err(|_| ApiError::bad_request("Client app not found"))
}

async fn ensure_passkey_enabled(state: &AppState, realm_id: &str) -> Result<(), ApiError> {
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT config_value FROM realm_config
         WHERE realm_id = $1 AND config_type = $2 AND config_key = 'settings' AND enabled = true",
    )
    .bind(realm_id)
    .bind(ConfigType::Passkey.as_ref())
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query passkey realm config: {e}");
        ApiError::internal("Internal server error")
    })?;

    let enabled = row
        .and_then(|(value,)| serde_json::from_str::<serde_json::Value>(&value).ok())
        .and_then(|value| value.get("enabled").and_then(|enabled| enabled.as_bool()))
        .unwrap_or(false);

    if !enabled {
        return Err(ApiError::not_found("Passkey is not enabled for this realm"));
    }

    Ok(())
}

async fn load_temp_session(
    state: &AppState,
    temp_token: &str,
    unauthorized_message: &'static str,
) -> Result<TempSessionData, ApiError> {
    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Redis connection error"))?;
    let temp_key = temp_session_key(temp_token);
    let temp_session_json: Option<String> = conn.get(&temp_key).await.map_err(|e| {
        tracing::error!("Failed to get temp session from Redis: {}", e);
        ApiError::internal("Redis operation error")
    })?;
    let temp_session_json =
        temp_session_json.ok_or_else(|| ApiError::unauthorized(unauthorized_message))?;

    serde_json::from_str(&temp_session_json).map_err(|e| {
        tracing::error!("Failed to parse temp session JSON: {}", e);
        ApiError::internal("Redis operation error")
    })
}

async fn delete_temp_session(state: &AppState, temp_token: &str) -> Result<(), ApiError> {
    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Redis connection error"))?;
    let _: () = conn.del(temp_session_key(temp_token)).await.map_err(|e| {
        tracing::error!("Failed to delete temp session: {}", e);
        ApiError::internal("Redis operation error")
    })?;
    Ok(())
}

fn temp_session_key(temp_token: &str) -> String {
    format!("totp:temp:{temp_token}")
}

fn parse_temp_user_id(temp_session: &TempSessionData) -> Result<Uuid, ApiError> {
    Uuid::parse_str(&temp_session.user_id).map_err(|e| {
        tracing::error!(
            user_id = %temp_session.user_id,
            error = %e,
            "Invalid user_id format in temp session"
        );
        ApiError::internal("Redis operation error")
    })
}

async fn rate_limit_passkey_start(
    state: &AppState,
    client_ip: &str,
    client_id: &str,
) -> Result<(), ApiError> {
    rate_limit_hit(
        state,
        format!("rl:passkey:ip:{client_ip}"),
        LOGIN_IP_RATE_LIMIT.0,
        LOGIN_IP_RATE_LIMIT.1,
    )
    .await?;
    rate_limit_hit(
        state,
        format!("rl:passkey:identifier:{client_id}"),
        LOGIN_IDENTIFIER_RATE_LIMIT.0,
        LOGIN_IDENTIFIER_RATE_LIMIT.1,
    )
    .await
}

async fn rate_limit_passkey_second_factor(
    state: &AppState,
    client_ip: &str,
    user_id: &str,
) -> Result<(), ApiError> {
    rate_limit_hit(
        state,
        format!("rl:passkey:2fa:ip:{client_ip}"),
        TOTP_VERIFY_IP_RATE_LIMIT.0,
        TOTP_VERIFY_IP_RATE_LIMIT.1,
    )
    .await?;
    rate_limit_hit(
        state,
        format!("rl:passkey:2fa:user:{user_id}"),
        TOTP_VERIFY_USER_RATE_LIMIT.0,
        TOTP_VERIFY_USER_RATE_LIMIT.1,
    )
    .await
}

async fn check_fail_count(state: &AppState, user_id: Uuid) -> Result<(), ApiError> {
    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Redis connection error"))?;
    let fail_count: Option<i64> = conn.get(fail_count_key(user_id)).await.map_err(|e| {
        tracing::error!("Failed to get passkey fail count from Redis: {}", e);
        ApiError::internal("Redis operation error")
    })?;

    if let Some(count) = fail_count
        && count >= TOTP_MAX_FAILURES
    {
        return Err(ApiError::too_many_requests(
            "Too many failed attempts. Please try again in 15 minutes.",
        ));
    }

    Ok(())
}

async fn record_fail_count(state: &AppState, user_id: Uuid) -> Result<(), ApiError> {
    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Redis connection error"))?;
    let key = fail_count_key(user_id);
    let _: () = conn.incr(&key, 1).await.map_err(|e| {
        tracing::error!("Failed to increment passkey fail count: {}", e);
        ApiError::internal("Redis operation error")
    })?;
    let _: () = conn
        .expire(&key, TOTP_LOCKOUT_SECONDS as i64)
        .await
        .map_err(|e| {
            tracing::error!("Failed to set passkey fail count expiry: {}", e);
            ApiError::internal("Redis operation error")
        })?;
    Ok(())
}

async fn clear_fail_count(state: &AppState, user_id: Uuid) -> Result<(), ApiError> {
    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Redis connection error"))?;
    let _: () = conn.del(fail_count_key(user_id)).await.map_err(|e| {
        tracing::error!("Failed to delete passkey fail count: {}", e);
        ApiError::internal("Redis operation error")
    })?;
    Ok(())
}

fn fail_count_key(user_id: Uuid) -> String {
    format!("passkey:fail_count:{user_id}")
}

fn passkey_service(
    state: &AppState,
) -> Result<UserPasskeyService<PostgresUserPasskeyRepository, RedisPasskeyChallengeStore>, ApiError>
{
    let rp_id =
        std::env::var("RP_ID").map_err(|_| ApiError::internal("RP_ID is not configured"))?;
    let rp_origin = std::env::var("RP_ORIGIN")
        .map_err(|_| ApiError::internal("RP_ORIGIN is not configured"))?;
    let repo = Arc::new(PostgresUserPasskeyRepository::new(state.db.clone()));
    let challenge_store = Arc::new(RedisPasskeyChallengeStore::new(state.redis_manager.clone()));

    UserPasskeyService::new(&rp_id, &rp_origin, repo, challenge_store)
        .map_err(map_passkey_setup_error)
}

fn map_passkey_begin_error(err: PasskeyError) -> ApiError {
    match err {
        PasskeyError::Disabled => ApiError::not_found("Passkey is not enabled for this realm"),
        PasskeyError::NotFound => ApiError::unauthorized(PASSKEY_VERIFY_FAILED),
        PasskeyError::Unsupported => ApiError::bad_request("Passkey is unsupported"),
        other => map_passkey_setup_error(other),
    }
}

fn map_passkey_verify_error(err: PasskeyError) -> ApiError {
    match err {
        PasskeyError::Repo(_) => {
            tracing::debug!(error = %err, "Passkey verification failed");
            ApiError::unauthorized(PASSKEY_VERIFY_FAILED)
        }
        PasskeyError::Disabled
        | PasskeyError::VerificationFailed
        | PasskeyError::NotFound
        | PasskeyError::ChallengeExpired
        | PasskeyError::Unsupported => ApiError::unauthorized(PASSKEY_VERIFY_FAILED),
    }
}

fn map_passkey_setup_error(err: PasskeyError) -> ApiError {
    match err {
        PasskeyError::Disabled => ApiError::not_found("Passkey is not enabled for this realm"),
        PasskeyError::Unsupported => ApiError::bad_request("Passkey is unsupported"),
        PasskeyError::Repo(err) => ApiError::from(err),
        PasskeyError::VerificationFailed
        | PasskeyError::ChallengeExpired
        | PasskeyError::NotFound => ApiError::unauthorized(PASSKEY_VERIFY_FAILED),
    }
}
