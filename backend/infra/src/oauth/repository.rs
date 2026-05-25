// OAuth provider repository implementation

use sea_orm::DatabaseConnection;
use std::sync::Arc;
use uuid::Uuid;

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::oauth::entities::OAuthProvider;
use herald_domain::oauth::ports::OAuthRepository;

// Type alias to reduce complexity
type OAuthProviderRow = (
    uuid::Uuid,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<Uuid>,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
);

#[derive(Debug)]
pub struct PostgresOAuthRepository {
    db: Arc<DatabaseConnection>,
}

impl PostgresOAuthRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    fn model_to_entity(
        id: uuid::Uuid,
        realm_id: String,
        provider_type: String,
        open_id: String,
        union_id: Option<String>,
        email: Option<String>,
        user_id: Option<Uuid>,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<OAuthProvider, CoreError> {
        use herald_domain::oauth::entities::ProviderType;
        use std::str::FromStr;

        let provider = ProviderType::from_str(&provider_type).map_err(|_| {
            CoreError::BadRequest(format!("Invalid provider type: {}", provider_type))
        })?;

        Ok(OAuthProvider {
            id,
            realm_id,
            provider_type: provider,
            open_id,
            union_id,
            email,
            user_id,
            created_at,
            updated_at,
        })
    }
}

impl OAuthRepository for PostgresOAuthRepository {
    async fn find_by_provider_and_open_id(
        &self,
        realm_id: &str,
        provider_type: &str,
        open_id: &str,
    ) -> Result<OAuthProvider, CoreError> {
        let (id, realm, provider, open_id, union_id, email, user_id, created_at, updated_at): OAuthProviderRow =
            sqlx::query_as(
                "SELECT id, realm_id, provider_type, open_id, union_id, email, user_id, created_at, updated_at
                 FROM provider
                 WHERE realm_id = $1 AND provider_type = $2 AND open_id = $3",
            )
            .bind(realm_id)
            .bind(provider_type)
            .bind(open_id)
            .fetch_one(self.db.get_postgres_connection_pool())
            .await
            .map_err(|e| {
                tracing::error!("Failed to find oauth provider: {e}");
                if e.to_string().contains("not found") || e.to_string().contains("no rows") {
                    CoreError::NotFound
                } else {
                    CoreError::InternalServerError(e.to_string())
                }
            })?;

        Self::model_to_entity(
            id, realm, provider, open_id, union_id, email, user_id, created_at, updated_at,
        )
    }

    async fn find_by_union_id(
        &self,
        realm_id: &str,
        union_id: &str,
    ) -> Result<OAuthProvider, CoreError> {
        let (id, realm, provider, open_id, union_id, email, user_id, created_at, updated_at): OAuthProviderRow =
            sqlx::query_as(
                "SELECT id, realm_id, provider_type, open_id, union_id, email, user_id, created_at, updated_at
                 FROM provider
                 WHERE realm_id = $1 AND union_id = $2",
            )
            .bind(realm_id)
            .bind(union_id)
            .fetch_one(self.db.get_postgres_connection_pool())
            .await
            .map_err(|e| {
                tracing::error!("Failed to find oauth provider by union_id: {e}");
                if e.to_string().contains("not found") || e.to_string().contains("no rows") {
                    CoreError::NotFound
                } else {
                    CoreError::InternalServerError(e.to_string())
                }
            })?;

        Self::model_to_entity(
            id, realm, provider, open_id, union_id, email, user_id, created_at, updated_at,
        )
    }

    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<OAuthProvider>, CoreError> {
        let rows: Vec<OAuthProviderRow> = sqlx::query_as(
            "SELECT id, realm_id, provider_type, open_id, union_id, email, user_id, created_at, updated_at
             FROM provider
             WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(self.db.get_postgres_connection_pool())
        .await
        .map_err(|e| CoreError::InternalServerError(e.to_string()))?;

        rows.into_iter()
            .map(
                |(
                    id,
                    realm,
                    provider,
                    open_id,
                    union_id,
                    email,
                    user_id,
                    created_at,
                    updated_at,
                )| {
                    Self::model_to_entity(
                        id, realm, provider, open_id, union_id, email, user_id, created_at,
                        updated_at,
                    )
                },
            )
            .collect()
    }

    async fn create_provider(&self, provider: OAuthProvider) -> Result<OAuthProvider, CoreError> {
        let rec: OAuthProviderRow = sqlx::query_as(
                "INSERT INTO provider (id, realm_id, provider_type, open_id, union_id, email, user_id, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 RETURNING id, realm_id, provider_type, open_id, union_id, email, user_id, created_at, updated_at",
            )
            .bind(provider.id)
            .bind(&provider.realm_id)
            .bind(provider.provider_type.as_str())
            .bind(&provider.open_id)
            .bind(&provider.union_id)
            .bind(&provider.email)
            .bind(provider.user_id)
            .bind(provider.created_at)
            .bind(provider.updated_at)
            .fetch_one(self.db.get_postgres_connection_pool())
            .await
            .map_err(|e| {
                tracing::error!("Failed to create oauth provider: {e}");
                if e.to_string().contains("duplicate key") || e.to_string().contains("unique constraint") {
                    CoreError::Conflict("OAuth provider already exists".to_string())
                } else {
                    CoreError::InternalServerError(e.to_string())
                }
            })?;

        Self::model_to_entity(
            rec.0, rec.1, rec.2, rec.3, rec.4, rec.5, rec.6, rec.7, rec.8,
        )
    }

    async fn link_provider_to_user(
        &self,
        user_id: Uuid,
        provider_id: Uuid,
    ) -> Result<(), CoreError> {
        sqlx::query("UPDATE provider SET user_id = $1, updated_at = NOW() WHERE id = $2")
            .bind(user_id)
            .bind(provider_id)
            .execute(self.db.get_postgres_connection_pool())
            .await
            .map_err(|e| CoreError::InternalServerError(e.to_string()))?;

        Ok(())
    }

    async fn unlink_provider_from_user(
        &self,
        user_id: Uuid,
        provider_id: Uuid,
    ) -> Result<(), CoreError> {
        sqlx::query(
            "UPDATE provider SET user_id = NULL, updated_at = NOW() WHERE id = $1 AND user_id = $2",
        )
        .bind(provider_id)
        .bind(user_id)
        .execute(self.db.get_postgres_connection_pool())
        .await
        .map_err(|e| CoreError::InternalServerError(e.to_string()))?;

        Ok(())
    }
}
