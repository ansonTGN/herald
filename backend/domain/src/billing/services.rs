use std::sync::Arc;
use uuid::Uuid;

use crate::authentication::Identity;
use crate::billing::entities::{
    ClientAppSubscriptionPlan, Product, SubscriptionPlan, SubscriptionPlanPaymentProvider,
    SubscriptionPlanType,
};
use crate::billing::policies::BillingPolicy;
use crate::billing::ports::BillingRepository;
use crate::common::entities::app_errors::CoreError;
use crate::common::policies::ensure_policy;

/// Input types for SubscriptionPlan operations
#[derive(Debug, Clone)]
pub struct CreateSubscriptionPlanInput {
    pub realm_id: String,
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub r#type: SubscriptionPlanType,
    pub price: i32,
    pub currency: String,
    pub checkout_url: Option<String>,
    pub trial_days: Option<i32>,
    pub sort_order: Option<i32>,
    pub product_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct UpdateSubscriptionPlanInput {
    pub name: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub r#type: Option<SubscriptionPlanType>,
    pub price: Option<i32>,
    pub currency: Option<String>,
    pub checkout_url: Option<String>,
    pub active: Option<bool>,
    pub trial_days: Option<i32>,
    pub sort_order: Option<i32>,
    pub product_id: Option<Uuid>,
}

/// SubscriptionPlan Service - Business logic for subscription plan management
///
/// Includes permission-based authorization checks using BillingPolicy
pub struct SubscriptionPlanService<R, P>
where
    R: BillingRepository,
    P: BillingPolicy,
{
    repository: Arc<R>,
    policy: Arc<P>,
}

impl<R, P> SubscriptionPlanService<R, P>
where
    R: BillingRepository + Send + Sync,
    P: BillingPolicy,
{
    pub fn new(repository: Arc<R>, policy: Arc<P>) -> Self {
        Self { repository, policy }
    }

    // ===== Plan CRUD =====

    pub async fn list_plans(
        &self,
        identity: Identity,
        realm_id: &str,
    ) -> Result<Vec<SubscriptionPlan>, CoreError> {
        // Check view permissions
        ensure_policy(
            self.policy
                .can_view_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to view billing plans",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing plans from a different realm".to_string(),
            ));
        }

        self.repository
            .list_subscription_plans_by_realm(realm_id)
            .await
    }

    pub async fn create_plan(
        &self,
        identity: Identity,
        realm_id: &str,
        input: CreateSubscriptionPlanInput,
    ) -> Result<SubscriptionPlan, CoreError> {
        // Check manage permissions
        ensure_policy(
            self.policy
                .can_manage_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to manage billing plans",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing plans from a different realm".to_string(),
            ));
        }

        self.ensure_product_belongs_to_realm(realm_id, input.product_id)
            .await?;

        let plan = SubscriptionPlan {
            id: Uuid::now_v7(),
            realm_id: realm_id.to_string(),
            name: input.name.clone(),
            title: input.title,
            description: input.description,
            r#type: input.r#type,
            price: input.price,
            currency: input.currency,
            checkout_url: input.checkout_url,
            active: true,
            trial_days: input.trial_days.unwrap_or(0),
            sort_order: input.sort_order.unwrap_or(0),
            product_id: input.product_id,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.repository.create_subscription_plan(plan).await
    }

    pub async fn get_plan(
        &self,
        identity: Identity,
        realm_id: &str,
        plan_id: Uuid,
    ) -> Result<SubscriptionPlan, CoreError> {
        // Check view permissions
        ensure_policy(
            self.policy
                .can_view_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to view billing plans",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing plans from a different realm".to_string(),
            ));
        }

        let plan = self
            .repository
            .find_subscription_plan_by_id(plan_id)
            .await?
            .ok_or_else(|| CoreError::SubscriptionPlanNotFound {
                realm_id: realm_id.to_string(),
                plan_id: plan_id.to_string(),
            })?;

        // Verify subscription plan belongs to realm
        if plan.realm_id != realm_id {
            return Err(CoreError::SubscriptionPlanNotFound {
                realm_id: realm_id.to_string(),
                plan_id: plan_id.to_string(),
            });
        }

        Ok(plan)
    }

    pub async fn update_plan(
        &self,
        identity: Identity,
        realm_id: &str,
        plan_id: Uuid,
        input: UpdateSubscriptionPlanInput,
    ) -> Result<SubscriptionPlan, CoreError> {
        // Check manage permissions
        ensure_policy(
            self.policy
                .can_manage_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to manage billing plans",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing plans from a different realm".to_string(),
            ));
        }

        let existing_plan = self.get_plan(identity, realm_id, plan_id).await?;
        let product_id = input.product_id.unwrap_or(existing_plan.product_id);

        self.ensure_product_belongs_to_realm(realm_id, product_id)
            .await?;

        let updated_plan = SubscriptionPlan {
            id: existing_plan.id,
            realm_id: existing_plan.realm_id.clone(),
            name: input.name.unwrap_or(existing_plan.name),
            title: input.title.unwrap_or(existing_plan.title),
            description: input.description.or(existing_plan.description),
            r#type: input.r#type.unwrap_or(existing_plan.r#type),
            price: input.price.unwrap_or(existing_plan.price),
            currency: input.currency.unwrap_or(existing_plan.currency),
            checkout_url: input.checkout_url.or(existing_plan.checkout_url),
            active: input.active.unwrap_or(existing_plan.active),
            trial_days: input.trial_days.unwrap_or(existing_plan.trial_days),
            sort_order: input.sort_order.unwrap_or(existing_plan.sort_order),
            product_id,
            created_at: existing_plan.created_at,
            updated_at: chrono::Utc::now(),
        };

        self.repository.update_subscription_plan(updated_plan).await
    }

    pub async fn delete_plan(
        &self,
        identity: Identity,
        realm_id: &str,
        plan_id: Uuid,
    ) -> Result<(), CoreError> {
        // Check manage permissions
        ensure_policy(
            self.policy
                .can_manage_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to manage billing plans",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing plans from a different realm".to_string(),
            ));
        }

        // Verify plan exists and belongs to realm
        let _plan = self.get_plan(identity, realm_id, plan_id).await?;

        let count = self
            .repository
            .count_active_subscriptions_for_subscription_plan(plan_id)
            .await?;

        if count > 0 {
            return Err(CoreError::SubscriptionPlanHasActiveSubscriptions {
                plan_id: plan_id.to_string(),
            });
        }

        self.repository.delete_subscription_plan(plan_id).await
    }

    // ===== Plan Assignment =====

    pub async fn assign_plan_to_client_app(
        &self,
        identity: Identity,
        realm_id: &str,
        client_app_id: Uuid,
        plan_id: Uuid,
    ) -> Result<ClientAppSubscriptionPlan, CoreError> {
        // Check manage permissions
        ensure_policy(
            self.policy
                .can_manage_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to manage billing plans",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing plans from a different realm".to_string(),
            ));
        }

        // Verify plan exists and belongs to realm
        let _plan = self.get_plan(identity, realm_id, plan_id).await?;

        // Call repository.assign_subscription_plan_to_client_app
        self.repository
            .assign_subscription_plan_to_client_app(client_app_id, plan_id)
            .await
    }

    pub async fn remove_plan_from_client_app(
        &self,
        identity: Identity,
        realm_id: &str,
        assignment_id: Uuid,
    ) -> Result<(), CoreError> {
        // Check manage permissions
        ensure_policy(
            self.policy
                .can_manage_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to manage billing plans",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing plans from a different realm".to_string(),
            ));
        }

        self.repository
            .remove_subscription_plan_from_client_app(assignment_id)
            .await
    }

    pub async fn list_plans_for_client_app(
        &self,
        identity: Identity,
        realm_id: &str,
        client_app_id: Uuid,
    ) -> Result<Vec<ClientAppSubscriptionPlan>, CoreError> {
        // Check view permissions
        ensure_policy(
            self.policy
                .can_view_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to view billing plans",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing plans from a different realm".to_string(),
            ));
        }

        self.repository
            .list_subscription_plans_for_client_app(client_app_id)
            .await
    }

    pub async fn toggle_plan_assignment(
        &self,
        identity: Identity,
        realm_id: &str,
        assignment_id: Uuid,
        enabled: bool,
    ) -> Result<ClientAppSubscriptionPlan, CoreError> {
        // Check manage permissions
        ensure_policy(
            self.policy
                .can_manage_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to manage billing plans",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing plans from a different realm".to_string(),
            ));
        }

        self.repository
            .toggle_subscription_plan_assignment(assignment_id, enabled)
            .await
    }

    async fn ensure_product_belongs_to_realm(
        &self,
        realm_id: &str,
        product_id: Uuid,
    ) -> Result<(), CoreError> {
        let product = self
            .repository
            .find_product_by_id(realm_id, product_id)
            .await?;
        if product.is_none() {
            return Err(CoreError::ProductNotFound {
                realm_id: realm_id.to_string(),
                product_id: product_id.to_string(),
            });
        }

        Ok(())
    }

    // ===== Plan Payment Provider Mapping =====

    /// List all payment provider mappings for a plan
    pub async fn list_plan_payment_providers(
        &self,
        identity: Identity,
        realm_id: &str,
        plan_id: Uuid,
    ) -> Result<Vec<SubscriptionPlanPaymentProvider>, CoreError> {
        // Check view permissions
        ensure_policy(
            self.policy
                .can_view_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to view billing plans",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing plans from a different realm".to_string(),
            ));
        }

        // Verify subscription plan exists and belongs to realm
        let _plan = self.get_plan(identity, realm_id, plan_id).await?;

        self.repository
            .list_subscription_plan_payment_providers(plan_id)
            .await
    }

    /// Add a payment provider mapping to a subscription plan
    pub async fn add_payment_provider_to_plan(
        &self,
        identity: Identity,
        realm_id: &str,
        plan_id: Uuid,
        payment_provider: String,
        external_product_id: String,
        external_price_id: Option<String>,
        enabled: bool,
    ) -> Result<SubscriptionPlanPaymentProvider, CoreError> {
        // Check manage permissions
        ensure_policy(
            self.policy
                .can_manage_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to manage billing plans",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing plans from a different realm".to_string(),
            ));
        }

        // Verify subscription plan exists and belongs to realm
        let _plan = self.get_plan(identity, realm_id, plan_id).await?;

        // Check if mapping already exists
        let existing = self
            .repository
            .find_subscription_plan_payment_provider_by_plan_and_provider(
                plan_id,
                &payment_provider,
            )
            .await?;

        if existing.is_some() {
            return Err(CoreError::BadRequest(format!(
                "Payment provider '{}' is already configured for this plan",
                payment_provider
            )));
        }

        let mapping = SubscriptionPlanPaymentProvider {
            id: Uuid::now_v7(),
            plan_id,
            payment_provider: payment_provider.clone(),
            external_product_id,
            external_price_id,
            enabled,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.repository
            .create_subscription_plan_payment_provider(mapping)
            .await
    }

    /// Update a payment provider mapping
    pub async fn update_plan_payment_provider(
        &self,
        identity: Identity,
        realm_id: &str,
        mapping_id: Uuid,
        external_product_id: Option<String>,
        external_price_id: Option<String>,
        enabled: Option<bool>,
    ) -> Result<SubscriptionPlanPaymentProvider, CoreError> {
        // Check manage permissions
        ensure_policy(
            self.policy
                .can_manage_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to manage billing plans",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing plans from a different realm".to_string(),
            ));
        }

        // Find existing mapping
        let existing = self
            .repository
            .find_subscription_plan_payment_provider_by_id(mapping_id)
            .await?
            .ok_or_else(|| {
                CoreError::BadRequest(format!(
                    "Payment provider mapping not found: {}",
                    mapping_id
                ))
            })?;

        let updated = SubscriptionPlanPaymentProvider {
            id: existing.id,
            plan_id: existing.plan_id,
            payment_provider: existing.payment_provider,
            external_product_id: external_product_id.unwrap_or(existing.external_product_id),
            external_price_id: external_price_id.or(existing.external_price_id),
            enabled: enabled.unwrap_or(existing.enabled),
            created_at: existing.created_at,
            updated_at: chrono::Utc::now(),
        };

        self.repository
            .update_subscription_plan_payment_provider(updated)
            .await
    }

    /// Toggle payment provider enabled status
    pub async fn toggle_plan_payment_provider(
        &self,
        identity: Identity,
        realm_id: &str,
        mapping_id: Uuid,
        enabled: bool,
    ) -> Result<SubscriptionPlanPaymentProvider, CoreError> {
        // Check manage permissions
        ensure_policy(
            self.policy
                .can_manage_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to manage billing plans",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing plans from a different realm".to_string(),
            ));
        }

        // Find existing mapping
        let existing = self
            .repository
            .find_subscription_plan_payment_provider_by_id(mapping_id)
            .await?
            .ok_or_else(|| {
                CoreError::BadRequest(format!(
                    "Payment provider mapping not found: {}",
                    mapping_id
                ))
            })?;

        let updated = SubscriptionPlanPaymentProvider {
            enabled,
            updated_at: chrono::Utc::now(),
            ..existing
        };

        self.repository
            .update_subscription_plan_payment_provider(updated)
            .await
    }

    /// Remove a payment provider mapping from a plan
    pub async fn remove_payment_provider_from_plan(
        &self,
        identity: Identity,
        realm_id: &str,
        mapping_id: Uuid,
    ) -> Result<(), CoreError> {
        // Check manage permissions
        ensure_policy(
            self.policy
                .can_manage_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to manage billing plans",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing plans from a different realm".to_string(),
            ));
        }

        // Verify mapping exists
        let existing = self
            .repository
            .find_subscription_plan_payment_provider_by_id(mapping_id)
            .await?
            .ok_or_else(|| {
                CoreError::BadRequest(format!(
                    "Payment provider mapping not found: {}",
                    mapping_id
                ))
            })?;

        let active_count = self
            .repository
            .count_active_subscriptions_for_subscription_plan(existing.plan_id)
            .await?;
        if active_count > 0 {
            return Err(CoreError::BadRequest(
                "Cannot remove payment provider mapping while the plan has active subscriptions"
                    .to_string(),
            ));
        }

        self.repository
            .delete_subscription_plan_payment_provider(mapping_id)
            .await
    }
}

/// Input types for Product operations
#[derive(Debug, Clone)]
pub struct CreateProductInput {
    pub realm_id: String,
    pub code: String,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateProductInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
}

/// Product Service - Business logic for product management
///
/// Includes permission-based authorization checks using BillingPolicy
pub struct ProductService<R, P>
where
    R: BillingRepository,
    P: BillingPolicy,
{
    repository: Arc<R>,
    policy: Arc<P>,
}

impl<R, P> ProductService<R, P>
where
    R: BillingRepository + Send + Sync,
    P: BillingPolicy,
{
    pub fn new(repository: Arc<R>, policy: Arc<P>) -> Self {
        Self { repository, policy }
    }

    pub async fn list_products(
        &self,
        identity: Identity,
        realm_id: &str,
        enabled_only: Option<bool>,
    ) -> Result<Vec<Product>, CoreError> {
        ensure_policy(
            self.policy
                .can_view_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to view billing products",
        )?;

        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing products from a different realm".to_string(),
            ));
        }

        self.repository.list_products(realm_id, enabled_only).await
    }

    pub async fn create_product(
        &self,
        identity: Identity,
        realm_id: &str,
        input: CreateProductInput,
    ) -> Result<Product, CoreError> {
        ensure_policy(
            self.policy
                .can_manage_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to manage billing products",
        )?;

        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing products from a different realm".to_string(),
            ));
        }

        if self
            .repository
            .product_code_exists(realm_id, &input.code)
            .await?
        {
            return Err(CoreError::ProductCodeExists {
                realm_id: realm_id.to_string(),
                code: input.code.clone(),
            });
        }

        let product = Product {
            id: Uuid::now_v7(),
            realm_id: realm_id.to_string(),
            code: input.code,
            title: input.title,
            description: input.description,
            enabled: true,
            plans_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.repository.create_product(product).await
    }

    pub async fn get_product(
        &self,
        identity: Identity,
        realm_id: &str,
        product_id: Uuid,
    ) -> Result<Product, CoreError> {
        ensure_policy(
            self.policy
                .can_view_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to view billing products",
        )?;

        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing products from a different realm".to_string(),
            ));
        }

        let product = self
            .repository
            .find_product_by_id(realm_id, product_id)
            .await?
            .ok_or_else(|| CoreError::ProductNotFound {
                realm_id: realm_id.to_string(),
                product_id: product_id.to_string(),
            })?;

        Ok(product)
    }

    pub async fn update_product(
        &self,
        identity: Identity,
        realm_id: &str,
        product_id: Uuid,
        input: UpdateProductInput,
    ) -> Result<Product, CoreError> {
        ensure_policy(
            self.policy
                .can_manage_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to manage billing products",
        )?;

        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing products from a different realm".to_string(),
            ));
        }

        self.repository
            .find_product_by_id(realm_id, product_id)
            .await?
            .ok_or_else(|| CoreError::ProductNotFound {
                realm_id: realm_id.to_string(),
                product_id: product_id.to_string(),
            })?;

        self.repository
            .update_product(
                realm_id,
                product_id,
                input.title,
                input.description,
                input.enabled,
            )
            .await
    }

    pub async fn delete_product(
        &self,
        identity: Identity,
        realm_id: &str,
        product_id: Uuid,
    ) -> Result<(), CoreError> {
        ensure_policy(
            self.policy
                .can_manage_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to manage billing products",
        )?;

        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing products from a different realm".to_string(),
            ));
        }

        self.repository
            .find_product_by_id(realm_id, product_id)
            .await?
            .ok_or_else(|| CoreError::ProductNotFound {
                realm_id: realm_id.to_string(),
                product_id: product_id.to_string(),
            })?;

        let plan_count = self
            .repository
            .count_subscription_plans_by_product(product_id)
            .await?;
        if plan_count > 0 {
            return Err(CoreError::ProductHasSubscriptionPlans {
                product_id: product_id.to_string(),
            });
        }

        self.repository.delete_product(realm_id, product_id).await
    }

    pub async fn list_plans_for_product(
        &self,
        identity: Identity,
        realm_id: &str,
        product_id: Uuid,
    ) -> Result<Vec<SubscriptionPlan>, CoreError> {
        ensure_policy(
            self.policy
                .can_view_subscription_plans(identity.clone())
                .await,
            "Insufficient permissions to view billing products",
        )?;

        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing products from a different realm".to_string(),
            ));
        }

        self.repository
            .find_product_by_id(realm_id, product_id)
            .await?
            .ok_or_else(|| CoreError::ProductNotFound {
                realm_id: realm_id.to_string(),
                product_id: product_id.to_string(),
            })?;

        self.repository
            .find_subscription_plans_by_product(realm_id, product_id)
            .await
    }
}
