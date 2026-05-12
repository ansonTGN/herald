// =============================================================================
// 共享容器模块 - 外部容器模式
// =============================================================================

use sqlx::PgPool;
use std::sync::Arc;

pub struct SharedContainers {
    pub pool: Arc<PgPool>,
    pub pg_host: String,
    pub pg_port: u16,
    pub redis_url: String,
    pub template_schema_name: String,
}

static SHARED_CONTAINERS: tokio::sync::OnceCell<SharedContainers> =
    tokio::sync::OnceCell::const_new();

impl SharedContainers {
    pub async fn get() -> &'static SharedContainers {
        SHARED_CONTAINERS
            .get_or_init(|| async {
                tracing::info!("🔗 连接到外部测试容器");

                let shared = herald_test_db::get_shared_test_database().await;
                let redis_url = std::env::var("TEST_REDIS_URL")
                    .unwrap_or_else(|_| "redis://127.0.0.1:6380/0".to_string());

                tracing::info!("✅ 外部测试容器连接完成");

                SharedContainers {
                    pool: Arc::clone(&shared.pool),
                    pg_host: shared.pg_host.clone(),
                    pg_port: shared.pg_port,
                    redis_url,
                    template_schema_name: shared.template_schema_name.clone(),
                }
            })
            .await
    }
}
