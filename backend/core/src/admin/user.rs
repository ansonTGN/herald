use herald_domain::authorization::principal_types;
use herald_domain::security_constants::DEFAULT_BCRYPT_COST;
use sqlx::{PgPool, Row};
use std::env;
use tracing::info;
use uuid::Uuid;

/// Built-in role names (must match the names in the roles table)
pub const BUILTIN_ROLE_REALM_ADMIN: &str = "realm-admin";
pub const BUILTIN_ROLE_USER: &str = "user";

pub async fn get_admin_root_id(pg: &PgPool) -> anyhow::Result<(String, String)> {
    // Use ADMIN_REALM_ID environment variable if set, otherwise fall back to first realm
    let realm_id: String = match env::var("ADMIN_REALM_ID") {
        Ok(admin_realm) => admin_realm,
        Err(_) => {
            let realm_id_opt: Option<String> = sqlx::query("select id from realm limit 1")
                .fetch_one(pg)
                .await
                .map(|x| x.get("id"))?;
            realm_id_opt.unwrap_or_else(|| "admin".to_string())
        }
    };

    // Use client_id (string identifier) instead of id (UUID)
    // roles.client_id stores the client identifier string (e.g., 'admin-web-console')
    let client_id: String = sqlx::query("select client_id from client_app limit 1")
        .fetch_one(pg)
        .await
        .map(|x| x.get("client_id"))?;
    Ok((realm_id, client_id))
}

/// Query the role ID by name from the roles table (ignoring client_id for compatibility)
async fn get_role_id_by_name(
    pool: &PgPool,
    role_name: &str,
    realm_id: &str,
) -> anyhow::Result<Option<Uuid>> {
    let result = sqlx::query_scalar("SELECT id FROM roles WHERE name = $1 AND realm_id = $2")
        .bind(role_name)
        .bind(realm_id)
        .fetch_optional(pool)
        .await?;

    Ok(result)
}

pub async fn init_admin_user(pool: &PgPool, app_env: &str) -> anyhow::Result<()> {
    let count: i64 = sqlx::query("SELECT count(*) as count FROM account limit 1")
        .fetch_one(pool)
        .await?
        .get("count");

    if count == 0 {
        info!("No users found, attempting to create admin user.");
        let admin_email = env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@cas.com".to_string());
        let admin_password = match env::var("ADMIN_PASSWORD") {
            Ok(pw) => pw,
            Err(_) => {
                if app_env == "production" {
                    anyhow::bail!(
                        "ADMIN_PASSWORD environment variable is required in production. \
                         Refusing to use default password."
                    );
                }
                tracing::warn!(
                    "ADMIN_PASSWORD not set, using default password (only allowed in {})",
                    app_env
                );
                "password".to_string()
            }
        };

        let hashed_password = bcrypt::hash(&admin_password, DEFAULT_BCRYPT_COST)?;

        // Get all required data before starting transaction
        let (realm_id, client_id) = get_admin_root_id(pool).await?;

        // Query realm-admin role (must exist - init_admin_realm_rbac should be called first)
        let realm_admin_role_id =
            get_role_id_by_name(pool, BUILTIN_ROLE_REALM_ADMIN, &realm_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("realm-admin role not found. Make sure init_admin_realm_rbac is called before init_admin_user"))?;

        // Start transaction for all write operations
        let mut tx = pool.begin().await?;

        let user_id: Uuid = sqlx::query(
            "INSERT INTO account (realm_id, email, password, status) VALUES ($1, $2, $3, 1) RETURNING id",
        )
        .bind(&realm_id)
        .bind(&admin_email)
        .bind(&hashed_password)
        .fetch_one(&mut *tx)
        .await?
        .get("id");

        sqlx::query("INSERT INTO profile (id, realm_id, nickname) VALUES ($1, $2, 'Admin')")
            .bind(user_id)
            .bind(&realm_id)
            .execute(&mut *tx)
            .await?;

        // Assign realm-admin role to admin user using user_roles table within transaction
        sqlx::query(
            "INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id, principal_type, principal_id) VALUES ($1, $2, $3, $4, $5, $6, $2::text)"
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(realm_admin_role_id)
        .bind(&realm_id)
        .bind(&client_id)
        .bind(principal_types::USER)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        // System-created admin user: record consent to the current effective
        // agreements so login is not blocked by the legal-consent gate (BE-D08).
        // Best-effort: if no agreement versions are seeded yet, skip silently.
        if let Err(e) = record_admin_user_consent(pool, &realm_id, user_id).await {
            tracing::warn!(
                user_id = %user_id,
                realm_id = %realm_id,
                error = %e,
                "Failed to record initial consent for system admin user"
            );
        }

        info!(
            "Successfully created admin user with email: {}",
            admin_email
        );
    }

    Ok(())
}

async fn record_admin_user_consent(
    pool: &PgPool,
    realm_id: &str,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    for agreement_type in ["terms_of_service", "privacy_policy"] {
        let version_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM legal_agreement_version
             WHERE agreement_type = $1
               AND (realm_id = $2 OR realm_id IS NULL)
             ORDER BY CASE WHEN realm_id = $2 THEN 0 ELSE 1 END, version_no DESC
             LIMIT 1",
        )
        .bind(agreement_type)
        .bind(realm_id)
        .fetch_optional(pool)
        .await?;

        if let Some(version_id) = version_id {
            sqlx::query(
                "INSERT INTO user_agreement_consent (id, user_id, realm_id, agreement_type, consented_version_id)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (user_id, agreement_type)
                 DO UPDATE SET consented_version_id = EXCLUDED.consented_version_id, consented_at = NOW()",
            )
            .bind(Uuid::now_v7())
            .bind(user_id)
            .bind(realm_id)
            .bind(agreement_type)
            .bind(version_id)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}
