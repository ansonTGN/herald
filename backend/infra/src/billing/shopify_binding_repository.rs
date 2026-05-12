//! SeaORM implementation of Shopify binding repository
//!
//! This provides the concrete implementation of the ShopifyBindingRepository trait
//! using SeaORM for database operations.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
};
use uuid::Uuid;

use herald_domain::billing::shopify_binding::{
    ShopifyBindingRepository, ShopifySubscriptionBinding,
};
use herald_domain::common::entities::app_errors::CoreError;
use herald_entity::shopify_subscription_binding;

/// PostgreSQL implementation of Shopify binding repository using SeaORM
pub struct ShopifyBindingRepositoryImpl {
    db: DatabaseConnection,
}

impl ShopifyBindingRepositoryImpl {
    /// Create a new repository instance
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Convert database model to domain entity
    fn model_to_entity(model: &shopify_subscription_binding::Model) -> ShopifySubscriptionBinding {
        ShopifySubscriptionBinding {
            id: model.id,
            subscription_id: model.subscription_id,
            realm_id: model.realm_id.clone(),
            shop_domain: model.shop_domain.clone(),
            contract_id: model.contract_id.clone(),
            contract_gid: model.contract_gid.clone(),
            contract_revision_id: model.contract_revision_id,
            customer_id: model.customer_id.clone(),
            customer_payment_method_id: model.customer_payment_method_id.clone(),
            last_billing_attempt_id: model.last_billing_attempt_id.clone(),
            last_order_id: model.last_order_id.clone(),
            cancel_reason: model.cancel_reason.clone(),
            created_at: model.created_at.into(),
            updated_at: model.updated_at.into(),
        }
    }
}

impl ShopifyBindingRepository for ShopifyBindingRepositoryImpl {
    async fn create_binding(
        &self,
        binding: &ShopifySubscriptionBinding,
    ) -> Result<ShopifySubscriptionBinding, CoreError> {
        let active_model = shopify_subscription_binding::ActiveModel {
            id: sea_orm::Set(binding.id),
            subscription_id: sea_orm::Set(binding.subscription_id),
            realm_id: sea_orm::Set(binding.realm_id.clone()),
            shop_domain: sea_orm::Set(binding.shop_domain.clone()),
            contract_id: sea_orm::Set(binding.contract_id.clone()),
            contract_gid: sea_orm::Set(binding.contract_gid.clone()),
            contract_revision_id: sea_orm::Set(binding.contract_revision_id),
            customer_id: sea_orm::Set(binding.customer_id.clone()),
            customer_payment_method_id: sea_orm::Set(binding.customer_payment_method_id.clone()),
            last_billing_attempt_id: sea_orm::Set(binding.last_billing_attempt_id.clone()),
            last_order_id: sea_orm::Set(binding.last_order_id.clone()),
            cancel_reason: sea_orm::Set(binding.cancel_reason.clone()),
            created_at: sea_orm::Set(binding.created_at.into()),
            updated_at: sea_orm::Set(binding.updated_at.into()),
        };

        let result = active_model.insert(&self.db).await.map_err(|e| {
            CoreError::InternalServerError(format!("Failed to create binding: {}", e))
        })?;

        Ok(Self::model_to_entity(&result))
    }

    async fn find_by_contract_id(
        &self,
        contract_id: &str,
    ) -> Result<Option<ShopifySubscriptionBinding>, CoreError> {
        let result = shopify_subscription_binding::Entity::find()
            .filter(shopify_subscription_binding::Column::ContractId.eq(contract_id))
            .one(&self.db)
            .await
            .map_err(|e| {
                CoreError::InternalServerError(format!("Failed to find binding: {}", e))
            })?;

        Ok(result.map(|model| Self::model_to_entity(&model)))
    }

    async fn find_by_subscription_id(
        &self,
        subscription_id: Uuid,
    ) -> Result<Option<ShopifySubscriptionBinding>, CoreError> {
        let result = shopify_subscription_binding::Entity::find()
            .filter(shopify_subscription_binding::Column::SubscriptionId.eq(subscription_id))
            .one(&self.db)
            .await
            .map_err(|e| {
                CoreError::InternalServerError(format!("Failed to find binding: {}", e))
            })?;

        Ok(result.map(|model| Self::model_to_entity(&model)))
    }

    async fn find_by_order_id(
        &self,
        order_id: &str,
    ) -> Result<Option<ShopifySubscriptionBinding>, CoreError> {
        let result = shopify_subscription_binding::Entity::find()
            .filter(shopify_subscription_binding::Column::LastOrderId.eq(order_id))
            .one(&self.db)
            .await
            .map_err(|e| {
                CoreError::InternalServerError(format!("Failed to find binding: {}", e))
            })?;

        Ok(result.map(|model| Self::model_to_entity(&model)))
    }

    async fn update_binding(
        &self,
        binding: &ShopifySubscriptionBinding,
    ) -> Result<ShopifySubscriptionBinding, CoreError> {
        let existing = shopify_subscription_binding::Entity::find()
            .filter(shopify_subscription_binding::Column::Id.eq(binding.id))
            .one(&self.db)
            .await
            .map_err(|e| CoreError::InternalServerError(format!("Failed to find binding: {}", e)))?
            .ok_or(CoreError::NotFound)?;

        let mut active_model: shopify_subscription_binding::ActiveModel = existing.into();
        active_model.subscription_id = sea_orm::Set(binding.subscription_id);
        active_model.realm_id = sea_orm::Set(binding.realm_id.clone());
        active_model.shop_domain = sea_orm::Set(binding.shop_domain.clone());
        active_model.contract_id = sea_orm::Set(binding.contract_id.clone());
        active_model.contract_gid = sea_orm::Set(binding.contract_gid.clone());
        active_model.contract_revision_id = sea_orm::Set(binding.contract_revision_id);
        active_model.customer_id = sea_orm::Set(binding.customer_id.clone());
        active_model.customer_payment_method_id =
            sea_orm::Set(binding.customer_payment_method_id.clone());
        active_model.last_billing_attempt_id =
            sea_orm::Set(binding.last_billing_attempt_id.clone());
        active_model.last_order_id = sea_orm::Set(binding.last_order_id.clone());
        active_model.cancel_reason = sea_orm::Set(binding.cancel_reason.clone());
        active_model.updated_at = sea_orm::Set(binding.updated_at.into());

        let result = active_model.update(&self.db).await.map_err(|e| {
            CoreError::InternalServerError(format!("Failed to update binding: {}", e))
        })?;

        Ok(Self::model_to_entity(&result))
    }

    async fn delete_binding(&self, id: i64) -> Result<(), CoreError> {
        let binding = shopify_subscription_binding::Entity::find()
            .filter(shopify_subscription_binding::Column::Id.eq(id))
            .one(&self.db)
            .await
            .map_err(|e| CoreError::InternalServerError(format!("Failed to find binding: {}", e)))?
            .ok_or(CoreError::NotFound)?;

        let active_model: shopify_subscription_binding::ActiveModel = binding.into();
        active_model.delete(&self.db).await.map_err(|e| {
            CoreError::InternalServerError(format!("Failed to delete binding: {}", e))
        })?;

        Ok(())
    }

    async fn count_active_by_realm(&self, realm_id: &str) -> Result<i64, CoreError> {
        // Note: This is a simplified count. For accurate "active" subscriptions,
        // we would need to JOIN with the subscription table and check status = 'active'
        // For now, we'll count all bindings for the realm
        let count = shopify_subscription_binding::Entity::find()
            .filter(shopify_subscription_binding::Column::RealmId.eq(realm_id))
            .count(&self.db)
            .await
            .map_err(|e| {
                CoreError::InternalServerError(format!("Failed to count bindings: {}", e))
            })?;

        Ok(count as i64)
    }
}
