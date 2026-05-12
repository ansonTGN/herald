use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

pub use herald_api_base::application::http::server::api_entities::ErrorResponse;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct PermissionCreateRequest {
    #[validate(length(min = 1, max = 36))]
    pub name: String,
    #[validate(length(max = 255))]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct PermissionUpdateRequest {
    #[validate(length(min = 1, max = 36))]
    pub name: String,
    #[validate(length(max = 255))]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PermissionResponse {
    pub id: Uuid,
    pub name: String,
    pub resource: String,
    pub action: String,
    pub description: Option<String>,
    pub realm_id: String,
    pub is_builtin: bool,
}
