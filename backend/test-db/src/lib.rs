use sea_orm::DatabaseConnection;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;

const SHARED_POOL_MAX_CONNECTIONS: u32 = 8;

pub struct SharedTestDatabase {
    pub pool: Arc<PgPool>,
    pub pg_host: String,
    pub pg_port: u16,
    pub template_schema_name: String,
}

static SHARED_TEST_DATABASE: tokio::sync::OnceCell<SharedTestDatabase> =
    tokio::sync::OnceCell::const_new();

pub async fn get_shared_test_database() -> &'static SharedTestDatabase {
    SHARED_TEST_DATABASE
        .get_or_init(|| async {
            let pg_host =
                std::env::var("TEST_POSTGRES_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
            let pg_port = std::env::var("TEST_POSTGRES_PORT")
                .unwrap_or_else(|_| "16432".to_string())
                .parse::<u16>()
                .unwrap_or(16432);

            let pg_url = postgres_url(&pg_host, pg_port);
            let pool = PgPoolOptions::new()
                .max_connections(SHARED_POOL_MAX_CONNECTIONS)
                .acquire_timeout(std::time::Duration::from_secs(30))
                .idle_timeout(std::time::Duration::from_secs(600))
                .max_lifetime(std::time::Duration::from_secs(1800))
                .test_before_acquire(true)
                .connect(&pg_url)
                .await
                .expect(
                    "❌ Failed to connect to PgDog. 测试环境未启动，请运行:\n  uv run scripts/backend-test.py -- <测试文件>\n或先启动环境:\n  uv run scripts/test-start.py",
                );

            let template_schema_name = format!("template_test_schema_{}", std::process::id());
            recreate_template_schema(&pool, &template_schema_name).await;
            run_template_migrations(&pool, &template_schema_name).await;
            init_template_data(&pool, &template_schema_name).await;

            SharedTestDatabase {
                pool: Arc::new(pool),
                pg_host,
                pg_port,
                template_schema_name,
            }
        })
        .await
}

pub async fn recreate_template_schema(pool: &PgPool, schema_name: &str) {
    let schema_exists: bool = sqlx::query_scalar::<_, bool>(
        r#"SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = $1)"#,
    )
    .bind(schema_name)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if schema_exists {
        sqlx::query(&format!(
            r#"DROP SCHEMA IF EXISTS "{}" CASCADE"#,
            schema_name
        ))
        .execute(pool)
        .await
        .expect("Failed to drop stale template schema");
    }

    sqlx::query(&format!(r#"CREATE SCHEMA "{}""#, schema_name))
        .execute(pool)
        .await
        .expect("Failed to create template schema");
}

pub async fn run_template_migrations(pool: &PgPool, schema_name: &str) {
    let mut conn = pool
        .acquire()
        .await
        .expect("Failed to acquire migration connection");

    sqlx::query(&format!(r#"SET search_path TO "{}""#, schema_name))
        .execute(&mut *conn)
        .await
        .expect("Failed to set search_path");

    let migrations_dir = "../app/migrations";
    let mut migration_files = tokio::fs::read_dir(migrations_dir)
        .await
        .expect("Failed to read migrations directory");

    let mut migration_files_list: Vec<String> = Vec::new();
    while let Some(entry) = migration_files.next_entry().await.unwrap() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("sql")
            && let Some(file_name) = path.file_name().and_then(|s| s.to_str())
        {
            migration_files_list.push(file_name.to_string());
        }
    }

    migration_files_list.sort();

    let mut full_sql = String::new();
    for file_name in migration_files_list {
        let file_path = format!("{}/{}", migrations_dir, file_name);
        let sql = tokio::fs::read_to_string(&file_path)
            .await
            .unwrap_or_else(|_| panic!("Failed to read migration file: {}", file_path));

        let up_section = sql
            .lines()
            .take_while(|line| !line.trim().to_lowercase().contains("down migration"))
            .collect::<Vec<_>>()
            .join("\n");

        full_sql.push_str(&up_section);
        full_sql.push('\n');
    }

    execute_sql_batch_with_do_blocks(&mut conn, &full_sql).await;
}

pub async fn init_template_data(pool: &PgPool, schema_name: &str) {
    let mut conn = pool
        .acquire()
        .await
        .expect("Failed to acquire template init connection");

    sqlx::query(&format!(r#"SET search_path TO "{}""#, schema_name))
        .execute(&mut *conn)
        .await
        .expect("Failed to set search_path");

    let realm_id = "default-template-realm";
    sqlx::query(r#"INSERT INTO realm (id, name) VALUES ($1, 'Template Realm')"#)
        .bind(realm_id)
        .execute(&mut *conn)
        .await
        .expect("Failed to insert template realm");

    sqlx::query(
        r#"INSERT INTO client_app (realm_id, client_id, name, description, redirect_uris, enabled, client_secret, is_first_party)
        VALUES ($1, 'admin-web-console', 'Admin Console', 'Admin Console for Testing', '["http://localhost:3000/callback"]'::jsonb, true, 'test-secret', true)"#,
    )
    .bind(realm_id)
    .execute(&mut *conn)
    .await
    .expect("Failed to insert template client_app");
}

pub async fn clone_schema_from_template(pool: &PgPool, template_schema: &str, new_schema: &str) {
    sqlx::query(&format!(r#"CREATE SCHEMA IF NOT EXISTS "{}""#, new_schema))
        .execute(pool)
        .await
        .expect("Failed to create new schema");

    let tables: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = $1
          AND table_type = 'BASE TABLE'
        ORDER BY table_name
        "#,
    )
    .bind(template_schema)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    for table in tables {
        let should_copy_data = matches!(
            table.as_str(),
            "realm"
                | "client_app"
                | "roles"
                | "role_permissions"
                | "permissions"
                | "legal_agreement_version"
        );

        let create_table_sql = format!(
            r#"CREATE TABLE "{}".{} (LIKE "{}".{} INCLUDING ALL)"#,
            new_schema, table, template_schema, table
        );

        if let Err(error) = sqlx::query(&create_table_sql).execute(pool).await {
            tracing::warn!("Failed to copy table structure {}: {}", table, error);
            continue;
        }

        if should_copy_data {
            let copy_data_sql = format!(
                r#"INSERT INTO "{}".{} SELECT * FROM "{}".{}"#,
                new_schema, table, template_schema, table
            );

            if let Err(error) = sqlx::query(&copy_data_sql).execute(pool).await {
                tracing::warn!("Failed to copy table data {}: {}", table, error);
            }
        }
    }

    let views: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT table_name, view_definition
        FROM information_schema.views
        WHERE table_schema = $1
        ORDER BY table_name
        "#,
    )
    .bind(template_schema)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    for (view_name, view_definition) in views {
        let create_view_sql = format!(
            r#"CREATE VIEW "{}".{} AS {}"#,
            new_schema, view_name, view_definition
        );

        if let Err(error) = sqlx::query(&create_view_sql).execute(pool).await {
            tracing::warn!("Failed to copy view {}: {}", view_name, error);
        }
    }

    let sequences: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT sequence_name
        FROM information_schema.sequences
        WHERE sequence_schema = $1
        ORDER BY sequence_name
        "#,
    )
    .bind(template_schema)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    for sequence in sequences {
        let create_seq_sql = format!(
            r#"CREATE SEQUENCE "{}"."{}" AS BIGINT"#,
            new_schema, sequence
        );

        if let Err(error) = sqlx::query(&create_seq_sql).execute(pool).await {
            tracing::warn!("Failed to copy sequence {}: {}", sequence, error);
        }
    }
}

pub async fn create_schema_scoped_connections(
    host: &str,
    port: u16,
    schema: &str,
    max_connections: u32,
) -> (PgPool, DatabaseConnection) {
    let db_url = postgres_url_with_schema(host, port, schema);

    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .test_before_acquire(true)
        .connect(&db_url)
        .await
        .expect(
            "❌ Failed to connect to PgDog (sqlx). 测试环境未启动，请运行:\n  uv run scripts/backend-test.py -- <测试文件>\n或先启动环境:\n  uv run scripts/test-start.py",
        );

    let sea_conn = sea_orm::SqlxPostgresConnector::from_sqlx_postgres_pool(pool.clone());
    (pool, sea_conn)
}

pub async fn create_isolated_schema_database(
    max_connections: u32,
) -> (SharedTestDatabaseHandle, PgPool, DatabaseConnection) {
    let shared = get_shared_test_database().await;
    let schema_name = format!(
        "test_{}",
        uuid::Uuid::now_v7().to_string().replace('-', "_")
    );

    clone_schema_from_template(&shared.pool, &shared.template_schema_name, &schema_name).await;
    let (pool, db) = create_schema_scoped_connections(
        &shared.pg_host,
        shared.pg_port,
        &schema_name,
        max_connections,
    )
    .await;

    (
        SharedTestDatabaseHandle {
            cleanup_pool: Arc::clone(&shared.pool),
            schema_name,
        },
        pool,
        db,
    )
}

pub async fn drop_schema(pool: &PgPool, schema_name: &str) {
    sqlx::query(&format!(
        r#"DROP SCHEMA IF EXISTS "{}" CASCADE"#,
        schema_name
    ))
    .execute(pool)
    .await
    .expect("Failed to drop test schema");
}

pub struct SharedTestDatabaseHandle {
    cleanup_pool: Arc<PgPool>,
    pub schema_name: String,
}

impl SharedTestDatabaseHandle {
    pub async fn teardown(self) {
        drop_schema(&self.cleanup_pool, &self.schema_name).await;
    }
}

fn postgres_url(host: &str, port: u16) -> String {
    format!(
        "postgres://postgres:postgres@{}:{}/postgres?sslmode=disable&statement-cache-capacity=0",
        host, port
    )
}

fn postgres_url_with_schema(host: &str, port: u16, schema: &str) -> String {
    let search_path_option = format!("-c%20search_path={}", schema);
    format!(
        "postgres://postgres:postgres@{}:{}/postgres?sslmode=disable&statement-cache-capacity=0&options={}",
        host, port, search_path_option
    )
}

async fn execute_sql_batch_with_do_blocks(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    sql: &str,
) {
    let mut current_statement = String::new();
    let mut in_do_block = false;

    for line in sql.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with("--") {
            continue;
        }

        if trimmed.starts_with("DO $$") {
            in_do_block = true;
            current_statement.push_str(line);
            current_statement.push('\n');
            continue;
        }

        if in_do_block {
            current_statement.push_str(line);
            current_statement.push('\n');

            if trimmed.contains("$$") && trimmed.contains("END") {
                in_do_block = false;
                execute_statement(conn, &current_statement).await;
                current_statement.clear();
            }
            continue;
        }

        current_statement.push_str(line);
        current_statement.push('\n');

        if trimmed.ends_with(';') {
            execute_statement(conn, &current_statement).await;
            current_statement.clear();
        }
    }
}

async fn execute_statement(conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>, statement: &str) {
    if let Err(error) = sqlx::query(statement).execute(&mut **conn).await {
        let err_str = error.to_string().to_lowercase();
        if !err_str.contains("already exists") && !err_str.contains("duplicate") {
            tracing::warn!(
                "Template migration statement failed (non-fatal): {} - SQL: {}",
                error,
                &statement[..100.min(statement.len())]
            );
        }
    }
}
