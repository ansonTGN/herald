use chrono::{Duration, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::redis::RedisConnectionManager;
use herald_domain::authentication::{
    entities::{Session, SessionData},
    ports::SessionRepository,
};
use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::security_constants::DEFAULT_SESSION_TTL_SECONDS;

pub struct RedisSessionRepository {
    manager: RedisConnectionManager,
}

impl RedisSessionRepository {
    pub fn new(manager: RedisConnectionManager) -> Self {
        Self { manager }
    }

    fn session_key(token: &str) -> String {
        format!("sess:{}", token)
    }

    async fn get_connection(&self) -> Result<redis::aio::ConnectionManager, CoreError> {
        self.manager.get().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get Redis connection");
            CoreError::InternalServerError(format!("Redis connection error: {}", e))
        })
    }
}

#[derive(Serialize, Deserialize)]
struct SessionDataInternal {
    realm_id: String,
    client_id: String,
    user_id: String,
    client_ip: String,
    #[serde(default)]
    renewal_ttl_seconds: Option<u64>,
}

impl SessionRepository for RedisSessionRepository {
    async fn store_session(
        &self,
        token: &str,
        data: SessionData,
        ttl_seconds: u64,
    ) -> Result<(), CoreError> {
        let mut con = self.get_connection().await?;

        let data_internal = SessionDataInternal {
            realm_id: data.realm_id,
            client_id: data.client_id,
            user_id: data.user_id,
            client_ip: data.client_ip,
            renewal_ttl_seconds: data.renewal_ttl_seconds,
        };

        let value = serde_json::to_string(&data_internal)?;
        let key = Self::session_key(token);

        let _: () = con.set_ex(key, value, ttl_seconds).await?;
        Ok(())
    }

    async fn load_session(&self, token: &str) -> Result<Option<Session>, CoreError> {
        let mut con = self.get_connection().await?;
        let key = Self::session_key(token);

        let value: Option<String> = con.get(&key).await?;
        let data_internal = match value {
            Some(v) => serde_json::from_str::<SessionDataInternal>(&v)?,
            None => return Ok(None),
        };

        Ok(Some(Session {
            token: token.to_string(),
            realm_id: data_internal.realm_id,
            client_id: data_internal.client_id,
            user_id: data_internal.user_id,
            expires_at: Utc::now() + Duration::seconds(DEFAULT_SESSION_TTL_SECONDS as i64),
        }))
    }

    async fn load_session_data(&self, token: &str) -> Result<Option<SessionData>, CoreError> {
        let mut con = self.get_connection().await?;
        let key = Self::session_key(token);

        let value: Option<String> = con.get(&key).await?;
        let data_internal = match value {
            Some(v) => serde_json::from_str::<SessionDataInternal>(&v)?,
            None => return Ok(None),
        };

        Ok(Some(SessionData {
            realm_id: data_internal.realm_id,
            client_id: data_internal.client_id,
            user_id: data_internal.user_id,
            client_ip: data_internal.client_ip,
            renewal_ttl_seconds: data_internal.renewal_ttl_seconds,
        }))
    }

    async fn delete_session(&self, token: &str) -> Result<(), CoreError> {
        let mut con = self.get_connection().await?;
        let key = Self::session_key(token);

        let _: usize = con.del(key).await?;
        Ok(())
    }

    async fn delete_user_sessions(&self, user_id: &str) -> Result<(), CoreError> {
        // This requires scanning keys which can be slow
        // For production, consider maintaining a user_sessions index
        let mut con = self.get_connection().await?;

        // Simple approach: scan for pattern (not ideal for production)
        let pattern = "sess:*".to_string();
        let keys: Vec<String> = con.keys(&pattern).await?;

        for key in keys {
            if let Ok(Some(session)) = self.load_session(&key.replace("sess:", "")).await
                && session.user_id == user_id
            {
                let _: usize = con.del(&key).await?;
            }
        }

        Ok(())
    }
}
