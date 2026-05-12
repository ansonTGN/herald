use std::future::Future;

use uuid::Uuid;

use crate::billing::{
    ClientAppPlan, PaymentEvent, Plan, PlanPaymentProvider, Product, Subscription,
    SubscriptionHistoryEvent, SubscriptionHistoryQuery,
};
use crate::common::entities::app_errors::CoreError;

/// Policy trait for billing authorization checks
///
/// NOTE: This trait is currently not implemented and commented out.
/// It should be enabled in the future when billing authorization is needed.
/*
#[async_trait::async_trait]
#[allow(clippy::too_many_arguments)]
pub trait BillingPolicy: Send + Sync {
    // Plan permissions
    async fn can_list_plans(&self, identity: &Identity, realm_id: &str) -> bool;
    async fn can_create_plan(&self, identity: &Identity, realm_id: &str) -> bool;
    async fn can_view_plan(&self, identity: &Identity, realm_id: &str) -> bool;
    async fn can_update_plan(&self, identity: &Identity, realm_id: &str) -> bool;
    async fn can_delete_plan(&self, identity: &Identity, realm_id: &str) -> bool;

    // Plan assignment permissions
    async fn can_assign_plan(&self, identity: &Identity, realm_id: &str) -> bool;
    async fn can_unassign_plan(&self, identity: &Identity, realm_id: &str) -> bool;
    async fn can_view_plan_assignments(&self, identity: &Identity, realm_id: &str) -> bool;
    async fn can_toggle_plan_assignment(&self, identity: &Identity, realm_id: &str) -> bool;

    // Subscription permissions
    async fn can_view_subscription(&self, identity: &Identity, realm_id: &str) -> bool;
    async fn can_cancel_subscription(&self, identity: &Identity, realm_id: &str) -> bool;
}
*/
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

    // ===== Plan CRUD =====
    fn create_plan(&self, plan: Plan) -> impl Future<Output = Result<Plan, CoreError>> + Send;
    fn find_plan_by_id(
        &self,
        plan_id: Uuid,
    ) -> impl Future<Output = Result<Option<Plan>, CoreError>> + Send;
    fn find_public_plan_by_id(
        &self,
        realm_id: &str,
        plan_id: Uuid,
    ) -> impl Future<Output = Result<Option<Plan>, CoreError>> + Send;
    fn list_plans_by_realm(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = Result<Vec<Plan>, CoreError>> + Send;
    fn list_public_plans_by_realm(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = Result<Vec<Plan>, CoreError>> + Send;
    fn update_plan(&self, plan: Plan) -> impl Future<Output = Result<Plan, CoreError>> + Send;
    fn delete_plan(&self, plan_id: Uuid) -> impl Future<Output = Result<(), CoreError>> + Send;

    // ===== Plan Assignment =====
    fn assign_plan_to_client_app(
        &self,
        client_app_id: Uuid,
        plan_id: Uuid,
    ) -> impl Future<Output = Result<ClientAppPlan, CoreError>> + Send;
    fn remove_plan_from_client_app(
        &self,
        assignment_id: Uuid,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
    fn list_plans_for_client_app(
        &self,
        client_app_id: Uuid,
    ) -> impl Future<Output = Result<Vec<ClientAppPlan>, CoreError>> + Send;
    fn find_plan_assignment(
        &self,
        client_app_id: Uuid,
        plan_id: Uuid,
    ) -> impl Future<Output = Result<Option<ClientAppPlan>, CoreError>> + Send;
    fn toggle_plan_assignment(
        &self,
        assignment_id: Uuid,
        enabled: bool,
    ) -> impl Future<Output = Result<ClientAppPlan, CoreError>> + Send;

    // ===== Updated Subscription =====
    fn find_subscription_by_client_app_id(
        &self,
        client_app_id: Uuid,
    ) -> impl Future<Output = Result<Option<Subscription>, CoreError>> + Send;
    fn cancel_subscription(
        &self,
        subscription_id: Uuid,
        cancel_at_period_end: bool,
    ) -> impl Future<Output = Result<Subscription, CoreError>> + Send;
    fn count_active_subscriptions_for_plan(
        &self,
        plan_id: Uuid,
    ) -> impl Future<Output = Result<i64, CoreError>> + Send;

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

    // Product CRUD
    fn create_product(
        &self,
        product: Product,
    ) -> impl Future<Output = Result<Product, CoreError>> + Send;
    fn find_product_by_id(
        &self,
        realm_id: &str,
        product_id: Uuid,
    ) -> impl Future<Output = Result<Option<Product>, CoreError>> + Send;
    fn product_name_exists(
        &self,
        realm_id: &str,
        name: &str,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;
    fn list_products(
        &self,
        realm_id: &str,
        enabled_only: Option<bool>,
    ) -> impl Future<Output = Result<Vec<Product>, CoreError>> + Send;
    fn update_product(
        &self,
        realm_id: &str,
        product_id: Uuid,
        title: Option<String>,
        description: Option<String>,
        sort_order: Option<i32>,
        enabled: Option<bool>,
    ) -> impl Future<Output = Result<Product, CoreError>> + Send;
    fn delete_product(
        &self,
        realm_id: &str,
        product_id: Uuid,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
    fn count_plans_by_product(
        &self,
        product_id: Uuid,
    ) -> impl Future<Output = Result<i64, CoreError>> + Send;
    fn find_plans_by_product(
        &self,
        realm_id: &str,
        product_id: Uuid,
    ) -> impl Future<Output = Result<Vec<Plan>, CoreError>> + Send;

    // ===== Plan Payment Provider Mapping =====
    /// Create a new plan payment provider mapping
    fn create_plan_payment_provider(
        &self,
        mapping: PlanPaymentProvider,
    ) -> impl Future<Output = Result<PlanPaymentProvider, CoreError>> + Send;

    /// Find plan payment provider mapping by ID
    fn find_plan_payment_provider_by_id(
        &self,
        mapping_id: Uuid,
    ) -> impl Future<Output = Result<Option<PlanPaymentProvider>, CoreError>> + Send;

    /// Find plan payment provider mapping by plan and provider
    fn find_plan_payment_provider_by_plan_and_provider(
        &self,
        plan_id: Uuid,
        payment_provider: &str,
    ) -> impl Future<Output = Result<Option<PlanPaymentProvider>, CoreError>> + Send;

    /// List all payment provider mappings for a plan
    fn list_plan_payment_providers(
        &self,
        plan_id: Uuid,
    ) -> impl Future<Output = Result<Vec<PlanPaymentProvider>, CoreError>> + Send;

    /// Update plan payment provider mapping
    fn update_plan_payment_provider(
        &self,
        mapping: PlanPaymentProvider,
    ) -> impl Future<Output = Result<PlanPaymentProvider, CoreError>> + Send;

    /// Delete plan payment provider mapping
    fn delete_plan_payment_provider(
        &self,
        mapping_id: Uuid,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Delete all payment provider mappings for a plan (cascade delete)
    fn delete_plan_payment_providers_by_plan(
        &self,
        plan_id: Uuid,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    /// Batch load payment provider mappings for multiple plans
    fn list_plan_payment_providers_batch(
        &self,
        plan_ids: &[Uuid],
    ) -> impl Future<Output = Result<Vec<PlanPaymentProvider>, CoreError>> + Send;
}
