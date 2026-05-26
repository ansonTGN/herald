// =============================================================================
// Auth Schema Test Context - 需要认证的测试上下文
// =============================================================================
//
// 适用于需要登录、会话、基础 auth 流程的测试
// 需要 admin realm / admin user
// 但不需要完整 billing / points / purchase 服务
//
// =============================================================================

use herald_core::admin::init_admin_realm_rbac;
use herald_core::admin::user::init_admin_user;
use herald_core::application::{ApplicationServiceBuilder, WebhookService};
use herald_core::infrastructure::authorization::{RedisCache, RedisPermissionChecker};
use herald_core::infrastructure::redis::{ManagerConfig, RedisConnectionManager};
use herald_core::infrastructure::user::repositories::PostgresUserRepository;
use herald_core::infrastructure::webhook::WebhookEventRepository;
use sqlx::Row;
use std::sync::Arc;
use test_context::AsyncTestContext;

/// 认证测试上下文
///
/// 在 BareSchemaTestContext 基础上补充：
/// - admin realm RBAC
/// - admin user
/// - auth 所需最小 AppState 组装
pub struct AuthSchemaTestContext {
    /// Schema 名称
    pub schema_name: String,
    /// Realm ID
    pub realm_id: String,
    /// Client ID (external identifier like 'admin-web-console')
    pub client_id: String,
    /// Client App UUID (internal database ID)
    pub client_app_id: String,
    /// SQLx 连接池
    pub pool: Arc<sqlx::PgPool>,
    /// Sea-ORM 连接
    pub db: Arc<sea_orm::DatabaseConnection>,
    /// Redis 管理器
    pub redis_manager: Arc<RedisConnectionManager>,
    /// 权限检查器
    pub permission_checker: Arc<RedisPermissionChecker>,
    /// 应用服务
    pub application_service: herald_core::application::ApplicationService,
    /// 用户存储库
    pub user_repository: Arc<PostgresUserRepository>,
    /// Webhook 服务
    pub webhook_service: Arc<WebhookService>,
    /// 原始连接池（用于清理）
    cleanup_pool: Arc<sqlx::PgPool>,
}

impl AsyncTestContext for AuthSchemaTestContext {
    async fn setup() -> Self {
        let _ = tracing_subscriber::fmt().try_init();

        // 1. 获取共享容器（连接到主数据库）
        let shared = crate::shared::SharedContainers::get().await;

        // 2. 生成唯一的 Schema 名称
        let schema_name = format!(
            "test_{}",
            uuid::Uuid::now_v7().to_string().replace("-", "_")
        );

        tracing::debug!("📦 创建认证测试 Schema: {}", schema_name);

        // 3. 从模板 Schema 克隆新 Schema
        herald_test_db::clone_schema_from_template(
            &shared.pool,
            &shared.template_schema_name,
            &schema_name,
        )
        .await;

        // 4. 创建带 Schema 的连接池
        let (pool_with_schema, sea_conn) = herald_test_db::create_schema_scoped_connections(
            &shared.pg_host,
            shared.pg_port,
            &schema_name,
            3, // SCHEMA_POOL_MAX_CONNECTIONS
        )
        .await;

        // 5. 创建 RedisConnectionManager（测试模式使用 DB 1）
        let redis_url_with_db = if let Some(last_slash_pos) = shared.redis_url.rfind('/') {
            let after_slash = &shared.redis_url[last_slash_pos + 1..];
            if after_slash.chars().all(|c| c.is_ascii_digit()) {
                format!("{}{}", &shared.redis_url[..last_slash_pos + 1], "1")
            } else {
                format!("{}/1", shared.redis_url)
            }
        } else {
            format!("{}/1", shared.redis_url)
        };

        let redis_config = ManagerConfig {
            url: redis_url_with_db,
            default_db: 1,
            test_mode: false,
            test_db: 1,
        };

        let redis_manager = Arc::new(
            RedisConnectionManager::new(redis_config)
                .await
                .expect("Failed to create RedisConnectionManager"),
        );

        // 6. 创建 RedisPermissionChecker
        let redis_cache =
            RedisCache::new((*redis_manager).clone()).expect("Failed to create Redis cache");
        let permission_checker = Arc::new(RedisPermissionChecker::new(
            Arc::new(sea_conn.clone()),
            Arc::new(tokio::sync::RwLock::new(redis_cache)),
        ));

        // 7. 构建 ApplicationService（必须在 init_admin_realm_rbac 之前）
        let application_service = ApplicationServiceBuilder::new()
            .with_database(Arc::new(sea_conn.clone()))
            .with_redis((*redis_manager).clone())
            .with_permission_checker(permission_checker.clone())
            .build()
            .expect("Failed to build ApplicationService");

        // 8. 初始化 admin realm RBAC（必须在 init_admin_user 之前）
        let rbac_init_service = application_service.realm_service().get_rbac_init_service();
        init_admin_realm_rbac(&pool_with_schema, rbac_init_service)
            .await
            .expect("Failed to initialize admin realm RBAC");

        // 9. 初始化 admin user
        init_admin_user(&pool_with_schema, "test")
            .await
            .expect("Failed to initialize admin user");

        // 10. 获取 realm_id 和 client_id
        let realm_id: String = sqlx::query("select id from realm limit 1")
            .fetch_one(&pool_with_schema)
            .await
            .map(|x| x.get("id"))
            .expect("Failed to get realm_id");
        let (client_id, client_app_id): (String, String) = sqlx::query_as(
            "select client_id, id::text from client_app where client_id = 'admin-web-console' and realm_id = $1 limit 1"
        )
            .bind(&realm_id)
            .fetch_one(&pool_with_schema)
            .await
            .expect("Failed to get admin-web-console client_id and UUID");

        // 11. 创建基础服务
        let user_repository = Arc::new(PostgresUserRepository::new(sea_conn.clone().into()));
        let webhook_service = Arc::new(WebhookService::new(Arc::new(WebhookEventRepository::new(
            pool_with_schema.clone(),
        ))));

        tracing::debug!(
            schema_name = %schema_name,
            "AuthSchemaTestContext setup completed"
        );

        Self {
            schema_name,
            realm_id,
            client_id,
            client_app_id,
            pool: Arc::new(pool_with_schema),
            db: Arc::new(sea_conn),
            redis_manager,
            permission_checker,
            application_service,
            user_repository,
            webhook_service,
            cleanup_pool: shared.pool.clone(),
        }
    }

    async fn teardown(self) {
        let Self {
            ref schema_name,
            pool,
            db,
            cleanup_pool,
            ..
        } = self;

        // 关闭连接池
        pool.close().await;
        let _ = (*db).clone().close().await;

        // 使用共享的 schema 清理逻辑
        if let Err(error) =
            crate::helpers::cleanup_schema_if_needed(schema_name, &cleanup_pool).await
        {
            tracing::warn!(schema_name = %schema_name, %error, "Failed to drop test schema");
        }
    }
}
