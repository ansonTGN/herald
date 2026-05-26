// =============================================================================
// Bare Schema Test Context - 轻量级数据库测试上下文
// =============================================================================
//
// 适用于只需要真实数据库 schema 隔离的测试
// 不需要完整 AppState、admin user、RBAC、permission checker
//
// =============================================================================

use herald_test_db::{clone_schema_from_template, create_schema_scoped_connections};
use sqlx::PgPool;
use std::sync::Arc;
use test_context::AsyncTestContext;

const SCHEMA_POOL_MAX_CONNECTIONS: u32 = 3;

/// 轻量级 Schema 隔离测试上下文
///
/// 提供：
/// - schema 名
/// - sqlx::PgPool
/// - sea_orm::DatabaseConnection
///
/// 不负责：
/// - admin seed
/// - Redis manager
/// - 完整 service graph
pub struct BareSchemaTestContext {
    /// Schema 名称
    pub schema_name: String,
    /// SQLx 连接池
    pub pool: Arc<PgPool>,
    /// Sea-ORM 连接
    pub db: Arc<sea_orm::DatabaseConnection>,
    /// 原始连接池（用于清理）
    cleanup_pool: Arc<PgPool>,
}

impl AsyncTestContext for BareSchemaTestContext {
    async fn setup() -> Self {
        let _ = tracing_subscriber::fmt().try_init();

        // 1. 获取共享容器（连接到主数据库）
        let shared = crate::shared::SharedContainers::get().await;

        // 2. 生成唯一的 Schema 名称
        let schema_name = format!(
            "test_{}",
            uuid::Uuid::now_v7().to_string().replace("-", "_")
        );

        tracing::debug!("📦 创建轻量级测试 Schema: {}", schema_name);

        // 3. 从模板 Schema 克隆新 Schema
        clone_schema_from_template(&shared.pool, &shared.template_schema_name, &schema_name).await;

        // 4. 创建带 Schema 的连接池
        let (pool_with_schema, sea_conn) = create_schema_scoped_connections(
            &shared.pg_host,
            shared.pg_port,
            &schema_name,
            SCHEMA_POOL_MAX_CONNECTIONS,
        )
        .await;

        tracing::debug!(
            schema_name = %schema_name,
            "BareSchemaTestContext setup completed"
        );

        Self {
            schema_name,
            pool: Arc::new(pool_with_schema),
            db: Arc::new(sea_conn),
            cleanup_pool: shared.pool.clone(),
        }
    }

    async fn teardown(self) {
        let Self {
            ref schema_name,
            pool,
            db,
            cleanup_pool,
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
