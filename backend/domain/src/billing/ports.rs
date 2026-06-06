use std::future::Future;

use uuid::Uuid;

use crate::billing::{
    EntitlementMapping, PaymentEvent, Subscription, SubscriptionHistoryEvent,
    SubscriptionHistoryQuery,
};
use crate::common::entities::app_errors::CoreError;

/// Repository for billing operations
pub trait BillingRepository: Send + Sync {
    /// Create a new subscription
    fn create_subscription(
        &self,
        sub: Subscription,
    ) -> impl Future<Output = Result<Subscription, CoreError>> + Send;

    /// Find subscription by realm ID
    fn find_by_realm_id(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = Result<Option<Subscription>, CoreError>> + Send;

    /// Find subscription by external subscription ID and provider
    fn find_by_external_subscription_id(
        &self,
        external_sub_id: &str,
        provider: &str,
    ) -> impl Future<Output = Result<Option<Subscription>, CoreError>> + Send;

    /// Find subscription by local subscription ID
    fn find_subscription_by_id(
        &self,
        subscription_id: Uuid,
    ) -> impl Future<Output = Result<Option<Subscription>, CoreError>> + Send;

    /// Update subscription
    fn update_subscription(
        &self,
        sub: Subscription,
    ) -> impl Future<Output = Result<Subscription, CoreError>> + Send;

    /// Create a payment event record
    fn create_payment_event(
        &self,
        event: PaymentEvent,
    ) -> impl Future<Output = Result<PaymentEvent, CoreError>> + Send;

    /// Find payment event by external event ID and provider (for idempotency)
    fn find_payment_event_by_external_id(
        &self,
        external_event_id: &str,
        payment_provider: &str,
    ) -> impl Future<Output = Result<Option<PaymentEvent>, CoreError>> + Send;

    /// Mark payment event as processed
    fn mark_payment_event_processed(
        &self,
        id: Uuid,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Find subscription by client app ID
    fn find_subscription_by_client_app_id(
        &self,
        client_app_id: Uuid,
    ) -> impl Future<Output = Result<Option<Subscription>, CoreError>> + Send;

    /// Cancel subscription
    fn cancel_subscription(
        &self,
        subscription_id: Uuid,
        cancel_at_period_end: bool,
    ) -> impl Future<Output = Result<Subscription, CoreError>> + Send;

    // ===== Subscription History =====
    /// Save a subscription history event
    fn save_history_event(
        &self,
        event: SubscriptionHistoryEvent,
    ) -> impl Future<Output = Result<SubscriptionHistoryEvent, CoreError>> + Send;

    /// Get history for a specific subscription
    fn get_subscription_history(
        &self,
        realm_id: &str,
        subscription_id: &Uuid,
    ) -> impl Future<Output = Result<Vec<SubscriptionHistoryEvent>, CoreError>> + Send;

    /// List subscription history with filtering and pagination
    /// Returns (events, total_count)
    fn list_subscription_history(
        &self,
        realm_id: &str,
        query: SubscriptionHistoryQuery,
    ) -> impl Future<Output = Result<(Vec<SubscriptionHistoryEvent>, u64), CoreError>> + Send;

    // ===== Entitlement Mapping CRUD =====

    /// Create an entitlement mapping
    fn create_entitlement_mapping(
        &self,
        mapping: EntitlementMapping,
    ) -> impl Future<Output = Result<EntitlementMapping, CoreError>> + Send;

    /// Find entitlement mapping by ID
    fn find_entitlement_mapping_by_id(
        &self,
        mapping_id: Uuid,
    ) -> impl Future<Output = Result<Option<EntitlementMapping>, CoreError>> + Send;

    /// List entitlement mappings for a realm with optional filters
    fn list_entitlement_mappings(
        &self,
        realm_id: &str,
        payment_provider: Option<&str>,
        enabled: Option<bool>,
        page: Option<u64>,
        page_size: Option<u64>,
    ) -> impl Future<Output = Result<(Vec<EntitlementMapping>, u64), CoreError>> + Send;

    /// Update an entitlement mapping
    fn update_entitlement_mapping(
        &self,
        mapping: EntitlementMapping,
    ) -> impl Future<Output = Result<EntitlementMapping, CoreError>> + Send;

    /// Upsert an entitlement mapping by (realm_id, payment_provider, external_product_id)
    fn upsert_entitlement_mapping(
        &self,
        mapping: EntitlementMapping,
    ) -> impl Future<Output = Result<EntitlementMapping, CoreError>> + Send;

    /// Find entitlement mapping by provider and external product ID
    fn find_entitlement_mapping_by_provider_product(
        &self,
        realm_id: &str,
        payment_provider: &str,
        external_product_id: &str,
    ) -> impl Future<Output = Result<Option<EntitlementMapping>, CoreError>> + Send;

    /// Find entitlement mapping by entitlement key
    fn find_entitlement_mapping_by_key(
        &self,
        realm_id: &str,
        entitlement_key: &str,
    ) -> impl Future<Output = Result<Option<EntitlementMapping>, CoreError>> + Send;
}
