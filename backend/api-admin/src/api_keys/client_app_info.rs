use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::client_api_keys::constants::ADMIN_API_CLIENT_ID;
use herald_core::entity::client_app;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

pub async fn resolve_client_app_for_create(
    state: &AppState,
    realm_id: &str,
    client_app_id: Option<Uuid>,
) -> Result<client_app::Model, ApiError> {
    let query = client_app::Entity::find().filter(client_app::Column::RealmId.eq(realm_id));
    let app = match client_app_id {
        Some(id) => {
            query
                .filter(client_app::Column::Id.eq(id))
                .one(state.db.as_ref())
                .await
        }
        None => {
            query
                .filter(client_app::Column::ClientId.eq(ADMIN_API_CLIENT_ID))
                .one(state.db.as_ref())
                .await
        }
    }
    .map_err(|e| {
        tracing::error!("Failed to query Client App for API key: {e}");
        ApiError::internal("Failed to create API key")
    })?;

    app.ok_or_else(|| {
        if client_app_id.is_some() {
            ApiError::bad_request("Client App not found in this realm")
        } else {
            ApiError::bad_request(
                "Realm is missing the built-in API Key Client App. Please contact support.",
            )
        }
    })
}

pub async fn client_app_name(
    state: &AppState,
    client_app_id: Option<Uuid>,
) -> Result<Option<String>, ApiError> {
    let Some(id) = client_app_id else {
        return Ok(None);
    };

    let app = client_app::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query API key Client App name: {e}");
            ApiError::internal("Failed to load API key Client App")
        })?;

    Ok(app.map(|app| app.name))
}
