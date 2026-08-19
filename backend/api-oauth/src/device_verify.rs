// Device verify endpoint (user enters user_code on verification page)
//
// The user visits the verification page, enters their user_code, and this
// endpoint validates the code, marks the device state as "verified", and
// returns the client app name/icon for the confirmation page.

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use herald_api_base::application::http::rate_limit::rate_limit_hit;
use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::security_constants::DEVICE_CODE_USER_CODE_ALPHABET;

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct DeviceVerifyErrorResponse {
    pub error: String,
    pub error_description: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DeviceVerifyRequest {
    pub user_code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeviceVerifyResponse {
    pub client_app_name: String,
    pub client_app_icon_url: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Validate user_code format: XXXX-XXXX using BCDFGHJKMNPQRSTVWXYZ alphabet.
fn validate_user_code(code: &str) -> Result<(), ApiError> {
    // Format: 4 chars, dash, 4 chars = 9 total
    if code.len() != 9 {
        return Err(ApiError::bad_request_json(DeviceVerifyErrorResponse {
            error: "invalid_request".to_string(),
            error_description: "user_code must be in XXXX-XXXX format".to_string(),
        }));
    }

    let bytes = code.as_bytes();
    if bytes[4] != b'-' {
        return Err(ApiError::bad_request_json(DeviceVerifyErrorResponse {
            error: "invalid_request".to_string(),
            error_description: "user_code must be in XXXX-XXXX format".to_string(),
        }));
    }

    let valid_chars: std::collections::HashSet<u8> =
        DEVICE_CODE_USER_CODE_ALPHABET.bytes().collect();
    for (i, &b) in bytes.iter().enumerate() {
        if i == 4 {
            continue;
        }
        if !valid_chars.contains(&b) {
            return Err(ApiError::bad_request_json(DeviceVerifyErrorResponse {
                error: "invalid_request".to_string(),
                error_description: format!(
                    "user_code contains invalid character '{}' at position {}",
                    b as char, i
                ),
            }));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/device/{realmId}/verify",
    tag = "device",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
    ),
    request_body = DeviceVerifyRequest,
    responses(
        (status = 200, description = "Device code verified", body = DeviceVerifyResponse),
        (status = 400, description = "Invalid request", body = DeviceVerifyErrorResponse),
        (status = 404, description = "Not found", body = DeviceVerifyErrorResponse),
        (status = 409, description = "Conflict", body = DeviceVerifyErrorResponse),
    )
)]
pub async fn device_verify(
    Path(realm_id): Path<String>,
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Json(payload): Json<DeviceVerifyRequest>,
) -> Result<Json<DeviceVerifyResponse>, ApiError> {
    if identity.realm_id() != realm_id {
        return Err(ApiError::forbidden(
            "Access denied: cannot verify device code for a different realm",
        ));
    }

    // Brute-force protection: guessing a live user_code binds the guesser to
    // the device grant (the code becomes verified as this user), so attempts
    // must be throttled per authenticated user.
    rate_limit_hit(
        &state,
        format!("rl:device-verify:user:{}", identity.user_id()),
        20,
        300,
    )
    .await?;

    let user_code = payload.user_code.to_uppercase();

    // Validate user_code format
    validate_user_code(&user_code)?;

    let mut conn = state
        .redis_manager
        .get()
        .await
        .map_err(|_| ApiError::internal("Internal server error"))?;

    // Lookup device_code from user_code index
    let user_code_key = format!("deviceUserCode:{}", user_code);
    let device_code: Option<String> = conn.get(&user_code_key).await.map_err(|e| {
        tracing::error!(error = %e, "Redis GET failed: user code lookup");
        ApiError::internal("Internal server error")
    })?;

    let Some(device_code) = device_code else {
        return Err(ApiError::with_json(
            axum::http::StatusCode::NOT_FOUND,
            DeviceVerifyErrorResponse {
                error: "not_found".to_string(),
                error_description: "Device code not found or expired".to_string(),
            },
        ));
    };

    // Lookup device state
    let device_key = format!("device:{}", device_code);
    let state_json: Option<String> = conn.get(&device_key).await.map_err(|e| {
        tracing::error!(error = %e, "Redis GET failed: device state lookup");
        ApiError::internal("Internal server error")
    })?;

    let Some(state_json) = state_json else {
        return Err(ApiError::with_json(
            axum::http::StatusCode::NOT_FOUND,
            DeviceVerifyErrorResponse {
                error: "not_found".to_string(),
                error_description: "Device code not found or expired".to_string(),
            },
        ));
    };

    let mut device_state: serde_json::Value = serde_json::from_str(&state_json).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse device state JSON");
        ApiError::internal("Internal server error")
    })?;

    // Validate realm isolation
    let stored_realm_id = device_state
        .get("realm_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if stored_realm_id != realm_id {
        return Err(ApiError::bad_request_json(DeviceVerifyErrorResponse {
            error: "invalid_request".to_string(),
            error_description: "Realm mismatch".to_string(),
        }));
    }

    let status = device_state
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let stored_user_id = device_state
        .get("user_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match status {
        // Terminal states: already confirmed/denied/consumed
        "denied" | "consumed" | "authorized" => {
            return Err(ApiError::conflict_json(DeviceVerifyErrorResponse {
                error: "already_confirmed".to_string(),
                error_description: "Device code has already been confirmed or denied".to_string(),
            }));
        }
        // Idempotent: same user re-verifying
        "verified" if stored_user_id == identity.user_id() => {
            // Fall through to query client app and return
        }
        // Different user already verified this code
        "verified" => {
            return Err(ApiError::conflict_json(DeviceVerifyErrorResponse {
                error: "already_used".to_string(),
                error_description: "Device code has already been used by another user".to_string(),
            }));
        }
        // Fresh pending code: verify it
        "pending" => {
            device_state["status"] = serde_json::Value::String("verified".to_string());
            device_state["user_id"] = serde_json::Value::String(identity.user_id());

            let updated_json = serde_json::to_string(&device_state).map_err(|e| {
                tracing::error!(error = %e, "Failed to serialize device state");
                ApiError::internal("Internal server error")
            })?;

            // Write back preserving TTL
            redis::cmd("SET")
                .arg(&device_key)
                .arg(&updated_json)
                .arg("KEEPTTL")
                .query_async::<String>(&mut conn)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "Redis SET KEEPTTL failed");
                    ApiError::internal("Internal server error")
                })?;
        }
        _ => {
            return Err(ApiError::with_json(
                axum::http::StatusCode::NOT_FOUND,
                DeviceVerifyErrorResponse {
                    error: "not_found".to_string(),
                    error_description: "Device code not found or expired".to_string(),
                },
            ));
        }
    }

    // Query client app for name and icon_url
    let client_id = device_state
        .get("client_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let client_row = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT name, icon_url FROM client_app WHERE realm_id = $1 AND client_id = $2",
    )
    .bind(&realm_id)
    .bind(client_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(
            realm_id = %realm_id,
            client_id = %client_id,
            error = %e,
            "Database query failed: client_app lookup"
        );
        ApiError::internal("Database query failed")
    })?;

    let Some((client_app_name, client_app_icon_url)) = client_row else {
        return Err(ApiError::with_json(
            axum::http::StatusCode::NOT_FOUND,
            DeviceVerifyErrorResponse {
                error: "not_found".to_string(),
                error_description: "Client app not found".to_string(),
            },
        ));
    };

    Ok(Json(DeviceVerifyResponse {
        client_app_name,
        client_app_icon_url,
    }))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_user_code_valid() {
        assert!(validate_user_code("BCDF-BDFG").is_ok());
    }

    #[test]
    fn test_validate_user_code_too_short() {
        let result = validate_user_code("BCD-BDF");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_user_code_no_dash() {
        let result = validate_user_code("BCDFBCDF");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_user_code_invalid_char() {
        let result = validate_user_code("ABCD-EFGH");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_user_code_lowercase() {
        let result = validate_user_code("bcdf-bcgh");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_user_code_dash_at_wrong_position() {
        let result = validate_user_code("BCD-FBCD");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_user_code_all_valid_chars() {
        // Every character from the alphabet should work
        for c in DEVICE_CODE_USER_CODE_ALPHABET.chars() {
            let code = format!(
                "{cccc}-{cccc}",
                cccc = std::iter::repeat_n(c, 4).collect::<String>()
            );
            assert!(validate_user_code(&code).is_ok(), "Failed for char: {}", c);
        }
    }
}
