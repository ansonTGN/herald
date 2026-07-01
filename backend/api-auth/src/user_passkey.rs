use axum::{
    Json,
    extract::{Extension, Path, State},
};
use axum_valid::Valid;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use herald_api_base::application::http::auth::util::rate_limit_hit;
pub use herald_api_base::application::http::server::api_entities::ErrorResponse;
use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::domain::realm_config::ConfigType;
use herald_core::domain::user::ports::UserRepository;
use herald_core::domain::user_passkey::{
    PasskeyCredentialView, PasskeyError, UserPasskeyRepository, UserPasskeyService,
};
use herald_core::infrastructure::user::repositories::PostgresUserRepository;
use herald_core::infrastructure::user_passkey::{
    PostgresUserPasskeyRepository, RedisPasskeyChallengeStore,
};

const PASSKEY_USER_RATE_LIMIT: (i64, usize) = (5, 60);
const PASSKEY_CHALLENGE_TTL_SECONDS: u64 = 300;

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/passkey/registration/begin",
            axum::routing::post(handle_begin_passkey_registration),
        )
        .route(
            "/passkey/registration/finish",
            axum::routing::post(handle_finish_passkey_registration),
        )
        .route(
            "/passkey/credentials",
            axum::routing::get(handle_list_passkey_credentials),
        )
        .route(
            "/passkey/credentials/{credentialId}",
            axum::routing::patch(handle_rename_passkey_credential)
                .delete(handle_delete_passkey_credential),
        )
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct BeginRegistrationRequest {
    pub password: String,
    pub nickname: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BeginRegistrationResponse {
    pub reg_token: String,
    pub options: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct FinishRegistrationRequest {
    pub reg_token: String,
    pub attestation: serde_json::Value,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FinishRegistrationResponse {
    pub credential_id: String,
    pub nickname: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyCredentialViewResponse {
    pub credential_id: String,
    pub nickname: Option<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub backup_eligible: bool,
    pub backup_state: bool,
    pub transports: Vec<String>,
    pub aaguid: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListPasskeysResponse {
    pub credentials: Vec<PasskeyCredentialViewResponse>,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RenamePasskeyRequest {
    #[validate(length(min = 1, max = 128))]
    pub nickname: String,
}

#[utoipa::path(
    post,
    path = "/api/user/passkey/registration/begin",
    tag = "user",
    request_body = BeginRegistrationRequest,
    responses(
        (status = 200, description = "Passkey registration challenge created", body = BeginRegistrationResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Passkey is not enabled for this realm", body = ErrorResponse),
        (status = 429, description = "Too many requests", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_begin_passkey_registration(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Valid(Json(req)): Valid<Json<BeginRegistrationRequest>>,
) -> Result<ApiResult<BeginRegistrationResponse>, ApiError> {
    let user_id = identity_user_id(&identity)?;
    rate_limit_passkey_user(&state, user_id).await?;

    let user_repo = PostgresUserRepository::new(state.db.clone());
    let user = user_repo.get_user_by_id(user_id).await?;
    ensure_passkey_enabled(&state, &user.realm_id).await?;

    let password_hash = user.password_hash.as_ref().ok_or_else(|| {
        ApiError::bad_request("Password authentication not enabled for this account")
    })?;
    let password_valid = bcrypt::verify(&req.password, password_hash)
        .map_err(|_| ApiError::internal("Internal server error"))?;
    if !password_valid {
        return Err(ApiError::unauthorized("Invalid password"));
    }

    let repo = Arc::new(PostgresUserPasskeyRepository::new(state.db.clone()));
    let existing = repo.list_by_user(&user.realm_id, user_id).await?;
    let exclude = existing
        .iter()
        .map(|credential| credential.credential_id.clone())
        .collect::<Vec<_>>();
    let service = passkey_service(&state, repo)?;
    let (options, reg_token) = service
        .begin_registration(&user.realm_id, &user, &exclude)
        .await
        .map_err(map_registration_begin_error)?;
    store_registration_nickname(&state, &reg_token, req.nickname.as_deref()).await?;
    let options =
        serde_json::to_value(options).map_err(|_| ApiError::internal("Internal server error"))?;

    Ok(ApiResult::ok(BeginRegistrationResponse {
        reg_token,
        options,
    }))
}

#[utoipa::path(
    post,
    path = "/api/user/passkey/registration/finish",
    tag = "user",
    request_body = FinishRegistrationRequest,
    responses(
        (status = 200, description = "Passkey registration finished", body = FinishRegistrationResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Invalid or expired registration token", body = ErrorResponse),
        (status = 409, description = "Credential already exists", body = ErrorResponse),
        (status = 422, description = "Attestation verification failed", body = ErrorResponse),
        (status = 429, description = "Too many requests", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_finish_passkey_registration(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Valid(Json(req)): Valid<Json<FinishRegistrationRequest>>,
) -> Result<ApiResult<FinishRegistrationResponse>, ApiError> {
    let user_id = identity_user_id(&identity)?;
    rate_limit_passkey_user(&state, user_id).await?;

    let repo = Arc::new(PostgresUserPasskeyRepository::new(state.db.clone()));
    let service = passkey_service(&state, repo)?;
    let nickname = load_registration_nickname(&state, &req.reg_token).await?;
    let credential = service
        .finish_registration(&req.reg_token, &req.attestation, nickname.as_deref())
        .await
        .map_err(map_registration_finish_error)?;

    if credential.user_id != user_id || credential.realm_id != identity.realm_id() {
        return Err(ApiError::forbidden(
            "passkey credential does not belong to user in realm",
        ));
    }

    Ok(ApiResult::ok(FinishRegistrationResponse {
        credential_id: credential.id.to_string(),
        nickname: credential.nickname,
        created_at: credential.created_at.to_rfc3339(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/user/passkey/credentials",
    tag = "user",
    responses(
        (status = 200, description = "Passkey credentials retrieved", body = ListPasskeysResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 429, description = "Too many requests", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_list_passkey_credentials(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
) -> Result<ApiResult<ListPasskeysResponse>, ApiError> {
    let user_id = identity_user_id(&identity)?;
    rate_limit_passkey_user(&state, user_id).await?;

    let repo = PostgresUserPasskeyRepository::new(state.db.clone());
    let credentials = repo
        .list_by_user(&identity.realm_id(), user_id)
        .await?
        .into_iter()
        .map(|credential| PasskeyCredentialViewResponse::from(credential.to_view()))
        .collect();

    Ok(ApiResult::ok(ListPasskeysResponse { credentials }))
}

#[utoipa::path(
    patch,
    path = "/api/user/passkey/credentials/{credentialId}",
    tag = "user",
    params(
        ("credentialId" = String, Path, description = "Passkey credential UUID")
    ),
    request_body = RenamePasskeyRequest,
    responses(
        (status = 204, description = "Passkey credential renamed"),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Credential not found", body = ErrorResponse),
        (status = 429, description = "Too many requests", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_rename_passkey_credential(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(credential_id): Path<String>,
    Valid(Json(req)): Valid<Json<RenamePasskeyRequest>>,
) -> Result<ApiResult<()>, ApiError> {
    let user_id = identity_user_id(&identity)?;
    rate_limit_passkey_user(&state, user_id).await?;
    let credential_id = Uuid::parse_str(&credential_id)
        .map_err(|_| ApiError::bad_request("Invalid credentialId"))?;

    let repo = PostgresUserPasskeyRepository::new(state.db.clone());
    repo.rename(&identity.realm_id(), user_id, credential_id, &req.nickname)
        .await
        .map_err(map_repository_error)?;

    Ok(ApiResult::no_content())
}

#[utoipa::path(
    delete,
    path = "/api/user/passkey/credentials/{credentialId}",
    tag = "user",
    params(
        ("credentialId" = String, Path, description = "Passkey credential UUID")
    ),
    responses(
        (status = 204, description = "Passkey credential deleted"),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Credential not found", body = ErrorResponse),
        (status = 429, description = "Too many requests", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_delete_passkey_credential(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(credential_id): Path<String>,
) -> Result<ApiResult<()>, ApiError> {
    let user_id = identity_user_id(&identity)?;
    rate_limit_passkey_user(&state, user_id).await?;
    let credential_id = Uuid::parse_str(&credential_id)
        .map_err(|_| ApiError::bad_request("Invalid credentialId"))?;

    let repo = PostgresUserPasskeyRepository::new(state.db.clone());
    repo.delete(&identity.realm_id(), user_id, credential_id)
        .await
        .map_err(map_repository_error)?;

    Ok(ApiResult::no_content())
}

impl From<PasskeyCredentialView> for PasskeyCredentialViewResponse {
    fn from(view: PasskeyCredentialView) -> Self {
        Self {
            credential_id: view.id.to_string(),
            nickname: view.nickname,
            created_at: view.created_at.to_rfc3339(),
            last_used_at: view.last_used_at.map(|dt| dt.to_rfc3339()),
            backup_eligible: view.backup_eligible,
            backup_state: view.backup_state,
            transports: view.transports,
            aaguid: view.aaguid.map(|id| id.to_string()),
        }
    }
}

fn identity_user_id(identity: &Identity) -> Result<Uuid, ApiError> {
    Uuid::parse_str(&identity.user_id()).map_err(|e| {
        tracing::error!("Invalid user_id format in identity: {}", e);
        ApiError::internal("Invalid user_id format")
    })
}

async fn rate_limit_passkey_user(state: &AppState, user_id: Uuid) -> Result<(), ApiError> {
    rate_limit_hit(
        state,
        format!("rl:passkey:user:{user_id}"),
        PASSKEY_USER_RATE_LIMIT.0,
        PASSKEY_USER_RATE_LIMIT.1,
    )
    .await
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

async fn store_registration_nickname(
    state: &AppState,
    reg_token: &str,
    nickname: Option<&str>,
) -> Result<(), ApiError> {
    let Some(nickname) = nickname else {
        return Ok(());
    };
    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;
    let _: () = conn
        .set_ex(
            registration_nickname_key(reg_token),
            nickname,
            PASSKEY_CHALLENGE_TTL_SECONDS,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to store passkey registration nickname: {}", e);
            ApiError::internal("Internal server error")
        })?;

    Ok(())
}

async fn load_registration_nickname(
    state: &AppState,
    reg_token: &str,
) -> Result<Option<String>, ApiError> {
    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;
    let key = registration_nickname_key(reg_token);
    let nickname: Option<String> = conn.get(&key).await.map_err(|e| {
        tracing::error!("Failed to load passkey registration nickname: {}", e);
        ApiError::internal("Internal server error")
    })?;
    let _: () = conn.del(&key).await.map_err(|e| {
        tracing::error!("Failed to delete passkey registration nickname: {}", e);
        ApiError::internal("Internal server error")
    })?;

    Ok(nickname)
}

fn registration_nickname_key(reg_token: &str) -> String {
    format!("passkey:reg:nickname:{reg_token}")
}

fn passkey_service(
    state: &AppState,
    repo: Arc<PostgresUserPasskeyRepository>,
) -> Result<UserPasskeyService<PostgresUserPasskeyRepository, RedisPasskeyChallengeStore>, ApiError>
{
    let rp_id =
        std::env::var("RP_ID").map_err(|_| ApiError::internal("RP_ID is not configured"))?;
    let rp_origin = std::env::var("RP_ORIGIN")
        .map_err(|_| ApiError::internal("RP_ORIGIN is not configured"))?;
    let challenge_store = Arc::new(RedisPasskeyChallengeStore::new(state.redis_manager.clone()));

    UserPasskeyService::new(&rp_id, &rp_origin, repo, challenge_store).map_err(map_passkey_error)
}

fn map_registration_begin_error(err: PasskeyError) -> ApiError {
    match err {
        PasskeyError::Disabled => ApiError::not_found("Passkey is not enabled for this realm"),
        other => map_passkey_error(other),
    }
}

fn map_registration_finish_error(err: PasskeyError) -> ApiError {
    match err {
        PasskeyError::ChallengeExpired => {
            ApiError::unauthorized("Invalid or expired registration token")
        }
        PasskeyError::VerificationFailed | PasskeyError::Unsupported => {
            ApiError::unprocessable_entity("Passkey verification failed")
        }
        PasskeyError::Repo(CoreError::Conflict(_)) => {
            ApiError::conflict("Passkey credential already exists")
        }
        PasskeyError::Repo(CoreError::DatabaseError(msg))
            if msg.to_ascii_lowercase().contains("unique") =>
        {
            ApiError::conflict("Passkey credential already exists")
        }
        other => map_passkey_error(other),
    }
}

fn map_passkey_error(err: PasskeyError) -> ApiError {
    match err {
        PasskeyError::Disabled => ApiError::not_found("Passkey is not enabled for this realm"),
        PasskeyError::VerificationFailed => ApiError::unauthorized("Passkey verification failed"),
        PasskeyError::NotFound => ApiError::not_found("Passkey credential not found"),
        PasskeyError::ChallengeExpired => ApiError::unauthorized("Challenge expired"),
        PasskeyError::Unsupported => ApiError::unprocessable_entity("Passkey is unsupported"),
        PasskeyError::Repo(err) => map_repository_error(err),
    }
}

fn map_repository_error(err: CoreError) -> ApiError {
    match err {
        CoreError::Forbidden(msg) => ApiError::forbidden(msg),
        CoreError::NotFound => ApiError::not_found("Passkey credential not found"),
        other => ApiError::from(other),
    }
}
