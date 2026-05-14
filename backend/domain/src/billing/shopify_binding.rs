//! Shopify subscription binding entity and repository
//!
//! Manages the relationship between Herald subscriptions and Shopify subscription contracts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;

/// Shopify subscription binding entity
///
/// Represents the link between a Herald subscription and a Shopify subscription contract.
/// Each binding stores Shopify-specific metadata and enables webhook event processing.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShopifySubscriptionBinding {
    pub id: i64,
    pub subscription_id: Uuid,
    pub realm_id: String,
    pub shop_domain: String,
    pub contract_id: String,
    pub contract_gid: String,
    pub contract_revision_id: i64,
    pub customer_id: Option<String>,
    pub customer_payment_method_id: Option<String>,
    pub last_billing_attempt_id: Option<String>,
    pub last_order_id: Option<String>,
    pub cancel_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Shopify subscription binding repository
///
/// Provides CRUD operations for Shopify subscription bindings.
/// Uses native async trait (no async_trait macro).
#[allow(async_fn_in_trait)]
pub trait ShopifyBindingRepository: Send + Sync {
    /// Create a new Shopify subscription binding
    async fn create_binding(
        &self,
        binding: &ShopifySubscriptionBinding,
    ) -> Result<ShopifySubscriptionBinding, CoreError>;

    /// Find binding by Shopify contract ID
    async fn find_by_contract_id(
        &self,
        contract_id: &str,
    ) -> Result<Option<ShopifySubscriptionBinding>, CoreError>;

    /// Find binding by subscription ID
    async fn find_by_subscription_id(
        &self,
        subscription_id: Uuid,
    ) -> Result<Option<ShopifySubscriptionBinding>, CoreError>;

    /// Find binding by order ID (for refund processing)
    async fn find_by_order_id(
        &self,
        order_id: &str,
    ) -> Result<Option<ShopifySubscriptionBinding>, CoreError>;

    /// Update existing binding
    async fn update_binding(
        &self,
        binding: &ShopifySubscriptionBinding,
    ) -> Result<ShopifySubscriptionBinding, CoreError>;

    /// Delete binding by ID
    async fn delete_binding(&self, id: i64) -> Result<(), CoreError>;

    /// Count active subscriptions by realm
    async fn count_active_by_realm(&self, realm_id: &str) -> Result<i64, CoreError>;
}
