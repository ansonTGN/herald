// Custom-domain host→realm mapping repository implementation
//
// Mirrors `PostgresRealmConfigRepository` patterns: `Arc<DatabaseConnection>`,
// sqlx queries via `db.get_postgres_connection_pool()`, and a `row_to_entity`
// helper. See design §4.3.2 (table) and §5.1 (effectiveness rule).

use sea_orm::DatabaseConnection;
use std::future::Future;
use std::sync::Arc;

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::custom_domain::{CustomDomainMappingRepository, MappingRow};

type MappingDbRow = (
    uuid::Uuid,
    String,
    String,
    bool,
    bool,
    bool,
    Option<chrono::DateTime<chrono::Utc>>,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
);

pub struct PostgresCustomDomainMappingRepository {
    db: Arc<DatabaseConnection>,
}

impl PostgresCustomDomainMappingRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    fn row_to_entity(row: MappingDbRow) -> MappingRow {
        let (
            id,
            realm_id,
            hostname,
            enabled,
            cname_verified,
            tls_ready,
            status_checked_at,
            created_at,
            updated_at,
        ) = row;
        MappingRow {
            id,
            realm_id,
            hostname,
            enabled,
            cname_verified,
            tls_ready,
            status_checked_at,
            created_at,
            updated_at,
        }
    }
}

const SELECT_COLS: &str = "id, realm_id, hostname, enabled, cname_verified, tls_ready, status_checked_at, created_at, updated_at";

impl CustomDomainMappingRepository for PostgresCustomDomainMappingRepository {
    fn find_by_hostname(
        &self,
        hostname: &str,
    ) -> impl Future<Output = Result<Option<MappingRow>, CoreError>> + Send {
        let hostname = hostname.to_string();
        let db = self.db.clone();

        async move {
            let row = sqlx::query_as(&format!(
                "SELECT {SELECT_COLS} FROM custom_domain_mapping
                 WHERE hostname = $1 AND enabled = true"
            ))
            .bind(&hostname)
            .fetch_optional(db.get_postgres_connection_pool())
            .await
            .map_err(|e| {
                tracing::error!(hostname = %hostname, "Failed to find custom domain mapping: {e}");
                CoreError::InternalServerError(e.to_string())
            })?;

            Ok(row.map(Self::row_to_entity))
        }
    }

    fn upsert_for_realm(
        &self,
        realm_id: &str,
        hostname: &str,
    ) -> impl Future<Output = Result<MappingRow, CoreError>> + Send {
        let realm_id = realm_id.to_string();
        let hostname = hostname.to_string();
        let db = self.db.clone();

        async move {
            // Idempotent fast path: the realm already has this hostname enabled.
            // Per design §4.2.2 publish, re-publishing the same hostname is a
            // no-op and must NOT reset CNAME/TLS status.
            let existing: Option<(uuid::Uuid,)> = sqlx::query_as(
                "SELECT id FROM custom_domain_mapping
                 WHERE realm_id = $1 AND hostname = $2 AND enabled = true",
            )
            .bind(&realm_id)
            .bind(&hostname)
            .fetch_optional(db.get_postgres_connection_pool())
            .await
            .map_err(|e| {
                tracing::error!(realm_id = %realm_id, hostname = %hostname, "Failed to check existing mapping: {e}");
                CoreError::InternalServerError(e.to_string())
            })?;

            if existing.is_some() {
                let row: MappingDbRow = sqlx::query_as(&format!(
                    "SELECT {SELECT_COLS} FROM custom_domain_mapping
                     WHERE realm_id = $1 AND hostname = $2 AND enabled = true"
                ))
                .bind(&realm_id)
                .bind(&hostname)
                .fetch_one(db.get_postgres_connection_pool())
                .await
                .map_err(|e| {
                    tracing::error!(realm_id = %realm_id, hostname = %hostname, "Failed to fetch idempotent mapping: {e}");
                    CoreError::InternalServerError(e.to_string())
                })?;
                return Ok(Self::row_to_entity(row));
            }

            let mut tx = db
                .get_postgres_connection_pool()
                .begin()
                .await
                .map_err(|e| {
                    tracing::error!(realm_id = %realm_id, hostname = %hostname, "Failed to begin transaction: {e}");
                    CoreError::InternalServerError(e.to_string())
                })?;

            // Conflict guard: the hostname is already owned by another realm.
            let owner: Option<(String,)> = sqlx::query_as(
                "SELECT realm_id FROM custom_domain_mapping WHERE hostname = $1",
            )
            .bind(&hostname)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!(hostname = %hostname, "Failed to check hostname owner: {e}");
                CoreError::InternalServerError(e.to_string())
            })?;
            if let Some((owner_realm,)) = owner
                && owner_realm != realm_id
            {
                tracing::warn!(
                    realm_id = %realm_id,
                    hostname = %hostname,
                    owner_realm = %owner_realm,
                    "Custom domain hostname already in use by another realm"
                );
                return Err(CoreError::Conflict(format!(
                    "custom domain hostname {hostname} already in use"
                )));
            }

            // Delete the realm's prior enabled hostname row(s) so at most one
            // enabled row exists per realm (design §4.3.2 invariant). The old
            // hostname is removed (not disabled).
            sqlx::query(
                "DELETE FROM custom_domain_mapping
                 WHERE realm_id = $1 AND hostname <> $2",
            )
            .bind(&realm_id)
            .bind(&hostname)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!(realm_id = %realm_id, hostname = %hostname, "Failed to delete prior realm hostname: {e}");
                CoreError::InternalServerError(e.to_string())
            })?;

            // Insert-or-replace the new hostname. ON CONFLICT(hostname) handles
            // the case where this realm previously owned the same hostname row
            // but with enabled=false (re-publish after restore). Status fields
            // reset to pending for a (re)published hostname (design §4.2.2).
            let row: MappingDbRow = sqlx::query_as(&format!(
                "INSERT INTO custom_domain_mapping (id, realm_id, hostname, enabled, cname_verified, tls_ready, status_checked_at, created_at, updated_at)
                 VALUES (uuidv7(), $1, $2, true, false, false, NULL, now(), now())
                 ON CONFLICT (hostname) DO UPDATE SET
                     realm_id = EXCLUDED.realm_id,
                     enabled = true,
                     cname_verified = false,
                     tls_ready = false,
                     status_checked_at = NULL,
                     updated_at = now()
                 RETURNING {SELECT_COLS}"
            ))
            .bind(&realm_id)
            .bind(&hostname)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!(realm_id = %realm_id, hostname = %hostname, "Failed to upsert custom domain mapping: {e}");
                CoreError::InternalServerError(e.to_string())
            })?;

            tx.commit().await.map_err(|e| {
                tracing::error!(realm_id = %realm_id, hostname = %hostname, "Failed to commit upsert: {e}");
                CoreError::InternalServerError(e.to_string())
            })?;

            Ok(Self::row_to_entity(row))
        }
    }

    fn delete_by_realm_or_hostname(
        &self,
        realm_id: Option<String>,
        hostname: Option<String>,
    ) -> impl Future<Output = Result<u64, CoreError>> + Send {
        let db = self.db.clone();

        async move {
            if realm_id.is_none() && hostname.is_none() {
                // No filter => delete-all is never the intended restore shape.
                // Fail loud rather than wipe the table.
                return Err(CoreError::bad_request(
                    "custom domain mapping delete",
                    "delete_by_realm_or_hostname requires at least one of realm_id or hostname",
                ));
            }

            let result = sqlx::query(
                "DELETE FROM custom_domain_mapping
                 WHERE ($1::text IS NOT NULL AND realm_id = $1)
                    OR ($2::text IS NOT NULL AND hostname = $2)",
            )
            .bind(realm_id.as_deref())
            .bind(hostname.as_deref())
            .execute(db.get_postgres_connection_pool())
            .await
            .map_err(|e| {
                tracing::error!(
                    realm_id = ?realm_id,
                    hostname = ?hostname,
                    "Failed to delete custom domain mapping: {e}"
                );
                CoreError::InternalServerError(e.to_string())
            })?;

            Ok(result.rows_affected())
        }
    }

    fn update_status(
        &self,
        hostname: &str,
        cname_verified: bool,
        tls_ready: bool,
    ) -> impl Future<Output = Result<(), CoreError>> + Send {
        let hostname = hostname.to_string();
        let db = self.db.clone();

        async move {
            let result = sqlx::query(
                "UPDATE custom_domain_mapping
                 SET cname_verified = $2,
                     tls_ready = $3,
                     status_checked_at = now(),
                     updated_at = now()
                 WHERE hostname = $1",
            )
            .bind(&hostname)
            .bind(cname_verified)
            .bind(tls_ready)
            .execute(db.get_postgres_connection_pool())
            .await
            .map_err(|e| {
                tracing::error!(hostname = %hostname, "Failed to update custom domain status: {e}");
                CoreError::InternalServerError(e.to_string())
            })?;

            if result.rows_affected() == 0 {
                return Err(CoreError::NotFound);
            }

            Ok(())
        }
    }
}

impl std::fmt::Debug for PostgresCustomDomainMappingRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresCustomDomainMappingRepository")
            .finish()
    }
}
