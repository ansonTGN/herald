use herald_core::infrastructure::wechat::client::WechatPayClient;
use herald_core::infrastructure::wechat::models::WechatPaymentOrder;
use herald_core::infrastructure::wechat::repository::{WechatConfigRow, WechatOrderRepository};
use sqlx::PgPool;
use tracing::{info, warn};

const DEFAULT_BATCH_SIZE: i64 = 200;

#[derive(Debug, Default)]
pub struct WechatOrderExpiryResult {
    pub candidates: i64,
    pub closed: i64,
    pub paid: i64,
    pub errors: i64,
}

pub struct WechatOrderExpiryJob {
    pool: PgPool,
    batch_size: i64,
}

impl WechatOrderExpiryJob {
    pub fn new(pool: PgPool) -> Self {
        let batch_size = std::env::var("WORKER_WECHAT_EXPIRY_BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(DEFAULT_BATCH_SIZE);
        Self { pool, batch_size }
    }

    pub async fn run(&self) -> anyhow::Result<WechatOrderExpiryResult> {
        let repo = WechatOrderRepository::new(self.pool.clone());
        let orders = self.list_expired_pending_orders().await?;
        let mut result = WechatOrderExpiryResult {
            candidates: orders.len() as i64,
            ..Default::default()
        };

        for order in orders {
            match self.process_order(&repo, order).await {
                Ok(ProcessOutcome::Paid) => result.paid += 1,
                Ok(ProcessOutcome::Closed) => result.closed += 1,
                Err(e) => {
                    result.errors += 1;
                    warn!(error = %e, "Failed to process expired WeChat order");
                }
            }
        }

        Ok(result)
    }

    async fn list_expired_pending_orders(&self) -> anyhow::Result<Vec<WechatPaymentOrder>> {
        let rows = sqlx::query_as::<_, (uuid::Uuid,)>(
            "SELECT id
             FROM wechat_payment_order
             WHERE status = 'pending' AND expires_at <= NOW()
             ORDER BY expires_at ASC
             LIMIT $1",
        )
        .bind(self.batch_size)
        .fetch_all(&self.pool)
        .await?;

        let repo = WechatOrderRepository::new(self.pool.clone());
        let mut orders = Vec::with_capacity(rows.len());
        for (id,) in rows {
            if let Some(order) = repo.find_order_by_id(id).await? {
                orders.push(order);
            }
        }

        Ok(orders)
    }

    async fn process_order(
        &self,
        repo: &WechatOrderRepository,
        order: WechatPaymentOrder,
    ) -> anyhow::Result<ProcessOutcome> {
        let config = repo
            .get_wechat_config(&order.realm_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("WeChat config not found for realm {}", order.realm_id)
            })?;
        let client = create_wechat_client_from_config(config).await?;
        let remote_status = client.query_order(&order.out_trade_no).await?;

        match remote_status.trade_state.as_str() {
            "SUCCESS" => {
                let transaction_id = remote_status.transaction_id.ok_or_else(|| {
                    anyhow::anyhow!("WeChat SUCCESS response missing transaction_id")
                })?;
                repo.mark_order_paid(order.id, &transaction_id).await?;
                info!(order_id = %order.id, "Expired WeChat order was already paid upstream");
                Ok(ProcessOutcome::Paid)
            }
            "CLOSED" | "REVOKED" | "PAYERROR" => {
                repo.mark_order_closed(order.id).await?;
                Ok(ProcessOutcome::Closed)
            }
            _ => {
                client.close_order(&order.out_trade_no).await?;
                repo.mark_order_closed(order.id).await?;
                Ok(ProcessOutcome::Closed)
            }
        }
    }
}

enum ProcessOutcome {
    Paid,
    Closed,
}

async fn create_wechat_client_from_config(
    config: WechatConfigRow,
) -> anyhow::Result<WechatPayClient> {
    let app_id = config
        .app_id
        .ok_or_else(|| anyhow::anyhow!("Missing app_id"))?;
    let mch_id = config
        .mch_id
        .ok_or_else(|| anyhow::anyhow!("Missing mch_id"))?;
    let private_key = config
        .private_key
        .ok_or_else(|| anyhow::anyhow!("Missing private_key"))?;
    let serial_no = config
        .serial_no
        .ok_or_else(|| anyhow::anyhow!("Missing serial_no"))?;
    let v3_key = config
        .v3_key
        .ok_or_else(|| anyhow::anyhow!("Missing v3_key"))?;
    let notify_url = config
        .notify_url
        .ok_or_else(|| anyhow::anyhow!("Missing notify_url"))?;

    WechatPayClient::new_async(
        app_id,
        mch_id,
        private_key,
        serial_no,
        v3_key,
        notify_url,
        config.mock_base_url,
    )
    .await
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}
