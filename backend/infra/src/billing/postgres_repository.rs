use sea_orm::DatabaseBackend;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    FromQueryResult, IntoActiveModel, JoinType, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait, Set, Statement,
};
use std::str::FromStr;
use uuid::Uuid;

use herald_domain::billing::entities::{BillingPeriod, PlanType};
use herald_domain::billing::{
    BillingRepository, ClientAppPlan, HistoryEventType, PaymentEvent, Plan, PlanPaymentProvider,
    Product, SortOrder, Subscription, SubscriptionHistoryEvent, SubscriptionHistoryQuery,
};
use herald_domain::common::entities::app_errors::CoreError;
use herald_entity::{
    client_app_plan, payment_event, plan, product, subscription, subscription_history,
};

/// PostgreSQL implementation of billing repository
pub struct PostgresBillingRepository {
    db: DatabaseConnection,
}

impl PostgresBillingRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Converts database model to domain Subscription
    fn model_to_subscription(model: subscription::Model) -> Result<Subscription, CoreError> {
        Ok(Subscription {
            id: model.id,
            realm_id: model.realm_id,
            user_id: model.user_id,
            external_subscription_id: model.external_subscription_id,
            external_product_id: model.external_product_id,
            payment_provider: model.payment_provider,
            status: model.status.parse()?,
            tier: model.tier.parse()?,
            current_period_start: model.current_period_start.map(chrono::DateTime::from),
            current_period_end: model.current_period_end.map(chrono::DateTime::from),
            cancel_at_period_end: model.cancel_at_period_end,
            client_app_id: model.client_app_id,
            plan_id: model.plan_id,
            billing_period: BillingPeriod::from(model.billing_period),
            cancel_at: model.cancel_at.map(chrono::DateTime::from),
            created_at: chrono::DateTime::from(model.created_at),
            updated_at: chrono::DateTime::from(model.updated_at),
        })
    }

    /// Converts database model to domain PaymentEvent
    fn model_to_payment_event(mut model: payment_event::Model) -> PaymentEvent {
        PaymentEvent {
            id: model.id,
            realm_id: model.realm_id,
            external_event_id: model.external_event_id,
            payment_provider: model.payment_provider,
            event_type: model.event_type,
            subscription_id: model.subscription_id,
            payload: model.payload.take(),
            processed: model.processed,
            processing_started_at: model.processing_started_at.map(chrono::DateTime::from),
            created_at: chrono::DateTime::from(model.created_at),
        }
    }

    /// Converts database model to domain Plan
    fn model_to_plan(model: plan::Model) -> Result<Plan, CoreError> {
        Ok(Plan {
            id: model.id,
            realm_id: model.realm_id,
            name: model.name,
            title: model.title,
            description: model.description,
            r#type: parse_plan_type(&model.r#type),
            price: model.price,
            currency: model.currency,
            checkout_url: model.checkout_url,
            active: model.active,
            trial_days: model.trial_days,
            sort_order: model.sort_order,
            product_id: model.product_id,
            created_at: chrono::DateTime::from(model.created_at),
            updated_at: chrono::DateTime::from(model.updated_at),
        })
    }

    fn model_to_client_app_plan(model: client_app_plan::Model) -> ClientAppPlan {
        ClientAppPlan {
            id: model.id,
            client_app_id: model.client_app_id,
            plan_id: model.plan_id,
            enabled: model.enabled,
            created_at: chrono::DateTime::from(model.created_at),
        }
    }

    async fn find_plan_model(
        &self,
        realm_id: Option<&str>,
        plan_id: Uuid,
        public_only: bool,
    ) -> Result<Option<plan::Model>, CoreError> {
        let mut query = plan::Entity::find().filter(plan::Column::Id.eq(plan_id));
        if let Some(realm_id) = realm_id {
            query = query.filter(plan::Column::RealmId.eq(realm_id));
        }
        if public_only {
            query = query
                .join(JoinType::InnerJoin, plan::Relation::Product.def())
                .filter(plan::Column::Active.eq(true))
                .filter(product::Column::Enabled.eq(true));
        }

        query
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))
    }

    /// Converts domain Plan to database active model
    fn plan_to_active_model(plan: Plan) -> plan::ActiveModel {
        plan::ActiveModel {
            id: Set(plan.id),
            realm_id: Set(plan.realm_id),
            name: Set(plan.name),
            title: Set(plan.title),
            description: Set(plan.description),
            r#type: Set(plan.r#type.as_str().to_string()),
            price: Set(plan.price),
            currency: Set(plan.currency),
            checkout_url: Set(plan.checkout_url),
            active: Set(plan.active),
            trial_days: Set(plan.trial_days),
            sort_order: Set(plan.sort_order),
            product_id: Set(plan.product_id),
            created_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(
                plan.created_at,
            )),
            updated_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(
                plan.updated_at,
            )),
        }
    }

    /// Converts database model to domain Product
    fn model_to_product_with_count(model: product::Model, plans_count: i64) -> Product {
        Product {
            id: model.id,
            realm_id: model.realm_id,
            code: model.code,
            title: model.title,
            description: model.description,
            enabled: model.enabled,
            plans_count,
            created_at: chrono::DateTime::from(model.created_at),
            updated_at: chrono::DateTime::from(model.updated_at),
        }
    }

    async fn model_to_product(
        model: product::Model,
        db: &DatabaseConnection,
    ) -> Result<Product, CoreError> {
        let plans_count = plan::Entity::find()
            .filter(plan::Column::ProductId.eq(model.id))
            .count(db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(Self::model_to_product_with_count(model, plans_count as i64))
    }

    /// Converts database model to domain SubscriptionHistoryEvent
    fn model_to_subscription_history_event(
        model: subscription_history::Model,
    ) -> Result<SubscriptionHistoryEvent, CoreError> {
        let event_type_str = model.event_type.to_lowercase();
        let event_type = HistoryEventType::from_str(&event_type_str)?;

        Ok(SubscriptionHistoryEvent {
            id: model.id,
            subscription_id: model.subscription_id,
            event_type,
            timestamp: chrono::DateTime::from(model.timestamp),
            actor: model.actor,
            changes: model
                .changes
                .map(|json| serde_json::to_value(json).unwrap_or(serde_json::Value::Null)),
            previous_state: model
                .previous_state
                .map(|json| serde_json::to_value(json).unwrap_or(serde_json::Value::Null)),
            new_state: model
                .new_state
                .map(|json| serde_json::to_value(json).unwrap_or(serde_json::Value::Null)),
            realm_id: model.realm_id,
            created_at: chrono::DateTime::from(model.created_at),
        })
    }

    /// Converts domain SubscriptionHistoryEvent to database active model
    fn history_event_to_active_model(
        event: SubscriptionHistoryEvent,
    ) -> subscription_history::ActiveModel {
        subscription_history::ActiveModel {
            id: Set(event.id),
            subscription_id: Set(event.subscription_id),
            event_type: Set(event.event_type.as_str().to_string()),
            timestamp: Set(sea_orm::prelude::DateTimeWithTimeZone::from(
                event.timestamp,
            )),
            actor: Set(event.actor),
            changes: Set(event.changes),
            previous_state: Set(event.previous_state),
            new_state: Set(event.new_state),
            realm_id: Set(event.realm_id),
            created_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(
                event.created_at,
            )),
        }
    }
}

/// Helper struct for deserializing plan payment provider query results
#[derive(FromQueryResult)]
struct PlanPaymentProviderQueryResult {
    id: Uuid,
    plan_id: Uuid,
    payment_provider: String,
    external_product_id: String,
    external_price_id: Option<String>,
    enabled: bool,
    created_at: sea_orm::prelude::DateTimeWithTimeZone,
    updated_at: sea_orm::prelude::DateTimeWithTimeZone,
}

/// Converts query result to domain PlanPaymentProvider
fn query_result_to_plan_payment_provider(
    result: PlanPaymentProviderQueryResult,
) -> PlanPaymentProvider {
    PlanPaymentProvider {
        id: result.id,
        plan_id: result.plan_id,
        payment_provider: result.payment_provider,
        external_product_id: result.external_product_id,
        external_price_id: result.external_price_id,
        enabled: result.enabled,
        created_at: chrono::DateTime::from(result.created_at),
        updated_at: chrono::DateTime::from(result.updated_at),
    }
}

/// Parse plan type string, defaulting to Monthly
fn parse_plan_type(s: &str) -> PlanType {
    match s.to_lowercase().as_str() {
        "monthly" => PlanType::Monthly,
        "yearly" => PlanType::Yearly,
        _ => PlanType::Monthly,
    }
}

impl BillingRepository for PostgresBillingRepository {
    async fn create_subscription(&self, sub: Subscription) -> Result<Subscription, CoreError> {
        let model = subscription::ActiveModel {
            id: Set(sub.id),
            realm_id: Set(sub.realm_id.clone()),
            user_id: Set(sub.user_id),
            external_subscription_id: Set(sub.external_subscription_id),
            external_product_id: Set(sub.external_product_id.clone()),
            payment_provider: Set(sub.payment_provider.clone()),
            status: Set(sub.status.as_str().to_string()),
            tier: Set(sub.tier.as_str().to_string()),
            current_period_start: Set(sub
                .current_period_start
                .map(sea_orm::prelude::DateTimeWithTimeZone::from)),
            current_period_end: Set(sub
                .current_period_end
                .map(sea_orm::prelude::DateTimeWithTimeZone::from)),
            cancel_at_period_end: Set(sub.cancel_at_period_end),
            client_app_id: Set(sub.client_app_id),
            plan_id: Set(sub.plan_id),
            billing_period: Set(sub.billing_period.to_string()),
            cancel_at: Set(sub
                .cancel_at
                .map(sea_orm::prelude::DateTimeWithTimeZone::from)),
            created_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(sub.created_at)),
            updated_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(sub.updated_at)),
        };

        let result = model.insert(&self.db).await?;
        Self::model_to_subscription(result)
    }

    async fn find_by_realm_id(&self, realm_id: &str) -> Result<Option<Subscription>, CoreError> {
        let result = subscription::Entity::find()
            .filter(subscription::Column::RealmId.eq(realm_id))
            .one(&self.db)
            .await?;

        result.map(Self::model_to_subscription).transpose()
    }

    async fn find_by_external_subscription_id(
        &self,
        external_sub_id: &str,
        provider: &str,
    ) -> Result<Option<Subscription>, CoreError> {
        let result = subscription::Entity::find()
            .filter(subscription::Column::ExternalSubscriptionId.eq(external_sub_id))
            .filter(subscription::Column::PaymentProvider.eq(provider))
            .one(&self.db)
            .await?;

        result.map(Self::model_to_subscription).transpose()
    }

    async fn find_subscription_by_id(
        &self,
        subscription_id: Uuid,
    ) -> Result<Option<Subscription>, CoreError> {
        let result = subscription::Entity::find_by_id(subscription_id)
            .one(&self.db)
            .await?;

        result.map(Self::model_to_subscription).transpose()
    }

    async fn update_subscription(&self, sub: Subscription) -> Result<Subscription, CoreError> {
        let existing = subscription::Entity::find_by_id(sub.id)
            .one(&self.db)
            .await?
            .ok_or_else(|| CoreError::SubscriptionNotFound(sub.id.to_string()))?;

        let mut active_model: subscription::ActiveModel = existing.into_active_model();
        active_model.realm_id = Set(sub.realm_id.clone());
        active_model.user_id = Set(sub.user_id);
        active_model.external_subscription_id = Set(sub.external_subscription_id.clone());
        active_model.external_product_id = Set(sub.external_product_id.clone());
        active_model.payment_provider = Set(sub.payment_provider.clone());
        active_model.status = Set(sub.status.as_str().to_string());
        active_model.tier = Set(sub.tier.as_str().to_string());
        active_model.current_period_start = Set(sub
            .current_period_start
            .map(sea_orm::prelude::DateTimeWithTimeZone::from));
        active_model.current_period_end = Set(sub
            .current_period_end
            .map(sea_orm::prelude::DateTimeWithTimeZone::from));
        active_model.cancel_at_period_end = Set(sub.cancel_at_period_end);
        active_model.client_app_id = Set(sub.client_app_id);
        active_model.plan_id = Set(sub.plan_id);
        active_model.billing_period = Set(sub.billing_period.to_string());
        active_model.cancel_at = Set(sub
            .cancel_at
            .map(sea_orm::prelude::DateTimeWithTimeZone::from));
        active_model.updated_at = Set(sea_orm::prelude::DateTimeWithTimeZone::from(sub.updated_at));

        let result = active_model.update(&self.db).await?;
        Self::model_to_subscription(result)
    }

    async fn create_payment_event(&self, event: PaymentEvent) -> Result<PaymentEvent, CoreError> {
        let model = payment_event::ActiveModel {
            id: Set(event.id),
            realm_id: Set(event.realm_id.clone()),
            external_event_id: Set(event.external_event_id.clone()),
            payment_provider: Set(event.payment_provider.clone()),
            event_type: Set(event.event_type.clone()),
            subscription_id: Set(event.subscription_id),
            payload: Set(event.payload.clone()),
            processed: Set(event.processed),
            processing_started_at: Set(event
                .processing_started_at
                .map(sea_orm::prelude::DateTimeWithTimeZone::from)),
            created_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(
                event.created_at,
            )),
        };

        let result = model.insert(&self.db).await?;
        Ok(Self::model_to_payment_event(result))
    }

    async fn find_payment_event_by_external_id(
        &self,
        external_event_id: &str,
        payment_provider: &str,
    ) -> Result<Option<PaymentEvent>, CoreError> {
        Ok(payment_event::Entity::find()
            .filter(payment_event::Column::ExternalEventId.eq(external_event_id))
            .filter(payment_event::Column::PaymentProvider.eq(payment_provider))
            .one(&self.db)
            .await?
            .map(Self::model_to_payment_event))
    }

    async fn mark_payment_event_processed(&self, id: Uuid) -> Result<(), CoreError> {
        let existing = payment_event::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(CoreError::NotFound)?;

        let mut active_model: payment_event::ActiveModel = existing.into_active_model();
        active_model.processed = Set(true);

        active_model.update(&self.db).await?;
        Ok(())
    }

    // ===== Plan CRUD =====

    async fn create_plan(&self, plan: Plan) -> Result<Plan, CoreError> {
        let plan_name = plan.name.clone();
        let realm_id = plan.realm_id.clone();
        let active_model = Self::plan_to_active_model(plan);

        let result = plan::Entity::insert(active_model)
            .exec(&self.db)
            .await
            .map_err(|e| {
                // Check for unique constraint violation
                if e.to_string().contains("plan_realm_id_name_key") {
                    CoreError::BadRequest(format!(
                        "Plan with name '{}' already exists in realm '{}'",
                        plan_name, realm_id
                    ))
                } else {
                    CoreError::DatabaseError(e.to_string())
                }
            })?;

        self.find_plan_by_id(result.last_insert_id)
            .await?
            .ok_or(CoreError::NotFound)
    }

    async fn find_plan_by_id(&self, plan_id: Uuid) -> Result<Option<Plan>, CoreError> {
        let result = self.find_plan_model(None, plan_id, false).await?;

        result.map(Self::model_to_plan).transpose()
    }

    async fn find_public_plan_by_id(
        &self,
        realm_id: &str,
        plan_id: Uuid,
    ) -> Result<Option<Plan>, CoreError> {
        let result = self.find_plan_model(Some(realm_id), plan_id, true).await?;

        result.map(Self::model_to_plan).transpose()
    }

    async fn list_plans_by_realm(&self, realm_id: &str) -> Result<Vec<Plan>, CoreError> {
        let results = plan::Entity::find()
            .filter(plan::Column::RealmId.eq(realm_id))
            // Note: Removed Active.eq(true) filter to allow listing of disabled plans
            // This enables admins to see and re-enable disabled plans
            .order_by_asc(plan::Column::SortOrder)
            .all(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        results.into_iter().map(Self::model_to_plan).collect()
    }

    async fn list_public_plans_by_realm(&self, realm_id: &str) -> Result<Vec<Plan>, CoreError> {
        let results = plan::Entity::find()
            .join(JoinType::InnerJoin, plan::Relation::Product.def())
            .filter(plan::Column::RealmId.eq(realm_id))
            .filter(plan::Column::Active.eq(true))
            .filter(product::Column::Enabled.eq(true))
            .order_by_asc(plan::Column::SortOrder)
            .all(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        results.into_iter().map(Self::model_to_plan).collect()
    }

    async fn update_plan(&self, plan: Plan) -> Result<Plan, CoreError> {
        let existing = plan::Entity::find_by_id(plan.id)
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            .ok_or_else(|| CoreError::PlanNotFound {
                realm_id: plan.realm_id.clone(),
                plan_id: plan.id.to_string(),
            })?;

        let mut active_model: plan::ActiveModel = existing.into_active_model();
        // Copy only updatable fields from the helper
        let helper = Self::plan_to_active_model(plan);
        active_model.realm_id = helper.realm_id;
        active_model.name = helper.name;
        active_model.title = helper.title;
        active_model.description = helper.description;
        active_model.r#type = helper.r#type;
        active_model.price = helper.price;
        active_model.currency = helper.currency;
        active_model.checkout_url = helper.checkout_url;
        active_model.active = helper.active;
        active_model.trial_days = helper.trial_days;
        active_model.sort_order = helper.sort_order;
        active_model.product_id = helper.product_id;
        active_model.updated_at = helper.updated_at;

        let result = active_model
            .update(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Self::model_to_plan(result)
    }

    async fn delete_plan(&self, plan_id: Uuid) -> Result<(), CoreError> {
        plan::Entity::delete_by_id(plan_id)
            .exec(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    // ===== Plan Assignment =====

    async fn assign_plan_to_client_app(
        &self,
        client_app_id: Uuid,
        plan_id: Uuid,
    ) -> Result<ClientAppPlan, CoreError> {
        // Check if already assigned
        if (self.find_plan_assignment(client_app_id, plan_id).await?).is_some() {
            return Err(CoreError::Conflict(
                "Plan already assigned to client app".to_string(),
            ));
        }

        let active_model = client_app_plan::ActiveModel {
            id: Set(Uuid::now_v7()),
            client_app_id: Set(client_app_id),
            plan_id: Set(plan_id),
            enabled: Set(true),
            created_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(
                chrono::Utc::now(),
            )),
        };

        let result = client_app_plan::Entity::insert(active_model)
            .exec(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        // Find and return the created assignment
        let assignment = client_app_plan::Entity::find_by_id(result.last_insert_id)
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            .ok_or(CoreError::NotFound)?;

        Ok(Self::model_to_client_app_plan(assignment))
    }

    async fn remove_plan_from_client_app(&self, assignment_id: Uuid) -> Result<(), CoreError> {
        client_app_plan::Entity::delete_by_id(assignment_id)
            .exec(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn list_plans_for_client_app(
        &self,
        client_app_id: Uuid,
    ) -> Result<Vec<ClientAppPlan>, CoreError> {
        let results = client_app_plan::Entity::find()
            .filter(client_app_plan::Column::ClientAppId.eq(client_app_id))
            .filter(client_app_plan::Column::Enabled.eq(true))
            .all(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(Self::model_to_client_app_plan)
            .collect())
    }

    async fn find_plan_assignment(
        &self,
        client_app_id: Uuid,
        plan_id: Uuid,
    ) -> Result<Option<ClientAppPlan>, CoreError> {
        client_app_plan::Entity::find()
            .filter(client_app_plan::Column::ClientAppId.eq(client_app_id))
            .filter(client_app_plan::Column::PlanId.eq(plan_id))
            .one(&self.db)
            .await
            .map(|opt| opt.map(Self::model_to_client_app_plan))
            .map_err(|e| CoreError::DatabaseError(e.to_string()))
    }

    async fn toggle_plan_assignment(
        &self,
        assignment_id: Uuid,
        enabled: bool,
    ) -> Result<ClientAppPlan, CoreError> {
        let assignment = client_app_plan::Entity::find_by_id(assignment_id)
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            .ok_or(CoreError::NotFound)?;

        let mut active_model: client_app_plan::ActiveModel = assignment.into_active_model();
        active_model.enabled = Set(enabled);

        let updated = active_model
            .update(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(Self::model_to_client_app_plan(updated))
    }

    async fn create_product(&self, product: Product) -> Result<Product, CoreError> {
        let active_model = product::ActiveModel {
            id: Set(product.id),
            realm_id: Set(product.realm_id.clone()),
            code: Set(product.code.clone()),
            title: Set(product.title.clone()),
            description: Set(product.description.clone()),
            enabled: Set(product.enabled),
            created_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(
                product.created_at,
            )),
            updated_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(
                product.updated_at,
            )),
        };

        product::Entity::insert(active_model)
            .exec(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(product)
    }

    async fn find_product_by_id(
        &self,
        realm_id: &str,
        product_id: Uuid,
    ) -> Result<Option<Product>, CoreError> {
        let result = product::Entity::find_by_id(product_id)
            .filter(product::Column::RealmId.eq(realm_id))
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => Ok(Some(Self::model_to_product(model, &self.db).await?)),
            None => Ok(None),
        }
    }

    async fn product_code_exists(&self, realm_id: &str, code: &str) -> Result<bool, CoreError> {
        let count = product::Entity::find()
            .filter(product::Column::RealmId.eq(realm_id))
            .filter(product::Column::Code.eq(code))
            .count(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(count > 0)
    }

    async fn list_products(
        &self,
        realm_id: &str,
        enabled_only: Option<bool>,
    ) -> Result<Vec<Product>, CoreError> {
        let mut query = product::Entity::find().filter(product::Column::RealmId.eq(realm_id));

        if let Some(enabled) = enabled_only {
            query = query.filter(product::Column::Enabled.eq(enabled));
        }

        let results = query
            .order_by_asc(product::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        if results.is_empty() {
            return Ok(Vec::new());
        }

        let product_ids: Vec<Uuid> = results.iter().map(|m| m.id).collect();
        let plan_product_ids: Vec<Uuid> = plan::Entity::find()
            .filter(plan::Column::ProductId.is_in(product_ids))
            .all(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            .into_iter()
            .map(|p| p.product_id)
            .collect();

        let mut counts: std::collections::HashMap<Uuid, i64> = std::collections::HashMap::new();
        for pid in plan_product_ids {
            *counts.entry(pid).or_insert(0) += 1;
        }

        Ok(results
            .into_iter()
            .map(|model| {
                let count = counts.get(&model.id).copied().unwrap_or(0);
                Self::model_to_product_with_count(model, count)
            })
            .collect())
    }

    async fn update_product(
        &self,
        realm_id: &str,
        product_id: Uuid,
        title: Option<String>,
        description: Option<String>,
        enabled: Option<bool>,
    ) -> Result<Product, CoreError> {
        let existing = product::Entity::find_by_id(product_id)
            .filter(product::Column::RealmId.eq(realm_id))
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            .ok_or(CoreError::ProductNotFound {
                realm_id: realm_id.to_string(),
                product_id: product_id.to_string(),
            })?;

        let mut active_model: product::ActiveModel = existing.into_active_model();
        if let Some(title) = title {
            active_model.title = Set(title);
        }
        if let Some(description) = description {
            active_model.description = Set(Some(description));
        }
        if let Some(enabled) = enabled {
            active_model.enabled = Set(enabled);
        }
        active_model.updated_at = Set(sea_orm::prelude::DateTimeWithTimeZone::from(
            chrono::Utc::now(),
        ));

        let updated = active_model
            .update(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Self::model_to_product(updated, &self.db).await
    }

    async fn delete_product(&self, realm_id: &str, product_id: Uuid) -> Result<(), CoreError> {
        let existing = product::Entity::find_by_id(product_id)
            .filter(product::Column::RealmId.eq(realm_id))
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            .ok_or(CoreError::ProductNotFound {
                realm_id: realm_id.to_string(),
                product_id: product_id.to_string(),
            })?;

        let active_model: product::ActiveModel = existing.into_active_model();
        active_model
            .delete(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn count_plans_by_product(&self, product_id: Uuid) -> Result<i64, CoreError> {
        let count = plan::Entity::find()
            .filter(plan::Column::ProductId.eq(product_id))
            .count(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(count as i64)
    }

    async fn find_plans_by_product(
        &self,
        realm_id: &str,
        product_id: Uuid,
    ) -> Result<Vec<Plan>, CoreError> {
        let results = plan::Entity::find()
            .filter(plan::Column::RealmId.eq(realm_id))
            .filter(plan::Column::ProductId.eq(product_id))
            .order_by_asc(plan::Column::SortOrder)
            .all(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        results.into_iter().map(Self::model_to_plan).collect()
    }

    // ===== Updated Subscription =====

    async fn find_subscription_by_client_app_id(
        &self,
        client_app_id: Uuid,
    ) -> Result<Option<Subscription>, CoreError> {
        let result = subscription::Entity::find()
            .filter(subscription::Column::ClientAppId.eq(client_app_id))
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        result.map(Self::model_to_subscription).transpose()
    }

    async fn cancel_subscription(
        &self,
        subscription_id: Uuid,
        cancel_at_period_end: bool,
    ) -> Result<Subscription, CoreError> {
        let subscription = subscription::Entity::find_by_id(subscription_id)
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            .ok_or(CoreError::NotFound)?;

        // Extract current_period_end before moving subscription
        let current_period_end = subscription.current_period_end;

        let mut active_model: subscription::ActiveModel = subscription.into_active_model();

        if cancel_at_period_end {
            // Set cancel_at to period_end if it exists
            if current_period_end.is_some() {
                active_model.cancel_at = Set(current_period_end);
            }
            // Set cancel_at_period_end flag
            active_model.cancel_at_period_end = Set(true);
        } else {
            // Immediate cancellation
            active_model.cancel_at = Set(Some(sea_orm::prelude::DateTimeWithTimeZone::from(
                chrono::Utc::now(),
            )));
            active_model.status = Set("canceled".to_string());
        }

        let updated = active_model
            .update(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Self::model_to_subscription(updated)
    }

    async fn count_active_subscriptions_for_plan(&self, plan_id: Uuid) -> Result<i64, CoreError> {
        use sea_orm::EntityTrait;

        let count = subscription::Entity::find()
            .filter(subscription::Column::PlanId.eq(plan_id))
            .filter(subscription::Column::Status.eq("active"))
            .count(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(count as i64)
    }

    // ===== Subscription History =====

    async fn save_history_event(
        &self,
        event: SubscriptionHistoryEvent,
    ) -> Result<SubscriptionHistoryEvent, CoreError> {
        let active_model = Self::history_event_to_active_model(event.clone());

        let result = subscription_history::Entity::insert(active_model)
            .exec(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        // Return the saved event with the ID from the database
        let saved_event = subscription_history::Entity::find_by_id(result.last_insert_id)
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            .ok_or(CoreError::NotFound)?;

        Self::model_to_subscription_history_event(saved_event)
    }

    async fn get_subscription_history(
        &self,
        realm_id: &str,
        subscription_id: &Uuid,
    ) -> Result<Vec<SubscriptionHistoryEvent>, CoreError> {
        let results = subscription_history::Entity::find()
            .filter(subscription_history::Column::RealmId.eq(realm_id))
            .filter(subscription_history::Column::SubscriptionId.eq(*subscription_id))
            .order_by_desc(subscription_history::Column::Timestamp)
            .all(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        results
            .into_iter()
            .map(Self::model_to_subscription_history_event)
            .collect()
    }

    async fn list_subscription_history(
        &self,
        realm_id: &str,
        query: SubscriptionHistoryQuery,
    ) -> Result<(Vec<SubscriptionHistoryEvent>, u64), CoreError> {
        use sea_orm::EntityTrait;

        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20).min(100); // Max 100 per page

        let mut select = subscription_history::Entity::find()
            .filter(subscription_history::Column::RealmId.eq(realm_id));

        let requires_subscription_join = query.user_id.is_some()
            || query.plan_id.is_some()
            || query.subscription_status.is_some();

        if requires_subscription_join {
            select = select.join(
                JoinType::InnerJoin,
                subscription_history::Relation::Subscription.def(),
            );
        }

        // Apply filters

        if let Some(event_type) = query.event_type {
            select = select.filter(subscription_history::Column::EventType.eq(event_type.as_str()));
        }

        if let Some(user_id) = query.user_id {
            select = select.filter(subscription::Column::ClientAppId.eq(user_id));
        }

        if let Some(plan_id) = query.plan_id {
            select = select.filter(subscription::Column::PlanId.eq(plan_id));
        }

        if let Some(subscription_status) = query.subscription_status {
            select = select.filter(subscription::Column::Status.eq(subscription_status));
        }

        if let Some(from_date) = query.from_date {
            select = select.filter(
                subscription_history::Column::Timestamp
                    .gte(sea_orm::prelude::DateTimeWithTimeZone::from(from_date)),
            );
        }

        if let Some(to_date) = query.to_date {
            select = select.filter(
                subscription_history::Column::Timestamp
                    .lte(sea_orm::prelude::DateTimeWithTimeZone::from(to_date)),
            );
        }

        // Get total count
        let total = select
            .clone()
            .count(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        // Apply sorting
        let sort_by = query
            .sort_by
            .unwrap_or_else(|| "timestamp".to_string())
            .to_lowercase();
        let sort_order = query.sort_order.unwrap_or(SortOrder::Desc);

        select = match sort_by.as_str() {
            "timestamp" => {
                if matches!(sort_order, SortOrder::Desc) {
                    select.order_by_desc(subscription_history::Column::Timestamp)
                } else {
                    select.order_by_asc(subscription_history::Column::Timestamp)
                }
            }
            "created_at" => {
                if matches!(sort_order, SortOrder::Desc) {
                    select.order_by_desc(subscription_history::Column::CreatedAt)
                } else {
                    select.order_by_asc(subscription_history::Column::CreatedAt)
                }
            }
            "event_type" => {
                if matches!(sort_order, SortOrder::Desc) {
                    select.order_by_desc(subscription_history::Column::EventType)
                } else {
                    select.order_by_asc(subscription_history::Column::EventType)
                }
            }
            _ => {
                // Default to timestamp DESC
                select.order_by_desc(subscription_history::Column::Timestamp)
            }
        };

        // Apply pagination
        let results = select
            .paginate(&self.db, page_size)
            .fetch_page(page - 1)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        let events = results
            .into_iter()
            .map(Self::model_to_subscription_history_event)
            .collect::<Result<Vec<_>, _>>()?;

        Ok((events, total))
    }

    // ===== Plan Payment Provider Mapping =====

    async fn create_plan_payment_provider(
        &self,
        mapping: PlanPaymentProvider,
    ) -> Result<PlanPaymentProvider, CoreError> {
        let query = r#"
            INSERT INTO plan_payment_provider (
                id, plan_id, payment_provider, external_product_id,
                external_price_id, enabled, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (plan_id, payment_provider) DO NOTHING
            RETURNING id, plan_id, payment_provider, external_product_id,
                      external_price_id, enabled, created_at, updated_at
        "#;

        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            query,
            [
                mapping.id.into(),
                mapping.plan_id.into(),
                mapping.payment_provider.clone().into(),
                mapping.external_product_id.clone().into(),
                mapping.external_price_id.clone().into(),
                mapping.enabled.into(),
                sea_orm::prelude::DateTimeWithTimeZone::from(mapping.created_at).into(),
                sea_orm::prelude::DateTimeWithTimeZone::from(mapping.updated_at).into(),
            ],
        );

        let result = PlanPaymentProviderQueryResult::find_by_statement(stmt)
            .one(&self.db)
            .await
            .map_err(|e| {
                if e.to_string().contains("duplicate key") {
                    CoreError::BadRequest(format!(
                        "Payment provider '{}' is already configured for this plan",
                        mapping.payment_provider
                    ))
                } else {
                    CoreError::DatabaseError(format!(
                        "Failed to create plan payment provider: {}",
                        e
                    ))
                }
            })?
            .ok_or_else(|| {
                CoreError::DatabaseError(
                    "Failed to create plan payment provider: no result".to_string(),
                )
            })?;

        Ok(query_result_to_plan_payment_provider(result))
    }

    async fn find_plan_payment_provider_by_id(
        &self,
        mapping_id: Uuid,
    ) -> Result<Option<PlanPaymentProvider>, CoreError> {
        let query = r#"
            SELECT id, plan_id, payment_provider, external_product_id,
                   external_price_id, enabled, created_at, updated_at
            FROM plan_payment_provider
            WHERE id = $1
        "#;

        let stmt =
            Statement::from_sql_and_values(DatabaseBackend::Postgres, query, [mapping_id.into()]);

        let result = PlanPaymentProviderQueryResult::find_by_statement(stmt)
            .one(&self.db)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to find plan payment provider: {}", e))
            })?;

        Ok(result.map(query_result_to_plan_payment_provider))
    }

    async fn find_plan_payment_provider_by_plan_and_provider(
        &self,
        plan_id: Uuid,
        payment_provider: &str,
    ) -> Result<Option<PlanPaymentProvider>, CoreError> {
        let query = r#"
            SELECT id, plan_id, payment_provider, external_product_id,
                   external_price_id, enabled, created_at, updated_at
            FROM plan_payment_provider
            WHERE plan_id = $1 AND payment_provider = $2
        "#;

        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            query,
            [plan_id.into(), payment_provider.to_string().into()],
        );

        let result = PlanPaymentProviderQueryResult::find_by_statement(stmt)
            .one(&self.db)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to find plan payment provider: {}", e))
            })?;

        Ok(result.map(query_result_to_plan_payment_provider))
    }

    async fn list_plan_payment_providers(
        &self,
        plan_id: Uuid,
    ) -> Result<Vec<PlanPaymentProvider>, CoreError> {
        let query = r#"
            SELECT id, plan_id, payment_provider, external_product_id,
                   external_price_id, enabled, created_at, updated_at
            FROM plan_payment_provider
            WHERE plan_id = $1
            ORDER BY created_at ASC
        "#;

        let stmt =
            Statement::from_sql_and_values(DatabaseBackend::Postgres, query, [plan_id.into()]);

        let results = PlanPaymentProviderQueryResult::find_by_statement(stmt)
            .all(&self.db)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to list plan payment providers: {}", e))
            })?;

        Ok(results
            .into_iter()
            .map(query_result_to_plan_payment_provider)
            .collect())
    }

    async fn update_plan_payment_provider(
        &self,
        mapping: PlanPaymentProvider,
    ) -> Result<PlanPaymentProvider, CoreError> {
        let query = r#"
            UPDATE plan_payment_provider
            SET external_product_id = $2,
                external_price_id = $3,
                enabled = $4,
                updated_at = $5
            WHERE id = $1
            RETURNING id, plan_id, payment_provider, external_product_id,
                      external_price_id, enabled, created_at, updated_at
        "#;

        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            query,
            [
                mapping.id.into(),
                mapping.external_product_id.clone().into(),
                mapping.external_price_id.clone().into(),
                mapping.enabled.into(),
                sea_orm::prelude::DateTimeWithTimeZone::from(mapping.updated_at).into(),
            ],
        );

        let result = PlanPaymentProviderQueryResult::find_by_statement(stmt)
            .one(&self.db)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to update plan payment provider: {}", e))
            })?
            .ok_or_else(|| {
                CoreError::DatabaseError(
                    "Failed to update plan payment provider: no result".to_string(),
                )
            })?;

        Ok(query_result_to_plan_payment_provider(result))
    }

    async fn delete_plan_payment_provider(&self, mapping_id: Uuid) -> Result<(), CoreError> {
        let query = r#"
            DELETE FROM plan_payment_provider
            WHERE id = $1
        "#;

        let stmt =
            Statement::from_sql_and_values(DatabaseBackend::Postgres, query, [mapping_id.into()]);

        self.db.execute(stmt).await.map_err(|e| {
            CoreError::DatabaseError(format!("Failed to delete plan payment provider: {}", e))
        })?;

        Ok(())
    }

    async fn delete_plan_payment_providers_by_plan(&self, plan_id: Uuid) -> Result<(), CoreError> {
        let query = r#"
            DELETE FROM plan_payment_provider
            WHERE plan_id = $1
        "#;

        let stmt =
            Statement::from_sql_and_values(DatabaseBackend::Postgres, query, [plan_id.into()]);

        self.db.execute(stmt).await.map_err(|e| {
            CoreError::DatabaseError(format!(
                "Failed to delete plan payment providers by plan: {}",
                e
            ))
        })?;

        Ok(())
    }

    async fn list_plan_payment_providers_batch(
        &self,
        plan_ids: &[Uuid],
    ) -> Result<Vec<PlanPaymentProvider>, CoreError> {
        if plan_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Build IN clause with parameterized values
        let query = r#"
            SELECT id, plan_id, payment_provider, external_product_id,
                   external_price_id, enabled, created_at, updated_at
            FROM plan_payment_provider
            WHERE plan_id = ANY($1)
            ORDER BY created_at ASC
        "#;

        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            query,
            [plan_ids.to_vec().into()],
        );

        let results = PlanPaymentProviderQueryResult::find_by_statement(stmt)
            .all(&self.db)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!(
                    "Failed to list plan payment providers batch: {}",
                    e
                ))
            })?;

        Ok(results
            .into_iter()
            .map(query_result_to_plan_payment_provider)
            .collect())
    }
}
