// =============================================================================
// Test Fixtures Module
// =============================================================================
//
// Reusable test fixtures to reduce initialization overhead.
// Fixtures are created once and can be shared across multiple tests.
//
// =============================================================================

use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::OnceCell;

/// Test Realm Fixture
///
/// Pre-configured realm with RBAC permissions.
/// Can be shared across multiple tests that need realm data.
pub struct TestRealmFixture {
    pub realm_id: String,
    pub admin_role_id: String,
    pub user_role_id: String,
}

impl TestRealmFixture {
    /// Create a new test realm fixture
    pub async fn new(pool: &PgPool, realm_name: &str) -> Result<Self, sqlx::Error> {
        let realm_id = uuid::Uuid::now_v7().to_string();

        // Create realm
        sqlx::query(r#"INSERT INTO realm (id, name) VALUES ($1, $2)"#)
            .bind(&realm_id)
            .bind(realm_name)
            .execute(pool)
            .await?;

        // Create admin role
        let admin_role_id = uuid::Uuid::now_v7().to_string();
        sqlx::query(r#"INSERT INTO role (id, name, realm_id) VALUES ($1, 'realm-admin', $2)"#)
            .bind(&admin_role_id)
            .bind(&realm_id)
            .execute(pool)
            .await?;

        // Create user role
        let user_role_id = uuid::Uuid::now_v7().to_string();
        sqlx::query(r#"INSERT INTO role (id, name, realm_id) VALUES ($1, 'user', $2)"#)
            .bind(&user_role_id)
            .bind(&realm_id)
            .execute(pool)
            .await?;

        Ok(Self {
            realm_id,
            admin_role_id,
            user_role_id,
        })
    }
}

/// Test User Fixture
///
/// Pre-configured user with test credentials.
pub struct TestUserFixture {
    pub user_id: String,
    pub email: String,
    pub username: String,
    pub password: String,
}

impl TestUserFixture {
    /// Create a new test user fixture
    pub async fn new(
        pool: &PgPool,
        email: &str,
        username: &str,
        password: &str,
        realm_id: &str,
    ) -> Result<Self, sqlx::Error> {
        let user_id = uuid::Uuid::now_v7().to_string();

        // Create user
        sqlx::query(
            r#"INSERT INTO account (id, realm_id, email, username, password, status) VALUES ($1, $2, $3, $4, $5, 1)"#
        )
        .bind(&user_id)
        .bind(realm_id)
        .bind(email)
        .bind(username)
        .bind(password)
        .execute(pool)
        .await?;

        Ok(Self {
            user_id,
            email: email.to_string(),
            username: username.to_string(),
            password: password.to_string(),
        })
    }
}

/// Test Redis Fixture
///
/// Shared Redis connection for tests.
/// Uses OnceCell to ensure only one connection is created.
pub struct TestRedisFixture {
    pub client: Arc<redis::Client>,
}

static REDIS_FIXTURE: OnceCell<TestRedisFixture> = OnceCell::const_new();

impl TestRedisFixture {
    /// Get or create the shared Redis fixture
    pub async fn get() -> &'static TestRedisFixture {
        REDIS_FIXTURE
            .get_or_init(|| async {
                // Get Redis URL from environment or use default
                let redis_url = std::env::var("TEST_REDIS_URL")
                    .unwrap_or_else(|_| "redis://127.0.0.1:6382/1".to_string());

                let client =
                    redis::Client::open(redis_url.as_str()).expect(
                        "❌ Failed to open Redis client. 测试环境未启动，请运行:\n  uv run scripts/backend-test.py -- <测试文件>\n或先启动环境:\n  uv run scripts/test-start.py",
                    );

                TestRedisFixture {
                    client: Arc::new(client),
                }
            })
            .await
    }

    /// Get a Redis connection from the shared client
    pub async fn get_connection(
        &self,
    ) -> Result<redis::aio::MultiplexedConnection, redis::RedisError> {
        self.client.get_multiplexed_async_connection().await
    }
}
