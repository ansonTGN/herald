use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub enum ApiResult<T> {
    Ok(T),
    Created(T),
    Accepted(T),
    NoContent,
}

impl<T> ApiResult<T> {
    pub fn ok(data: T) -> Self {
        Self::Ok(data)
    }

    pub fn created(data: T) -> Self {
        Self::Created(data)
    }

    pub fn accepted(data: T) -> Self {
        Self::Accepted(data)
    }

    pub fn no_content() -> Self {
        Self::NoContent
    }
}

impl<T> IntoResponse for ApiResult<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match self {
            Self::Ok(body) => (StatusCode::OK, Json(body)).into_response(),
            Self::Created(body) => (StatusCode::CREATED, Json(body)).into_response(),
            Self::Accepted(body) => (StatusCode::ACCEPTED, Json(body)).into_response(),
            Self::NoContent => StatusCode::NO_CONTENT.into_response(),
        }
    }
}
