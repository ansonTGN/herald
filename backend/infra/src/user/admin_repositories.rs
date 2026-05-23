// PostgreSQL Repository Implementations for User Admin Module
//
// This module provides database operations for user administration using SQLx.
// Following hexagonal architecture principles, these implementations depend only
// on the domain layer traits and entities.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use herald_domain::user::{
    AdminUserEntity, AdminUserRepository, PolicyEntity, RoleEntity, RolePolicyRepository,
    UserAdminError, UserAdminResult, UserRoleRepository,
};

use herald_domain::authorization::principal_types;
use herald_domain::common::entities::generate_uuid_v7;

// ============================================================================
// Admin User Repository
// ============================================================================

/// PostgreSQL implementation of AdminUserRepository
pub struct PostgresAdminUserRepository {
    pool: PgPool,
}

impl PostgresAdminUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Map database row to AdminUserEntity
    fn row_to_user_entity(
        id: Uuid,
        realm_id: String,
        email: String,
        nickname: Option<String>,
        status: i16,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> AdminUserEntity {
        AdminUserEntity {
            id,
            realm_id,
            email,
            nickname,
            status: i32::from(status),
            created_at,
            updated_at,
        }
    }
}

impl AdminUserRepository for PostgresAdminUserRepository {
    async fn create_user_with_profile(
        &self,
        realm_id: &str,
        email: &str,
        password_hash: &str,
        nickname: Option<&str>,
        status: i32,
    ) -> UserAdminResult<Uuid> {
        // Start a transaction for atomic user + profile creation
        let mut tx = self.pool.begin().await.map_err(|e| {
            tracing::error!("Failed to begin transaction: {}", e);
            UserAdminError::DatabaseError(format!("Failed to begin transaction: {}", e))
        })?;

        // Insert user account (let database generate UUID via DEFAULT uuidv7())
        let user_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO account (realm_id, email, password, status, provider_ids)
            VALUES ($1, $2, $3, $4, ARRAY[]::UUID[])
            RETURNING id
            "#,
        )
        .bind(realm_id)
        .bind(email)
        .bind(password_hash)
        .bind(status)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            if e.to_string().contains("account_email_key") {
                tracing::debug!("Email already exists: {}", email);
                UserAdminError::DuplicateEmail(email.to_string())
            } else {
                tracing::error!("Failed to create user: {}", e);
                UserAdminError::DatabaseError(format!("Failed to create user: {}", e))
            }
        })?;

        // Insert profile if nickname provided
        if let Some(nick) = nickname {
            sqlx::query(
                r#"
                INSERT INTO profile (id, realm_id, nickname)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(user_id)
            .bind(realm_id)
            .bind(nick)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create profile: {}", e);
                UserAdminError::DatabaseError(format!("Failed to create profile: {}", e))
            })?;
        }

        tx.commit().await.map_err(|e| {
            tracing::error!("Failed to commit transaction: {}", e);
            UserAdminError::DatabaseError(format!("Failed to commit transaction: {}", e))
        })?;

        tracing::info!(
            "Created user with profile: user_id={}, email={}",
            user_id,
            email
        );
        Ok(user_id)
    }

    async fn update_user_fields(
        &self,
        user_id: Uuid,
        email: Option<&str>,
        nickname: Option<&str>,
        status: Option<i32>,
    ) -> UserAdminResult<()> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            tracing::error!("Failed to begin transaction: {}", e);
            UserAdminError::DatabaseError(format!("Failed to begin transaction: {}", e))
        })?;

        // Update account fields if provided
        if email.is_some() || status.is_some() {
            let mut query = String::from("UPDATE account SET updated_at = NOW()");
            let mut param_count = 0;

            if email.is_some() {
                param_count += 1;
                query.push_str(&format!(", email = ${}", param_count));
            }
            if status.is_some() {
                param_count += 1;
                query.push_str(&format!(", status = ${}", param_count));
            }

            param_count += 1;
            query.push_str(&format!(" WHERE id = ${}", param_count));

            let mut query_builder = sqlx::query(&query);

            if let Some(e) = email {
                query_builder = query_builder.bind(e);
            }
            if let Some(s) = status {
                query_builder = query_builder.bind(s);
            }
            query_builder = query_builder.bind(user_id);

            query_builder.execute(&mut *tx).await.map_err(|e| {
                if e.to_string().contains("account_email_key") {
                    tracing::debug!("Email already exists");
                    UserAdminError::DuplicateEmail("Email already exists".to_string())
                } else {
                    tracing::error!("Failed to update user: {}", e);
                    UserAdminError::DatabaseError(format!("Failed to update user: {}", e))
                }
            })?;
        }

        // Update profile if nickname provided
        if let Some(nick) = nickname {
            sqlx::query(
                r#"
                INSERT INTO profile (id, realm_id, nickname, updated_at)
                VALUES ($1, (SELECT realm_id FROM account WHERE id = $1), $2, NOW())
                ON CONFLICT (id, realm_id) DO UPDATE SET nickname = $2, updated_at = NOW()
                "#,
            )
            .bind(user_id)
            .bind(nick)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update profile: {}", e);
                UserAdminError::DatabaseError(format!("Failed to update profile: {}", e))
            })?;
        }

        tx.commit().await.map_err(|e| {
            tracing::error!("Failed to commit transaction: {}", e);
            UserAdminError::DatabaseError(format!("Failed to commit transaction: {}", e))
        })?;

        tracing::info!("Updated user fields: user_id={}", user_id);
        Ok(())
    }

    async fn get_user_with_profile(
        &self,
        user_id: Uuid,
    ) -> UserAdminResult<Option<AdminUserEntity>> {
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                Option<String>,
                i16,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT a.id, a.realm_id, a.email, p.nickname, a.status, a.created_at, a.updated_at
            FROM account a
            LEFT JOIN profile p ON a.id = p.id
            WHERE a.id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch user: {}", e);
            UserAdminError::DatabaseError(format!("Failed to fetch user: {}", e))
        })?;

        Ok(row.map(
            |(id, realm_id, email, nickname, status, created_at, updated_at)| {
                Self::row_to_user_entity(
                    id, realm_id, email, nickname, status, created_at, updated_at,
                )
            },
        ))
    }

    async fn email_exists(&self, realm_id: &str, email: &str) -> UserAdminResult<bool> {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(SELECT 1 FROM account WHERE realm_id = $1 AND email = $2)
            "#,
        )
        .bind(realm_id)
        .bind(email)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check email existence: {}", e);
            UserAdminError::DatabaseError(format!("Failed to check email existence: {}", e))
        })?;

        Ok(exists)
    }

    async fn get_user_by_email(
        &self,
        realm_id: &str,
        email: &str,
    ) -> UserAdminResult<Option<AdminUserEntity>> {
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                Option<String>,
                i16,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT a.id, a.realm_id, a.email, p.nickname, a.status, a.created_at, a.updated_at
            FROM account a
            LEFT JOIN profile p ON a.id = p.id
            WHERE a.realm_id = $1 AND a.email = $2
            "#,
        )
        .bind(realm_id)
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch user by email: {}", e);
            UserAdminError::DatabaseError(format!("Failed to fetch user by email: {}", e))
        })?;

        Ok(row.map(
            |(id, realm_id, email, nickname, status, created_at, updated_at)| {
                Self::row_to_user_entity(
                    id, realm_id, email, nickname, status, created_at, updated_at,
                )
            },
        ))
    }

    async fn delete_user(&self, user_id: Uuid) -> UserAdminResult<bool> {
        // Start a transaction for atomic user deletion
        let mut tx = self.pool.begin().await.map_err(|e| {
            tracing::error!("Failed to begin transaction: {}", e);
            UserAdminError::DatabaseError(format!("Failed to begin transaction: {}", e))
        })?;

        // Delete profile first (foreign key constraint)
        sqlx::query("DELETE FROM profile WHERE id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to delete profile: {}", e);
                UserAdminError::DatabaseError(format!("Failed to delete profile: {}", e))
            })?;

        // Delete account
        let result = sqlx::query("DELETE FROM account WHERE id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to delete account: {}", e);
                UserAdminError::DatabaseError(format!("Failed to delete account: {}", e))
            })?;

        // Commit transaction
        tx.commit().await.map_err(|e| {
            tracing::error!("Failed to commit transaction: {}", e);
            UserAdminError::DatabaseError(format!("Failed to commit transaction: {}", e))
        })?;

        let deleted = result.rows_affected() > 0;
        tracing::info!("Deleted user: user_id={}, deleted={}", user_id, deleted);

        Ok(deleted)
    }

    async fn update_user_password(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> UserAdminResult<bool> {
        let result = sqlx::query(
            "UPDATE account SET password = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(password_hash)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update password: {}", e);
            UserAdminError::DatabaseError(format!("Failed to update password: {}", e))
        })?;

        let updated = result.rows_affected() > 0;
        tracing::info!(
            "Updated user password: user_id={}, updated={}",
            user_id,
            updated
        );

        Ok(updated)
    }
}

// ============================================================================
// User Role Repository
// ============================================================================

/// PostgreSQL implementation of UserRoleRepository
pub struct PostgresUserRoleRepository {
    pool: PgPool,
}

impl PostgresUserRoleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl UserRoleRepository for PostgresUserRoleRepository {
    async fn replace_user_roles(
        &self,
        user_id: Uuid,
        realm_id: &str,
        client_id: &str,
        role_ids: &[Uuid],
    ) -> UserAdminResult<()> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            tracing::error!("Failed to begin transaction: {}", e);
            UserAdminError::DatabaseError(format!("Failed to begin transaction: {}", e))
        })?;

        // Delete existing roles for this user in the realm/client context
        sqlx::query(
            r#"
            DELETE FROM user_roles
            WHERE user_id = $1 AND realm_id = $2 AND client_id = $3
            "#,
        )
        .bind(user_id)
        .bind(realm_id)
        .bind(client_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete existing roles: {}", e);
            UserAdminError::DatabaseError(format!("Failed to delete existing roles: {}", e))
        })?;

        // Insert new role assignments
        for role_id in role_ids {
            let user_role_id = generate_uuid_v7();
            sqlx::query(
                r#"
                INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id, principal_type, principal_id)
                VALUES ($1, $2, $3, $4, $5, $6, $2::text)
                "#,
            )
            .bind(user_role_id)
            .bind(user_id)
            .bind(role_id)
            .bind(realm_id)
            .bind(client_id)
            .bind(principal_types::USER)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to insert user role: {}", e);
                UserAdminError::DatabaseError(format!("Failed to insert user role: {}", e))
            })?;
        }

        tx.commit().await.map_err(|e| {
            tracing::error!("Failed to commit transaction: {}", e);
            UserAdminError::DatabaseError(format!("Failed to commit transaction: {}", e))
        })?;

        tracing::info!(
            "Replaced roles for user: user_id={}, role_count={}",
            user_id,
            role_ids.len()
        );
        Ok(())
    }

    async fn get_user_role_ids(&self, user_id: Uuid) -> UserAdminResult<Vec<Uuid>> {
        let role_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT role_id
            FROM user_roles
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch user role IDs: {}", e);
            UserAdminError::DatabaseError(format!("Failed to fetch user role IDs: {}", e))
        })?;

        Ok(role_ids)
    }

    async fn get_user_roles(&self, user_id: Uuid) -> UserAdminResult<Vec<RoleEntity>> {
        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                Option<String>,
                bool,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT r.id, r.realm_id, r.name, r.description, r.is_builtin, r.created_at, r.updated_at
            FROM roles r
            INNER JOIN user_roles ur ON r.id = ur.role_id
            WHERE ur.user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch user roles: {}", e);
            UserAdminError::DatabaseError(format!("Failed to fetch user roles: {}", e))
        })?;

        let roles = rows
            .into_iter()
            .map(
                |(id, realm_id, name, description, is_builtin, created_at, updated_at)| {
                    RoleEntity {
                        id,
                        realm_id,
                        name,
                        description,
                        is_builtin,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect();

        Ok(roles)
    }

    async fn add_user_role(
        &self,
        user_id: Uuid,
        role_id: Uuid,
        realm_id: &str,
        client_id: &str,
    ) -> UserAdminResult<()> {
        let user_role_id = generate_uuid_v7();

        sqlx::query(
            r#"
            INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id, principal_type, principal_id)
            VALUES ($1, $2, $3, $4, $5, $6, $2::text)
            "#,
        )
        .bind(user_role_id)
        .bind(user_id)
        .bind(role_id)
        .bind(realm_id)
        .bind(client_id)
        .bind(principal_types::USER)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add user role: {}", e);
            UserAdminError::DatabaseError(format!("Failed to add user role: {}", e))
        })?;

        tracing::info!(
            "Added user role: user_id={}, role_id={}, client_id={}",
            user_id,
            role_id,
            client_id
        );
        Ok(())
    }

    async fn remove_user_role(
        &self,
        user_id: Uuid,
        role_id: Uuid,
        client_id: &str,
    ) -> UserAdminResult<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM user_roles
            WHERE user_id = $1 AND role_id = $2 AND client_id = $3
            "#,
        )
        .bind(user_id)
        .bind(role_id)
        .bind(client_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to remove user role: {}", e);
            UserAdminError::DatabaseError(format!("Failed to remove user role: {}", e))
        })?;

        let removed = result.rows_affected() > 0;
        tracing::info!(
            "Removed user role: user_id={}, role_id={}, removed={}",
            user_id,
            role_id,
            removed
        );
        Ok(removed)
    }

    async fn list_user_roles_by_realm_client(
        &self,
        realm_id: &str,
        client_id: &str,
    ) -> UserAdminResult<Vec<(Uuid, Uuid)>> {
        let rows = sqlx::query_as::<_, (Uuid, Uuid)>(
            r#"
            SELECT user_id, role_id
            FROM user_roles
            WHERE realm_id = $1 AND client_id = $2
            "#,
        )
        .bind(realm_id)
        .bind(client_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list user roles: {}", e);
            UserAdminError::DatabaseError(format!("Failed to list user roles: {}", e))
        })?;

        Ok(rows)
    }

    async fn replace_api_key_roles(
        &self,
        api_key_id: &str,
        realm_id: &str,
        client_id: &str,
        role_ids: &[Uuid],
    ) -> UserAdminResult<()> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            tracing::error!("Failed to begin transaction: {}", e);
            UserAdminError::DatabaseError(format!("Failed to begin transaction: {}", e))
        })?;

        // Delete existing roles for this API key principal
        sqlx::query(
            r#"
            DELETE FROM user_roles
            WHERE principal_type = 'api_key' AND principal_id = $1 AND realm_id = $2 AND client_id = $3
            "#,
        )
        .bind(api_key_id)
        .bind(realm_id)
        .bind(client_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete existing API key roles: {}", e);
            UserAdminError::DatabaseError(format!("Failed to delete existing API key roles: {}", e))
        })?;

        // Insert new role assignments
        for role_id in role_ids {
            let user_role_id = generate_uuid_v7();
            sqlx::query(
                r#"
                INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id, principal_type, principal_id)
                VALUES ($1, NULL, $2, $3, $4, 'api_key', $5)
                "#,
            )
            .bind(user_role_id)
            .bind(role_id)
            .bind(realm_id)
            .bind(client_id)
            .bind(api_key_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to insert API key role: {}", e);
                UserAdminError::DatabaseError(format!("Failed to insert API key role: {}", e))
            })?;
        }

        tx.commit().await.map_err(|e| {
            tracing::error!("Failed to commit transaction: {}", e);
            UserAdminError::DatabaseError(format!("Failed to commit transaction: {}", e))
        })?;

        tracing::info!(
            "Replaced roles for API key: api_key_id={}, role_count={}",
            api_key_id,
            role_ids.len()
        );
        Ok(())
    }

    async fn get_api_key_roles(&self, api_key_id: &str) -> UserAdminResult<Vec<RoleEntity>> {
        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                Option<String>,
                bool,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT r.id, r.realm_id, r.name, r.description, r.is_builtin, r.created_at, r.updated_at
            FROM roles r
            INNER JOIN user_roles ur ON r.id = ur.role_id
            WHERE ur.principal_type = 'api_key' AND ur.principal_id = $1
            "#,
        )
        .bind(api_key_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch API key roles: {}", e);
            UserAdminError::DatabaseError(format!("Failed to fetch API key roles: {}", e))
        })?;

        let roles = rows
            .into_iter()
            .map(
                |(id, realm_id, name, description, is_builtin, created_at, updated_at)| {
                    RoleEntity {
                        id,
                        realm_id,
                        name,
                        description,
                        is_builtin,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect();

        Ok(roles)
    }

    async fn get_api_key_role_summaries_batch(
        &self,
        api_key_ids: &[String],
    ) -> UserAdminResult<Vec<(String, Vec<(Uuid, String)>)>> {
        if api_key_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Build placeholder string for IN clause
        let placeholders = api_key_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", i + 1))
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            r#"
            SELECT ur.principal_id, r.id, r.name
            FROM user_roles ur
            INNER JOIN roles r ON ur.role_id = r.id
            WHERE ur.principal_type = 'api_key' AND ur.principal_id IN ({})
            "#,
            placeholders
        );

        let mut query_builder = sqlx::query_as::<_, (String, Uuid, String)>(&query);

        for api_key_id in api_key_ids {
            query_builder = query_builder.bind(api_key_id);
        }

        let rows = query_builder.fetch_all(&self.pool).await.map_err(|e| {
            tracing::error!("Failed to fetch API key role summaries: {}", e);
            UserAdminError::DatabaseError(format!("Failed to fetch API key role summaries: {}", e))
        })?;

        // Group by principal_id preserving input order
        let mut result: Vec<(String, Vec<(Uuid, String)>)> = Vec::new();
        for api_key_id in api_key_ids {
            let roles: Vec<(Uuid, String)> = rows
                .iter()
                .filter(|(pid, _, _)| pid == api_key_id)
                .map(|(_, role_id, role_name)| (*role_id, role_name.clone()))
                .collect();
            if !roles.is_empty() {
                result.push((api_key_id.clone(), roles));
            }
        }

        Ok(result)
    }
}

// ============================================================================
// Role Policy Repository
// ============================================================================

/// PostgreSQL implementation of RolePolicyRepository
pub struct PostgresRolePolicyRepository {
    pool: PgPool,
}

impl PostgresRolePolicyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl RolePolicyRepository for PostgresRolePolicyRepository {
    async fn get_role_policies_for_user(
        &self,
        realm_id: &str,
        role_ids: &[Uuid],
    ) -> UserAdminResult<Vec<PolicyEntity>> {
        if role_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Build placeholder string for IN clause: $2, $3, $4, ...
        let placeholders = role_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", i + 2))
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            r#"
            SELECT DISTINCT rp.id, rp.realm_id, rp.resource, rp.action, NULL::text AS policy_json, rp.created_at, rp.updated_at
            FROM role_policies rp
            WHERE rp.realm_id = $1 AND rp.role_id IN ({}) AND rp.effect = true
            "#,
            placeholders
        );

        let mut query_builder = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                String,
                Option<String>,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(&query);
        query_builder = query_builder.bind(realm_id);

        for role_id in role_ids {
            query_builder = query_builder.bind(role_id);
        }

        let rows = query_builder.fetch_all(&self.pool).await.map_err(|e| {
            tracing::error!("Failed to fetch role policies: {}", e);
            UserAdminError::DatabaseError(format!("Failed to fetch role policies: {}", e))
        })?;

        let policies = rows
            .into_iter()
            .map(
                |(id, realm_id, resource, action, policy_json, created_at, updated_at)| {
                    PolicyEntity {
                        id,
                        realm_id,
                        resource,
                        action,
                        policy_json,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect();

        Ok(policies)
    }

    async fn get_direct_user_policies(&self, _user_id: Uuid) -> UserAdminResult<Vec<PolicyEntity>> {
        // Note: user_policies table doesn't exist in current schema
        // This is a placeholder for future direct user permission assignment
        // For now, return empty vector
        tracing::warn!("get_direct_user_policies called but user_policies table doesn't exist yet");
        Ok(Vec::new())
    }

    async fn get_roles_by_ids(&self, role_ids: &[Uuid]) -> UserAdminResult<Vec<RoleEntity>> {
        if role_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Build placeholder string for IN clause
        let placeholders = role_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", i + 1))
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            r#"
            SELECT id, realm_id, name, description, is_builtin, created_at, updated_at
            FROM roles
            WHERE id IN ({})
            "#,
            placeholders
        );

        let mut query_builder = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                Option<String>,
                bool,
                DateTime<Utc>,
                DateTime<Utc>,
            ),
        >(&query);

        for role_id in role_ids {
            query_builder = query_builder.bind(role_id);
        }

        let rows = query_builder.fetch_all(&self.pool).await.map_err(|e| {
            tracing::error!("Failed to fetch roles by IDs: {}", e);
            UserAdminError::DatabaseError(format!("Failed to fetch roles by IDs: {}", e))
        })?;

        let roles = rows
            .into_iter()
            .map(
                |(id, realm_id, name, description, is_builtin, created_at, updated_at)| {
                    RoleEntity {
                        id,
                        realm_id,
                        name,
                        description,
                        is_builtin,
                        created_at,
                        updated_at,
                    }
                },
            )
            .collect();

        Ok(roles)
    }

    async fn assign_direct_permission(
        &self,
        _user_id: Uuid,
        _realm_id: &str,
        _policy_id: Uuid,
    ) -> UserAdminResult<()> {
        // Note: user_policies table doesn't exist in current schema
        // This is a placeholder for future direct user permission assignment
        tracing::warn!("assign_direct_permission called but user_policies table doesn't exist yet");
        Err(UserAdminError::InternalError(
            "Direct user permission assignment is not yet supported".to_string(),
        ))
    }

    async fn remove_direct_permission(
        &self,
        _user_id: Uuid,
        _policy_id: Uuid,
    ) -> UserAdminResult<()> {
        // Note: user_policies table doesn't exist in current schema
        // This is a placeholder for future direct user permission assignment
        tracing::warn!("remove_direct_permission called but user_policies table doesn't exist yet");
        Err(UserAdminError::InternalError(
            "Direct user permission assignment is not yet supported".to_string(),
        ))
    }

    async fn create_role_policy(
        &self,
        role_id: Uuid,
        realm_id: &str,
        resource: &str,
        action: &str,
    ) -> UserAdminResult<()> {
        let policy_id = generate_uuid_v7();

        sqlx::query(
            r#"
            INSERT INTO role_policies (id, role_id, realm_id, resource, action)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(policy_id)
        .bind(role_id)
        .bind(realm_id)
        .bind(resource)
        .bind(action)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create role policy: {}", e);
            UserAdminError::DatabaseError(format!("Failed to create role policy: {}", e))
        })?;

        tracing::info!(
            "Created role policy: role_id={}, resource={}, action={}",
            role_id,
            resource,
            action
        );
        Ok(())
    }

    async fn delete_role_policy(
        &self,
        role_id: Uuid,
        resource: &str,
        action: &str,
    ) -> UserAdminResult<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM role_policies
            WHERE role_id = $1 AND resource = $2 AND action = $3
            "#,
        )
        .bind(role_id)
        .bind(resource)
        .bind(action)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete role policy: {}", e);
            UserAdminError::DatabaseError(format!("Failed to delete role policy: {}", e))
        })?;

        let deleted = result.rows_affected() > 0;
        tracing::info!(
            "Deleted role policy: role_id={}, resource={}, action={}, deleted={}",
            role_id,
            resource,
            action,
            deleted
        );
        Ok(deleted)
    }

    async fn list_role_policies_by_realm(
        &self,
        realm_id: &str,
    ) -> UserAdminResult<Vec<(Uuid, String, String)>> {
        let rows = sqlx::query_as::<_, (Uuid, String, String)>(
            r#"
            SELECT role_id, resource, action
            FROM role_policies
            WHERE realm_id = $1
            "#,
        )
        .bind(realm_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list role policies: {}", e);
            UserAdminError::DatabaseError(format!("Failed to list role policies: {}", e))
        })?;

        Ok(rows)
    }
}
