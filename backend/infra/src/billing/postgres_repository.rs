use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, IntoActiveModel, JoinType, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait, Set, TransactionTrait,
};
use std::str::FromStr;
use uuid::Uuid;

use herald_domain::billing::entities::EntitlementMapping;
use herald_domain::billing::{
    BillingRepository, HistoryEventType, PaymentEvent, SortOrder, Subscription,
    SubscriptionHistoryEvent, SubscriptionHistoryQuery,
};
use herald_domain::common::entities::app_errors::CoreError;
use herald_entity::{
    payment_event, provider_entitlement_mapping, subscription, subscription_history,
};

/// PostgreSQL implementation of billing repository
pub struct PostgresBillingRepository {
    db: DatabaseConnection,
}

impl PostgresBillingRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn begin_transaction(&self) -> Result<DatabaseTransaction, CoreError> {
        self.db
            .begin()
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))
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
            entitlement_key: model.entitlement_key,
            external_price_id: model.external_price_id,
            provider_metadata: model.provider_metadata,
            synced_at: model.synced_at.map(chrono::DateTime::from),
            current_period_start: model.current_period_start.map(chrono::DateTime::from),
            current_period_end: model.current_period_end.map(chrono::DateTime::from),
            cancel_at_period_end: model.cancel_at_period_end,
            client_app_id: model.client_app_id,
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

    /// Converts database model to domain EntitlementMapping
    fn model_to_entitlement_mapping(
        model: provider_entitlement_mapping::Model,
    ) -> EntitlementMapping {
        EntitlementMapping {
            id: model.id,
            realm_id: model.realm_id,
            payment_provider: model.payment_provider,
            external_product_id: model.external_product_id,
            external_price_id: model.external_price_id,
            entitlement_key: model.entitlement_key,
            billing_type: model.billing_type.and_then(|s| s.parse().ok()),
            billing_period: model.billing_period,
            points_per_period: model.points_per_period.map(|v| v as i64),
            grant_period_type: model.grant_period_type,
            validity_days: model.validity_days.map(|v| v as i64),
            grant_on_subscribe: model.grant_on_subscribe,
            max_periods: model.max_periods.map(|v| v as i64),
            enabled: model.enabled,
            provider_product_info: model.provider_product_info,
            synced_at: model.synced_at.map(chrono::DateTime::from),
            created_at: chrono::DateTime::from(model.created_at),
            updated_at: chrono::DateTime::from(model.updated_at),
        }
    }

    /// Converts domain EntitlementMapping to database active model
    fn entitlement_mapping_to_active_model(
        mapping: EntitlementMapping,
    ) -> provider_entitlement_mapping::ActiveModel {
        provider_entitlement_mapping::ActiveModel {
            id: Set(mapping.id),
            realm_id: Set(mapping.realm_id),
            payment_provider: Set(mapping.payment_provider),
            external_product_id: Set(mapping.external_product_id),
            external_price_id: Set(mapping.external_price_id),
            entitlement_key: Set(mapping.entitlement_key),
            billing_type: Set(mapping.billing_type.map(|t| t.as_str().to_string())),
            billing_period: Set(mapping.billing_period),
            points_per_period: Set(mapping.points_per_period.map(|v| v as i32)),
            grant_period_type: Set(mapping.grant_period_type),
            validity_days: Set(mapping.validity_days.map(|v| v as i32)),
            grant_on_subscribe: Set(mapping.grant_on_subscribe),
            max_periods: Set(mapping.max_periods.map(|v| v as i32)),
            enabled: Set(mapping.enabled),
            provider_product_info: Set(mapping.provider_product_info),
            synced_at: Set(mapping
                .synced_at
                .map(sea_orm::prelude::DateTimeWithTimeZone::from)),
            created_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(
                mapping.created_at,
            )),
            updated_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(
                mapping.updated_at,
            )),
        }
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

    fn subscription_to_active_model(sub: Subscription) -> subscription::ActiveModel {
        subscription::ActiveModel {
            id: Set(sub.id),
            realm_id: Set(sub.realm_id.clone()),
            user_id: Set(sub.user_id),
            external_subscription_id: Set(sub.external_subscription_id),
            external_product_id: Set(sub.external_product_id.clone()),
            payment_provider: Set(sub.payment_provider.clone()),
            status: Set(sub.status.as_str().to_string()),
            entitlement_key: Set(sub.entitlement_key.clone()),
            external_price_id: Set(sub.external_price_id.clone()),
            provider_metadata: Set(sub.provider_metadata.clone()),
            synced_at: Set(sub
                .synced_at
                .map(sea_orm::prelude::DateTimeWithTimeZone::from)),
            current_period_start: Set(sub
                .current_period_start
                .map(sea_orm::prelude::DateTimeWithTimeZone::from)),
            current_period_end: Set(sub
                .current_period_end
                .map(sea_orm::prelude::DateTimeWithTimeZone::from)),
            cancel_at_period_end: Set(sub.cancel_at_period_end),
            client_app_id: Set(sub.client_app_id),
            cancel_at: Set(sub
                .cancel_at
                .map(sea_orm::prelude::DateTimeWithTimeZone::from)),
            created_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(sub.created_at)),
            updated_at: Set(sea_orm::prelude::DateTimeWithTimeZone::from(sub.updated_at)),
        }
    }

    fn apply_subscription_update(active_model: &mut subscription::ActiveModel, sub: Subscription) {
        active_model.realm_id = Set(sub.realm_id.clone());
        active_model.user_id = Set(sub.user_id);
        active_model.external_subscription_id = Set(sub.external_subscription_id.clone());
        active_model.external_product_id = Set(sub.external_product_id.clone());
        active_model.payment_provider = Set(sub.payment_provider.clone());
        active_model.status = Set(sub.status.as_str().to_string());
        active_model.entitlement_key = Set(sub.entitlement_key.clone());
        active_model.external_price_id = Set(sub.external_price_id.clone());
        active_model.provider_metadata = Set(sub.provider_metadata.clone());
        active_model.synced_at = Set(sub
            .synced_at
            .map(sea_orm::prelude::DateTimeWithTimeZone::from));
        active_model.current_period_start = Set(sub
            .current_period_start
            .map(sea_orm::prelude::DateTimeWithTimeZone::from));
        active_model.current_period_end = Set(sub
            .current_period_end
            .map(sea_orm::prelude::DateTimeWithTimeZone::from));
        active_model.cancel_at_period_end = Set(sub.cancel_at_period_end);
        active_model.client_app_id = Set(sub.client_app_id);
        active_model.cancel_at = Set(sub
            .cancel_at
            .map(sea_orm::prelude::DateTimeWithTimeZone::from));
        active_model.updated_at = Set(sea_orm::prelude::DateTimeWithTimeZone::from(sub.updated_at));
    }

    pub async fn create_subscription_conn<C: ConnectionTrait>(
        db: &C,
        sub: Subscription,
    ) -> Result<Subscription, CoreError> {
        let result = Self::subscription_to_active_model(sub).insert(db).await?;
        Self::model_to_subscription(result)
    }

    pub async fn find_by_external_subscription_id_conn<C: ConnectionTrait>(
        db: &C,
        external_sub_id: &str,
        provider: &str,
    ) -> Result<Option<Subscription>, CoreError> {
        let result = subscription::Entity::find()
            .filter(subscription::Column::ExternalSubscriptionId.eq(external_sub_id))
            .filter(subscription::Column::PaymentProvider.eq(provider))
            .one(db)
            .await?;

        result.map(Self::model_to_subscription).transpose()
    }

    pub async fn find_subscription_by_client_app_id_conn<C: ConnectionTrait>(
        db: &C,
        client_app_id: Uuid,
    ) -> Result<Option<Subscription>, CoreError> {
        let result = subscription::Entity::find()
            .filter(subscription::Column::ClientAppId.eq(client_app_id))
            .one(db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        result.map(Self::model_to_subscription).transpose()
    }

    pub async fn update_subscription_conn<C: ConnectionTrait>(
        db: &C,
        sub: Subscription,
    ) -> Result<Subscription, CoreError> {
        let existing = subscription::Entity::find_by_id(sub.id)
            .one(db)
            .await?
            .ok_or_else(|| CoreError::SubscriptionNotFound(sub.id.to_string()))?;

        let mut active_model: subscription::ActiveModel = existing.into_active_model();
        Self::apply_subscription_update(&mut active_model, sub);

        let result = active_model.update(db).await?;
        Self::model_to_subscription(result)
    }

    pub async fn save_history_event_conn<C: ConnectionTrait>(
        db: &C,
        event: SubscriptionHistoryEvent,
    ) -> Result<SubscriptionHistoryEvent, CoreError> {
        let active_model = Self::history_event_to_active_model(event);

        let result = subscription_history::Entity::insert(active_model)
            .exec(db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        let saved_event = subscription_history::Entity::find_by_id(result.last_insert_id)
            .one(db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            .ok_or(CoreError::NotFound)?;

        Self::model_to_subscription_history_event(saved_event)
    }
}

impl BillingRepository for PostgresBillingRepository {
    async fn create_subscription(&self, sub: Subscription) -> Result<Subscription, CoreError> {
        Self::create_subscription_conn(&self.db, sub).await
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
        Self::find_by_external_subscription_id_conn(&self.db, external_sub_id, provider).await
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
        Self::update_subscription_conn(&self.db, sub).await
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

    async fn find_subscription_by_client_app_id(
        &self,
        client_app_id: Uuid,
    ) -> Result<Option<Subscription>, CoreError> {
        Self::find_subscription_by_client_app_id_conn(&self.db, client_app_id).await
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

        let updated: subscription::Model = active_model
            .update(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Self::model_to_subscription(updated)
    }

    // ===== Subscription History =====

    async fn save_history_event(
        &self,
        event: SubscriptionHistoryEvent,
    ) -> Result<SubscriptionHistoryEvent, CoreError> {
        Self::save_history_event_conn(&self.db, event).await
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
            || query.entitlement_key.is_some()
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
            select = select.filter(subscription::Column::UserId.eq(user_id));
        }

        if let Some(entitlement_key) = query.entitlement_key {
            select = select.filter(subscription::Column::EntitlementKey.eq(entitlement_key));
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

    // ===== Entitlement Mapping CRUD =====

    async fn create_entitlement_mapping(
        &self,
        mapping: EntitlementMapping,
    ) -> Result<EntitlementMapping, CoreError> {
        let active_model = Self::entitlement_mapping_to_active_model(mapping);
        let result = active_model.insert(&self.db).await.map_err(|e| {
            if e.to_string().contains("duplicate key")
                || e.to_string()
                    .contains("provider_entitlement_mappings_realm_id_payment_provider_external_product_id_key")
            {
                CoreError::Conflict("Entitlement mapping already exists for this provider and product".to_string())
            } else {
                CoreError::DatabaseError(e.to_string())
            }
        })?;
        Ok(Self::model_to_entitlement_mapping(result))
    }

    async fn find_entitlement_mapping_by_id(
        &self,
        mapping_id: Uuid,
    ) -> Result<Option<EntitlementMapping>, CoreError> {
        let result = provider_entitlement_mapping::Entity::find_by_id(mapping_id)
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(result.map(Self::model_to_entitlement_mapping))
    }

    async fn list_entitlement_mappings(
        &self,
        realm_id: &str,
        payment_provider: Option<&str>,
        enabled: Option<bool>,
        page: Option<u64>,
        page_size: Option<u64>,
    ) -> Result<(Vec<EntitlementMapping>, u64), CoreError> {
        let page = page.unwrap_or(1);
        let page_size = page_size.unwrap_or(20).min(100);

        let mut query = provider_entitlement_mapping::Entity::find()
            .filter(provider_entitlement_mapping::Column::RealmId.eq(realm_id));

        if let Some(provider) = payment_provider {
            query =
                query.filter(provider_entitlement_mapping::Column::PaymentProvider.eq(provider));
        }
        if let Some(enabled) = enabled {
            query = query.filter(provider_entitlement_mapping::Column::Enabled.eq(enabled));
        }

        let total = query
            .clone()
            .count(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        let results = query
            .order_by_asc(provider_entitlement_mapping::Column::CreatedAt)
            .paginate(&self.db, page_size)
            .fetch_page(page - 1)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        let mappings = results
            .into_iter()
            .map(Self::model_to_entitlement_mapping)
            .collect();

        Ok((mappings, total))
    }

    async fn update_entitlement_mapping(
        &self,
        mapping: EntitlementMapping,
    ) -> Result<EntitlementMapping, CoreError> {
        let existing = provider_entitlement_mapping::Entity::find_by_id(mapping.id)
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            .ok_or(CoreError::NotFound)?;

        let mut active_model: provider_entitlement_mapping::ActiveModel =
            existing.into_active_model();
        let update = Self::entitlement_mapping_to_active_model(mapping);
        active_model.entitlement_key = update.entitlement_key;
        active_model.billing_type = update.billing_type;
        active_model.billing_period = update.billing_period;
        active_model.points_per_period = update.points_per_period;
        active_model.grant_period_type = update.grant_period_type;
        active_model.validity_days = update.validity_days;
        active_model.grant_on_subscribe = update.grant_on_subscribe;
        active_model.max_periods = update.max_periods;
        active_model.enabled = update.enabled;
        active_model.provider_product_info = update.provider_product_info;
        active_model.synced_at = update.synced_at;
        active_model.updated_at = update.updated_at;

        let result = active_model
            .update(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(Self::model_to_entitlement_mapping(result))
    }

    async fn upsert_entitlement_mapping(
        &self,
        mapping: EntitlementMapping,
    ) -> Result<EntitlementMapping, CoreError> {
        // Try to find existing by unique constraint
        let existing = self
            .find_entitlement_mapping_by_provider_product(
                &mapping.realm_id,
                &mapping.payment_provider,
                &mapping.external_product_id,
            )
            .await?;

        match existing {
            Some(mut existing_mapping) => {
                // Update existing
                existing_mapping.entitlement_key = mapping.entitlement_key;
                existing_mapping.external_price_id = mapping.external_price_id;
                existing_mapping.billing_type = mapping.billing_type;
                existing_mapping.billing_period = mapping.billing_period;
                existing_mapping.points_per_period = mapping.points_per_period;
                existing_mapping.grant_period_type = mapping.grant_period_type;
                existing_mapping.validity_days = mapping.validity_days;
                existing_mapping.grant_on_subscribe = mapping.grant_on_subscribe;
                existing_mapping.max_periods = mapping.max_periods;
                existing_mapping.enabled = mapping.enabled;
                existing_mapping.provider_product_info = mapping.provider_product_info;
                existing_mapping.synced_at = mapping.synced_at;
                existing_mapping.updated_at = chrono::Utc::now();
                self.update_entitlement_mapping(existing_mapping).await
            }
            None => self.create_entitlement_mapping(mapping).await,
        }
    }

    async fn find_entitlement_mapping_by_provider_product(
        &self,
        realm_id: &str,
        payment_provider: &str,
        external_product_id: &str,
    ) -> Result<Option<EntitlementMapping>, CoreError> {
        let result = provider_entitlement_mapping::Entity::find()
            .filter(provider_entitlement_mapping::Column::RealmId.eq(realm_id))
            .filter(provider_entitlement_mapping::Column::PaymentProvider.eq(payment_provider))
            .filter(provider_entitlement_mapping::Column::ExternalProductId.eq(external_product_id))
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(result.map(Self::model_to_entitlement_mapping))
    }

    async fn find_entitlement_mapping_by_key(
        &self,
        realm_id: &str,
        entitlement_key: &str,
    ) -> Result<Option<EntitlementMapping>, CoreError> {
        let result = provider_entitlement_mapping::Entity::find()
            .filter(provider_entitlement_mapping::Column::RealmId.eq(realm_id))
            .filter(provider_entitlement_mapping::Column::EntitlementKey.eq(entitlement_key))
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(result.map(Self::model_to_entitlement_mapping))
    }

    async fn list_one_time_mappings(
        &self,
        realm_id: &str,
    ) -> Result<Vec<EntitlementMapping>, CoreError> {
        use sea_orm::QueryFilter;

        let results = provider_entitlement_mapping::Entity::find()
            .filter(provider_entitlement_mapping::Column::RealmId.eq(realm_id))
            .filter(provider_entitlement_mapping::Column::Enabled.eq(true))
            .filter(provider_entitlement_mapping::Column::BillingType.eq("one_time"))
            .filter(provider_entitlement_mapping::Column::ProviderProductInfo.is_not_null())
            .order_by_asc(provider_entitlement_mapping::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(Self::model_to_entitlement_mapping)
            .collect())
    }

    async fn find_external_subscription_id_by_payment_intent(
        &self,
        payment_intent: &str,
        provider: &str,
        realm_id: &str,
    ) -> Result<Option<String>, CoreError> {
        let stripe_subscription_id: Option<String> = sqlx::query_scalar(
            "SELECT payload->'data'->'object'->>'subscription' \
             FROM payment_event \
             WHERE payment_provider = $1 \
               AND realm_id = $3 \
               AND event_type IN ('checkout.session.completed', 'invoice.payment_succeeded') \
               AND payload->'data'->'object'->>'payment_intent' = $2 \
             LIMIT 1",
        )
        .bind(provider)
        .bind(payment_intent)
        .bind(realm_id)
        .fetch_optional(self.db.get_postgres_connection_pool())
        .await
        .map_err(|e| {
            CoreError::InternalServerError(format!(
                "Failed to lookup subscription by payment_intent: {}",
                e
            ))
        })?
        .flatten();

        Ok(stripe_subscription_id)
    }
}
