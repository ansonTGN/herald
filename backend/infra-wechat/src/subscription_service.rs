//! WeChat Pay subscription service
//!
//! Handles subscription creation and points granting for WeChat Pay payments.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use herald_domain::common::entities::app_errors::CoreError;

/// Service for handling WeChat Pay subscription operations
pub struct WechatSubscriptionService {
    pool: PgPool,
}

impl WechatSubscriptionService {
    /// Create a new subscription service instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create subscription and grant points after successful payment
    ///
    /// ⚠️ ARCHITECTURAL DEBT (P3-04): This method contains too much business logic
    /// for an Infrastructure layer service. It handles:
    /// - Subscription creation (billing domain)
    /// - Points calculation and granting (points domain)
    /// - Payment event recording (billing domain)
    ///
    /// TODO: Refactor in future WeChat billing tasks:
    /// - Extract points/subscription calculation logic to Application layer
    /// - Use domain services for business rules
    /// - Keep this service focused on WeChat API integration only
    ///
    /// This method:
    /// 1. Loads the entitlement mapping details to get points amount and billing period
    /// 2. Creates a subscription record
    /// 3. Grants points to the user
    /// 4. Records the payment event for idempotency
    pub async fn create_subscription_and_grant_points(
        &self,
        realm_id: &str,
        user_id: Uuid,
        mapping_id: Uuid,
        transaction_id: &str,
        _amount: i32, // Amount is already verified in webhook handler
    ) -> Result<SubscriptionResult, CoreError> {
        // Load entitlement mapping details
        let mapping_row = sqlx::query(
            r#"
            SELECT id, billing_period, points_per_period
            FROM provider_entitlement_mappings WHERE id = $1
            "#,
        )
        .bind(mapping_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(mapping_id = %mapping_id, error = %e, "Failed to load entitlement mapping");
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        let mapping_row = mapping_row.ok_or_else(|| {
            tracing::error!(mapping_id = %mapping_id, "Entitlement mapping not found");
            CoreError::BillingError("Entitlement mapping not found".to_string())
        })?;

        // Get billing period and points
        let billing_period: String = mapping_row.try_get("billing_period").unwrap_or_else(|_| {
            tracing::warn!(
                mapping_id = %mapping_id,
                "Failed to read billing period from database, defaulting to 'monthly'"
            );
            "monthly".to_string()
        });
        let points_amount: i32 = mapping_row
            .try_get("points_per_period")
            .unwrap_or_else(|_| {
                tracing::warn!(
                    mapping_id = %mapping_id,
                    "Failed to read points_per_period from database, defaulting to 0"
                );
                0
            });

        // Calculate subscription expiration based on billing period
        let now = Utc::now();
        let expires_at: Option<DateTime<Utc>> = match billing_period.as_str() {
            "monthly" => Some(now + chrono::Duration::days(30)),
            "yearly" => Some(now + chrono::Duration::days(365)),
            _ => None,
        };

        // Create subscription
        let subscription_id = Uuid::now_v7();
        sqlx::query(
            r#"
            INSERT INTO subscription (id, user_id, realm_id, entitlement_key, payment_provider, status, external_subscription_id, external_product_id, current_period_end, created_at, updated_at)
            SELECT $1, $2, $3, entitlement_key, 'wechat', 'active', $4, external_product_id, $5, $6, $6
            FROM provider_entitlement_mappings
            WHERE id = $7
            "#,
        )
        .bind(subscription_id)
        .bind(user_id)
        .bind(realm_id)
        .bind(transaction_id)
        .bind(expires_at)
        .bind(now)
        .bind(mapping_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                subscription_id = %subscription_id,
                error = %e,
                "Failed to create subscription"
            );
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        // Grant points
        // First, get the user's points account
        let account_row = sqlx::query(
            r#"
            SELECT id, subscription_balance
            FROM points_wallets
            WHERE user_id = $1 AND realm_id = $2
            "#,
        )
        .bind(user_id)
        .bind(realm_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                subscription_id = %subscription_id,
                user_id = %user_id,
                error = %e,
                "Failed to load points account"
            );
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        let account_row = account_row.ok_or_else(|| {
            tracing::error!(
                subscription_id = %subscription_id,
                user_id = %user_id,
                "Points account not found"
            );
            CoreError::InternalServerError("Points account not found".to_string())
        })?;

        let account_id: Uuid = account_row.try_get("id").map_err(|e| {
            tracing::error!(
                subscription_id = %subscription_id,
                user_id = %user_id,
                error = %e,
                "Failed to read account id from points_wallets row"
            );
            CoreError::DatabaseError(format!("Failed to read account id: {}", e))
        })?;
        let current_balance: i64 = account_row.try_get("subscription_balance").unwrap_or_else(
            |_| {
                tracing::warn!(
                    subscription_id = %subscription_id,
                    user_id = %user_id,
                    "Failed to read subscription_balance from points_wallets row, defaulting to 0"
                );
                0
            },
        );
        let new_balance = current_balance + points_amount as i64;

        // Insert points transaction
        sqlx::query(
            r#"
            INSERT INTO points_transactions (id, account_id, user_id, realm_id, type, amount, balance_after, subscription_balance_after, subscription_id, created_at)
            VALUES ($1, $2, $3, $4, 'subscription_grant', $5, $6, $7, $8, $9)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(account_id)
        .bind(user_id)
        .bind(realm_id)
        .bind(points_amount)
        .bind(new_balance)
        .bind(new_balance)
        .bind(subscription_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                subscription_id = %subscription_id,
                user_id = %user_id,
                error = %e,
                "Failed to grant points"
            );
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        // Update points account balance (total_balance is auto-computed)
        sqlx::query(
            r#"
            UPDATE points_wallets
            SET subscription_balance = subscription_balance + $1,
                total_subscription_granted = total_subscription_granted + $1,
                updated_at = $2
            WHERE id = $3
            "#,
        )
        .bind(points_amount)
        .bind(now)
        .bind(account_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                subscription_id = %subscription_id,
                user_id = %user_id,
                error = %e,
                "Failed to update points account balance"
            );
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        // Record payment event for idempotency
        let event_id = transaction_id.to_string();
        sqlx::query(
            r#"
            INSERT INTO payment_event (id, external_event_id, payment_provider, event_type, realm_id, subscription_id, processed, created_at)
            VALUES ($1, $2, 'wechat', 'pay.success', $3, $4, true, $5)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(&event_id)
        .bind(realm_id)
        .bind(subscription_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                event_id = %event_id,
                error = %e,
                "Failed to record payment event"
            );
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        tracing::info!(
            subscription_id = %subscription_id,
            user_id = %user_id,
            points_amount = points_amount,
            "Created subscription and granted points"
        );

        Ok(SubscriptionResult {
            subscription_id,
            points_amount,
            expires_at,
        })
    }
}

/// Result of subscription creation
#[derive(Debug, Clone)]
pub struct SubscriptionResult {
    pub subscription_id: Uuid,
    pub points_amount: i32,
    pub expires_at: Option<DateTime<Utc>>,
}
