//! Shopify Repository Implementation
//!
//! Provides database access methods for Shopify subscription bindings,
//! payment events, and related configuration.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::ShopifySubscriptionContractWebhook;
use herald_domain::common::entities::app_errors::CoreError;

/// Type alias for subscription query result to reduce type complexity
type SubscriptionQueryResult = (
    Uuid,
    String,
    Option<Uuid>,
    Option<Uuid>,
    String,
    Option<DateTime<Utc>>,
);

/// Shopify subscription binding record
#[derive(Debug, Clone)]
pub struct ShopifyBinding {
    pub id: i64,
    pub subscription_id: Uuid,
    pub realm_id: String,
    pub shop_domain: String,
    pub contract_id: String,
    pub contract_gid: String,
    pub contract_revision_id: i64,
    pub customer_id: Option<String>,
    pub last_billing_attempt_id: Option<String>,
    pub last_order_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ShopifySubscriptionRecord {
    pub id: Uuid,
    pub realm_id: String,
    pub user_id: Option<Uuid>,
    pub plan_id: Option<Uuid>,
    pub status: String,
    pub current_period_end: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct ShopifyUserBindingRecord {
    pub id: i64,
    pub realm_id: String,
    pub shop_domain: String,
    pub shopify_customer_id: String,
    pub shopify_customer_gid: Option<String>,
    pub user_id: Uuid,
    pub status: String,
}

/// Repository for Shopify-specific database operations
pub struct ShopifyRepository {
    pool: PgPool,
}

impl ShopifyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ===== Configuration =====

    /// Get Shopify client secret for a realm
    pub async fn get_client_secret(&self, realm_uuid: &str) -> Result<String, CoreError> {
        let client_secret: String = sqlx::query_scalar(
            "SELECT config_value FROM realm_config
             WHERE realm_id = $1
             AND config_type = 'shopify'
             AND config_key = 'app_client_secret'",
        )
        .bind(realm_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error loading Shopify config: {}", e);
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?
        .ok_or_else(|| {
            tracing::warn!("Shopify config not found for realm: {}", realm_uuid);
            CoreError::NotFound
        })?;

        Ok(client_secret)
    }

    /// Get configured shop domain for a realm
    pub async fn get_shop_domain(&self, realm_id: &str) -> Result<String, CoreError> {
        let shop_domain: String = sqlx::query_scalar(
            "SELECT config_value FROM realm_config
             WHERE realm_id = $1
             AND config_type = 'shopify'
             AND config_key = 'shop_domain'",
        )
        .bind(realm_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CoreError::InternalServerError(format!("Database error: {}", e)))?
        .ok_or(CoreError::NotFound)?;

        Ok(shop_domain)
    }

    // ===== Payment Events =====

    /// Check if payment event exists (for idempotency)
    pub async fn payment_event_exists(&self, external_event_id: &str) -> Result<bool, CoreError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM payment_event
             WHERE external_event_id = $1 AND payment_provider = 'shopify')",
        )
        .bind(external_event_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error checking idempotency: {}", e);
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(exists)
    }

    /// Create a payment event record
    pub async fn create_payment_event(
        &self,
        realm_uuid: &str,
        external_event_id: &str,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<(), CoreError> {
        sqlx::query(
            "INSERT INTO payment_event
             (id, realm_id, external_event_id, payment_provider, event_type, payload, processed, created_at)
             VALUES ($1, $2, $3, 'shopify', $4, $5, false, NOW())"
        )
        .bind(uuid::Uuid::now_v7())
        .bind(realm_uuid)
        .bind(external_event_id)
        .bind(event_type)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error saving payment event: {}", e);
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(())
    }

    // ===== Subscription =====

    /// Create a subscription record
    pub async fn create_subscription(
        &self,
        realm_uuid: &str,
        contract: &ShopifySubscriptionContractWebhook,
        user_id: Option<Uuid>,
        plan_id: Uuid,
        client_app_id: Option<Uuid>,
        billing_period: &str,
    ) -> Result<Uuid, CoreError> {
        let subscription_id = uuid::Uuid::now_v7();

        sqlx::query(
            "INSERT INTO subscription
             (id, realm_id, user_id, external_subscription_id, external_product_id, payment_provider,
              status, tier, current_period_end, plan_id, client_app_id, billing_period)
             VALUES ($1, $2, $3, $4, $5, 'shopify', 'active', 'pro', $6, $7, $8, $9)",
        )
        .bind(subscription_id)
        .bind(realm_uuid)
        .bind(user_id)
        .bind(&contract.id)
        .bind(&contract.selling_plan_id)
        .bind(contract.current_period_end)
        .bind(plan_id)
        .bind(client_app_id)
        .bind(billing_period)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create subscription record: {}", e);
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(subscription_id)
    }

    /// Find subscription by external ID
    pub async fn find_subscription_by_external_id(
        &self,
        external_subscription_id: &str,
    ) -> Result<Option<(Uuid, Option<Uuid>, String)>, CoreError> {
        let result = sqlx::query_as(
            "SELECT id, plan_id, realm_id
             FROM subscription
             WHERE external_subscription_id = $1",
        )
        .bind(external_subscription_id)
        .fetch_optional(&self.pool)
        .await?
        .map(|row: (Uuid, Option<Uuid>, String)| row);

        Ok(result)
    }

    pub async fn find_subscription_with_user(
        &self,
        subscription_id: Uuid,
    ) -> Result<Option<ShopifySubscriptionRecord>, CoreError> {
        let result = sqlx::query_as(
            "SELECT id, realm_id, user_id, plan_id, status, current_period_end
             FROM subscription
             WHERE id = $1",
        )
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CoreError::InternalServerError(format!("Database error: {}", e)))?
        .map(
            |(id, realm_id, user_id, plan_id, status, current_period_end): SubscriptionQueryResult| ShopifySubscriptionRecord {
                id,
                realm_id,
                user_id,
                plan_id,
                status,
                current_period_end,
            },
        );

        Ok(result)
    }

    /// Get subscription plan ID
    pub async fn get_subscription_plan_id(
        &self,
        subscription_id: Uuid,
    ) -> Result<Option<Uuid>, CoreError> {
        let plan_id: Option<Uuid> =
            sqlx::query_scalar("SELECT plan_id FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_optional(&self.pool)
                .await?
                .flatten();

        Ok(plan_id)
    }

    /// Get subscription realm ID
    pub async fn get_subscription_realm_id(
        &self,
        subscription_id: Uuid,
    ) -> Result<String, CoreError> {
        let realm_id: String =
            sqlx::query_scalar("SELECT realm_id FROM subscription WHERE id = $1")
                .bind(subscription_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| {
                    tracing::error!(
                        subscription_id = %subscription_id,
                        "Subscription not found"
                    );
                    CoreError::NotFound
                })?;

        Ok(realm_id)
    }

    /// Update subscription plan
    pub async fn update_subscription_plan(
        &self,
        subscription_id: Uuid,
        plan_id: Uuid,
        current_period_end: DateTime<Utc>,
    ) -> Result<(), CoreError> {
        sqlx::query(
            "UPDATE subscription
             SET plan_id = $1,
                 current_period_end = $2,
                 updated_at = NOW()
             WHERE id = $3",
        )
        .bind(plan_id)
        .bind(current_period_end)
        .bind(subscription_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                subscription_id = %subscription_id,
                error = %e,
                "Failed to update subscription"
            );
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(())
    }

    pub async fn assign_subscription_user(
        &self,
        subscription_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), CoreError> {
        sqlx::query(
            "UPDATE subscription
             SET user_id = $1,
                 updated_at = NOW()
             WHERE id = $2",
        )
        .bind(user_id)
        .bind(subscription_id)
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::InternalServerError(format!("Database error: {}", e)))?;

        Ok(())
    }

    /// Update subscription status
    pub async fn update_subscription_status(
        &self,
        subscription_id: Uuid,
        status: &str,
    ) -> Result<(), CoreError> {
        sqlx::query(
            "UPDATE subscription
             SET status = $1,
                 updated_at = NOW()
             WHERE id = $2",
        )
        .bind(status)
        .bind(subscription_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                subscription_id = %subscription_id,
                error = %e,
                "Failed to update subscription status"
            );
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(())
    }

    pub async fn update_subscription_period_end(
        &self,
        subscription_id: Uuid,
        current_period_end: DateTime<Utc>,
    ) -> Result<(), CoreError> {
        sqlx::query(
            "UPDATE subscription
             SET current_period_end = $1,
                 updated_at = NOW()
             WHERE id = $2",
        )
        .bind(current_period_end)
        .bind(subscription_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                subscription_id = %subscription_id,
                error = %e,
                "Failed to update subscription period end"
            );
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(())
    }

    // ===== Shopify Binding =====

    /// Create Shopify subscription binding
    pub async fn create_binding(
        &self,
        subscription_id: Uuid,
        realm_id: &str,
        shop_domain: &str,
        contract: &ShopifySubscriptionContractWebhook,
    ) -> Result<i64, CoreError> {
        let binding_id: i64 = sqlx::query_scalar(
            "INSERT INTO shopify_subscription_binding
             (subscription_id, realm_id, shop_domain, contract_id, contract_gid,
              contract_revision_id, customer_id, last_order_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id",
        )
        .bind(subscription_id)
        .bind(realm_id)
        .bind(shop_domain)
        .bind(&contract.id)
        .bind(&contract.admin_graphql_api_id)
        .bind(contract.contract_revision_id.unwrap_or(1))
        .bind(&contract.customer_id)
        .bind(&contract.origin_order_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create Shopify binding: {}", e);
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(binding_id)
    }

    /// Find binding by contract ID
    pub async fn find_binding_by_contract_id(
        &self,
        contract_id: &str,
    ) -> Result<Option<(i64, Uuid, i64, Option<String>)>, CoreError> {
        let result = sqlx::query_as(
            "SELECT id, subscription_id, contract_revision_id, customer_id
             FROM shopify_subscription_binding
             WHERE contract_id = $1",
        )
        .bind(contract_id)
        .fetch_optional(&self.pool)
        .await?
        .map(|row: (i64, Uuid, i64, Option<String>)| row);

        Ok(result)
    }

    pub async fn find_subscription_id_by_contract_id(
        &self,
        contract_id: &str,
    ) -> Result<Option<Uuid>, CoreError> {
        let result: Option<Uuid> = sqlx::query_scalar(
            "SELECT subscription_id
             FROM shopify_subscription_binding
             WHERE contract_id = $1",
        )
        .bind(contract_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CoreError::InternalServerError(format!("Database error: {}", e)))?;

        Ok(result)
    }

    /// Find binding by contract ID (for billing attempts - returns different fields)
    pub async fn find_binding_for_billing(
        &self,
        contract_id: &str,
    ) -> Result<Option<(i64, Uuid, i64, String)>, CoreError> {
        let result = sqlx::query_as(
            "SELECT id, subscription_id, contract_revision_id, customer_id
             FROM shopify_subscription_binding
             WHERE contract_id = $1",
        )
        .bind(contract_id)
        .fetch_optional(&self.pool)
        .await?
        .map(|row: (i64, Uuid, i64, String)| row);

        Ok(result)
    }

    /// Find binding by order ID (for refunds)
    pub async fn find_binding_by_order_id(
        &self,
        order_id: &str,
    ) -> Result<Option<(Uuid, Option<String>)>, CoreError> {
        let result = sqlx::query_as(
            "SELECT subscription_id, customer_id
             FROM shopify_subscription_binding
             WHERE last_order_id = $1",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await?
        .map(|row: (Uuid, Option<String>)| row);

        Ok(result)
    }

    pub async fn find_subscription_id_by_order_id(
        &self,
        order_id: &str,
    ) -> Result<Option<Uuid>, CoreError> {
        let result: Option<Uuid> = sqlx::query_scalar(
            "SELECT subscription_id
             FROM shopify_subscription_binding
             WHERE last_order_id = $1",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CoreError::InternalServerError(format!("Database error: {}", e)))?;

        Ok(result)
    }

    /// Find binding by subscription ID (for failures)
    pub async fn find_binding_by_subscription_id(
        &self,
        subscription_id: Uuid,
    ) -> Result<Option<(Uuid, i64)>, CoreError> {
        let result = sqlx::query_as(
            "SELECT subscription_id, id
             FROM shopify_subscription_binding
             WHERE subscription_id = $1",
        )
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?
        .map(|row: (Uuid, i64)| row);

        Ok(result)
    }

    /// Update binding with new billing attempt
    pub async fn update_binding_billing_attempt(
        &self,
        binding_id: i64,
        billing_attempt_id: &str,
        order_id: Option<&str>,
    ) -> Result<(), CoreError> {
        sqlx::query(
            "UPDATE shopify_subscription_binding
             SET last_billing_attempt_id = $1,
                 last_order_id = $2,
                 updated_at = NOW()
             WHERE id = $3",
        )
        .bind(billing_attempt_id)
        .bind(order_id)
        .bind(binding_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                binding_id = %binding_id,
                error = %e,
                "Failed to update Shopify binding"
            );
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(())
    }

    /// Update binding revision ID
    pub async fn update_binding_revision(
        &self,
        binding_id: i64,
        revision_id: i64,
    ) -> Result<(), CoreError> {
        sqlx::query(
            "UPDATE shopify_subscription_binding
             SET contract_revision_id = $1,
                 updated_at = NOW()
             WHERE id = $2",
        )
        .bind(revision_id)
        .bind(binding_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                binding_id = %binding_id,
                error = %e,
                "Failed to update Shopify binding revision"
            );
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(())
    }

    /// Update binding with billing attempt ID only (for failures)
    pub async fn update_binding_billing_attempt_id(
        &self,
        binding_id: i64,
        billing_attempt_id: &str,
    ) -> Result<(), CoreError> {
        sqlx::query(
            "UPDATE shopify_subscription_binding
             SET last_billing_attempt_id = $1,
                 updated_at = NOW()
             WHERE id = $2",
        )
        .bind(billing_attempt_id)
        .bind(binding_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                binding_id = %binding_id,
                error = %e,
                "Failed to update Shopify binding with billing attempt ID"
            );
            CoreError::InternalServerError(format!("Database error: {}", e))
        })?;

        Ok(())
    }

    // ===== Shopify User Binding =====

    pub async fn find_user_binding_by_customer_id(
        &self,
        realm_id: &str,
        shop_domain: &str,
        shopify_customer_id: &str,
    ) -> Result<Option<ShopifyUserBindingRecord>, CoreError> {
        let result = sqlx::query_as(
            "SELECT id, realm_id, shop_domain, shopify_customer_id, shopify_customer_gid, user_id, status
             FROM shopify_user_binding
             WHERE realm_id = $1
               AND shop_domain = $2
               AND shopify_customer_id = $3",
        )
        .bind(realm_id)
        .bind(shop_domain)
        .bind(shopify_customer_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CoreError::InternalServerError(format!("Database error: {}", e)))?
        .map(
            |(
                id,
                realm_id,
                shop_domain,
                shopify_customer_id,
                shopify_customer_gid,
                user_id,
                status,
            ): (
                i64,
                String,
                String,
                String,
                Option<String>,
                Uuid,
                String,
            )| ShopifyUserBindingRecord {
                id,
                realm_id,
                shop_domain,
                shopify_customer_id,
                shopify_customer_gid,
                user_id,
                status,
            },
        );

        Ok(result)
    }

    pub async fn upsert_user_binding(
        &self,
        realm_id: &str,
        shop_domain: &str,
        shopify_customer_id: &str,
        shopify_customer_gid: Option<&str>,
        user_id: Uuid,
    ) -> Result<ShopifyUserBindingRecord, CoreError> {
        let result = sqlx::query_as(
            "INSERT INTO shopify_user_binding
             (realm_id, shop_domain, shopify_customer_id, shopify_customer_gid, user_id, status)
             VALUES ($1, $2, $3, $4, $5, 'active')
             ON CONFLICT (realm_id, shop_domain, shopify_customer_id)
             DO UPDATE SET
                 shopify_customer_gid = EXCLUDED.shopify_customer_gid,
                 user_id = EXCLUDED.user_id,
                 status = 'active',
                 updated_at = NOW()
             RETURNING id, realm_id, shop_domain, shopify_customer_id, shopify_customer_gid, user_id, status",
        )
        .bind(realm_id)
        .bind(shop_domain)
        .bind(shopify_customer_id)
        .bind(shopify_customer_gid)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| CoreError::InternalServerError(format!("Database error: {}", e)))?;

        let (id, realm_id, shop_domain, shopify_customer_id, shopify_customer_gid, user_id, status): (
            i64,
            String,
            String,
            String,
            Option<String>,
            Uuid,
            String,
        ) = result;

        Ok(ShopifyUserBindingRecord {
            id,
            realm_id,
            shop_domain,
            shopify_customer_id,
            shopify_customer_gid,
            user_id,
            status,
        })
    }

    pub async fn list_unclaimed_subscriptions_by_customer(
        &self,
        realm_id: &str,
        shop_domain: &str,
        shopify_customer_id: &str,
    ) -> Result<Vec<ShopifySubscriptionRecord>, CoreError> {
        let rows = sqlx::query_as(
            "SELECT s.id, s.realm_id, s.user_id, s.plan_id, s.status, s.current_period_end
             FROM subscription s
             JOIN shopify_subscription_binding b ON b.subscription_id = s.id
             WHERE s.realm_id = $1
               AND b.shop_domain = $2
               AND b.customer_id = $3
               AND s.user_id IS NULL",
        )
        .bind(realm_id)
        .bind(shop_domain)
        .bind(shopify_customer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::InternalServerError(format!("Database error: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(
                |(id, realm_id, user_id, plan_id, status, current_period_end): SubscriptionQueryResult| ShopifySubscriptionRecord {
                    id,
                    realm_id,
                    user_id,
                    plan_id,
                    status,
                    current_period_end,
                },
            )
            .collect())
    }

    pub async fn get_customer_id_by_contract_id(
        &self,
        contract_id: &str,
    ) -> Result<Option<String>, CoreError> {
        let result: Option<String> = sqlx::query_scalar(
            "SELECT customer_id
             FROM shopify_subscription_binding
             WHERE contract_id = $1",
        )
        .bind(contract_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CoreError::InternalServerError(format!("Database error: {}", e)))?;

        Ok(result)
    }

    pub async fn get_customer_id_by_order_id(
        &self,
        order_id: &str,
    ) -> Result<Option<String>, CoreError> {
        let result: Option<String> = sqlx::query_scalar(
            "SELECT customer_id
             FROM shopify_subscription_binding
             WHERE last_order_id = $1",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| CoreError::InternalServerError(format!("Database error: {}", e)))?;

        Ok(result)
    }
}
