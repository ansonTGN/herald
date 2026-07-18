use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use herald_api_base::application::http::server::api_entities::{ApiError, ApiResult};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::{BrowserTokenService, RefreshError};
use herald_core::infrastructure::authentication::RedisBrowserTokenService;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrowserTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub refresh_expires_in: u64,
    pub token_type: String,
}

impl From<herald_core::domain::authentication::BrowserTokenSet> for BrowserTokenResponse {
    fn from(tokens: herald_core::domain::authentication::BrowserTokenSet) -> Self {
        Self {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_in: tokens.expires_in,
            refresh_expires_in: tokens.refresh_expires_in,
            token_type: tokens.token_type,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefreshBrowserTokenRequest {
    pub refresh_token: String,
}

#[utoipa::path(
    post,
    path = "/api/auth/browser-token/refresh",
    tag = "auth",
    request_body = RefreshBrowserTokenRequest,
    responses((status = 200, body = BrowserTokenResponse), (status = 401))
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(request): Json<RefreshBrowserTokenRequest>,
) -> Result<ApiResult<BrowserTokenResponse>, ApiError> {
    let service = RedisBrowserTokenService::new(state.redis_manager.clone());
    let tokens = service
        .refresh(&request.refresh_token)
        .await
        .map_err(map_refresh_error)?;
    Ok(ApiResult::ok(tokens.into()))
}

fn map_refresh_error(error: RefreshError) -> ApiError {
    match error {
        RefreshError::Invalid | RefreshError::ReuseDetected => {
            ApiError::unauthorized("invalid refresh token")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn browser_token_response_uses_bearer_contract_fields() {
        let value = serde_json::to_value(BrowserTokenResponse {
            access_token: "access".into(),
            refresh_token: "refresh".into(),
            expires_in: 900,
            refresh_expires_in: 3600,
            token_type: "Bearer".into(),
        })
        .unwrap();
        assert_eq!(value["accessToken"], "access");
        assert_eq!(value["refreshToken"], "refresh");
        assert_eq!(value["tokenType"], "Bearer");
    }

    #[test]
    fn browser_token_refresh_errors_are_all_unauthorized() {
        for error in [RefreshError::Invalid, RefreshError::ReuseDetected] {
            assert_eq!(
                map_refresh_error(error).into_response().status(),
                axum::http::StatusCode::UNAUTHORIZED
            );
        }
    }
}
