use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, IntoActiveModel, JoinType, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait, Set, TransactionTrait,
};
use std::str::FromStr;
use uuid::Uuid;

use herald_domain::billing::credit_bucket::{
    CreateCreditBucketInput, CreditBucket, CreditBucketDetail, CreditBucketError,
    CreditBucketListItem, CreditBucketOverview, CreditBucketOverviewRow, UpdateCreditBucketInput,
};
use herald_domain::billing::entities::EntitlementMapping;
use herald_domain::billing::{
    BatchMappingError, BatchUpdateMappingsInput, BatchUpdateResult, BillingRepository,
    FeatureFacts, HistoryEventType, PaymentEvent, SortOrder, Subscription,
    SubscriptionHistoryEvent, SubscriptionHistoryQuery,
};
use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::points::entities::QuotaWindow;
use herald_domain::points::services::registration_pool_resolver::RegistrationPoolResolver;
use herald_entity::{
    payment_event, provider_entitlement_mapping, subscription, subscription_history,
};

use crate::points::postgres_repository::{
    parse_quota_windows_value, serialize_quota_windows_value,
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
            billing_type: model.billing_type.parse()?,
            external_price_id: model.external_price_id,
            bucket_id: model.bucket_id,
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
        // Hydrate quota_windows via the shared infra serde boundary
        // JSONB array ⟺ `Some(Vec<QuotaWindow>)`. A malformed JSONB value is
        // logged and treated as "no window grant" rather than poisoning the
        // whole read (the read path must not crash a list on one bad row).
        let quota_windows = parse_quota_windows_value(model.quota_windows)
            .map_err(|e| {
                tracing::warn!(error = %e, mapping_id = %model.id, "Malformed quota_windows JSONB on entitlement mapping; treating as no window grant");
                e
            })
            .ok();
        let quota_windows = quota_windows.filter(|w| !w.is_empty());
        EntitlementMapping {
            id: model.id,
            realm_id: model.realm_id,
            payment_provider: model.payment_provider,
            external_product_id: model.external_product_id,
            external_price_id: model.external_price_id,
            bucket_id: model.bucket_id,
            entitlement_key: model.entitlement_key,
            billing_type: model.billing_type.and_then(|s| s.parse().ok()),
            billing_period: model.billing_period,
            service_duration_days: model.service_duration_days.map(|v| v as i64),
            points_per_period: model.points_per_period.map(|v| v as i64),
            grant_period_type: model.grant_period_type,
            validity_days: model.validity_days.map(|v| v as i64),
            grant_on_subscribe: model.grant_on_subscribe,
            max_periods: model.max_periods.map(|v| v as i64),
            enabled: model.enabled,
            provider_product_info: model.provider_product_info,
            quota_windows,
            granted_role_ids: model.granted_role_ids,
            synced_at: model.synced_at.map(chrono::DateTime::from),
            created_at: chrono::DateTime::from(model.created_at),
            updated_at: chrono::DateTime::from(model.updated_at),
        }
    }

    /// Converts domain EntitlementMapping to database active model
    fn entitlement_mapping_to_active_model(
        mapping: EntitlementMapping,
    ) -> provider_entitlement_mapping::ActiveModel {
        // `None`/empty ⟺ SQL NULL (no window grant).
        let quota_windows = mapping
            .quota_windows
            .as_deref()
            .and_then(|w| serialize_quota_windows_value(w).ok().flatten());
        provider_entitlement_mapping::ActiveModel {
            id: Set(mapping.id),
            realm_id: Set(mapping.realm_id),
            payment_provider: Set(mapping.payment_provider),
            external_product_id: Set(mapping.external_product_id),
            external_price_id: Set(mapping.external_price_id),
            bucket_id: Set(mapping.bucket_id),
            entitlement_key: Set(mapping.entitlement_key),
            billing_type: Set(mapping.billing_type.map(|t| t.as_str().to_string())),
            billing_period: Set(mapping.billing_period),
            service_duration_days: Set(mapping.service_duration_days.map(|v| v as i32)),
            points_per_period: Set(mapping.points_per_period.map(|v| v as i32)),
            grant_period_type: Set(mapping.grant_period_type),
            validity_days: Set(mapping.validity_days.map(|v| v as i32)),
            grant_on_subscribe: Set(mapping.grant_on_subscribe),
            max_periods: Set(mapping.max_periods.map(|v| v as i32)),
            enabled: Set(mapping.enabled),
            provider_product_info: Set(mapping.provider_product_info),
            quota_windows: Set(quota_windows),
            granted_role_ids: Set(mapping.granted_role_ids),
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
            billing_type: Set(sub.billing_type.as_str().to_string()),
            external_price_id: Set(sub.external_price_id.clone()),
            bucket_id: Set(sub.bucket_id),
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
        active_model.billing_type = Set(sub.billing_type.as_str().to_string());
        active_model.external_price_id = Set(sub.external_price_id.clone());
        active_model.bucket_id = Set(sub.bucket_id);
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

    // All multi-table writes run in a single sqlx transaction (matches the
    // invoice_postgres_repository pattern: `self.db.get_postgres_connection_pool().begin()`).
    // Coverage-set changes only affect future routing; attached-mapping
    // replacement only sets `bucket_id` on the listed mappings (does not touch
    // balances). Registration-pool uniqueness is guarded by pre-check + the partial
    // unique index `uq_credit_buckets_registration_pool` (caught → 409 conflict).

    /// Create a Credit Bucket with its coverage set and optional attached mappings.
    ///
    /// Transaction:
    /// 1. INSERT `credit_buckets` (raises unique violation on registration-pool
    ///    collision → `RegistrationPoolConflict`).
    /// 2. INSERT `credit_bucket_client_apps` rows for the coverage set.
    /// 3. UPDATE `provider_entitlement_mappings SET bucket_id=$1` for the listed
    ///    mapping ids (scoped to realm).
    pub async fn create_credit_bucket(
        &self,
        input: CreateCreditBucketInput,
    ) -> Result<CreditBucketDetail, CreditBucketError> {
        let id = Uuid::now_v7();
        let mut tx = self
            .db
            .get_postgres_connection_pool()
            .begin()
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        // 1. Insert bucket. The partial unique index uq_credit_buckets_registration_pool
        //    fires on conflict when receives_registration_credits=true.
        let row = sqlx::query(
            "INSERT INTO credit_buckets \
             (id, realm_id, bucket_key, name, description, display_order, \
              receives_registration_credits, enabled, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW()) \
             RETURNING id, realm_id, bucket_key, name, description, display_order, \
                       receives_registration_credits, enabled",
        )
        .bind(id)
        .bind(&input.realm_id)
        .bind(&input.bucket_key)
        .bind(&input.name)
        .bind(input.description.as_deref())
        .bind(input.display_order)
        .bind(input.receives_registration_credits)
        .bind(input.enabled)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| Self::classify_bucket_insert_error(&e, &input.realm_id))?;

        let bucket = Self::row_to_credit_bucket(&row);

        // 2. Insert coverage set.
        for client_app_id in &input.client_app_ids {
            sqlx::query(
                "INSERT INTO credit_bucket_client_apps \
                 (bucket_id, client_app_id, realm_id, created_at) \
                 VALUES ($1, $2, $3, NOW()) \
                 ON CONFLICT (bucket_id, client_app_id) DO NOTHING",
            )
            .bind(id)
            .bind(client_app_id)
            .bind(&input.realm_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to insert coverage row: {}", e))
            })?;
        }

        // 3. Attach mappings (set their bucket_id). Scoped to realm to prevent
        //    cross-realm attachment.
        if !input.entitlement_mapping_ids.is_empty() {
            Self::reattach_mappings_tx(
                &mut tx,
                id,
                &input.realm_id,
                &input.entitlement_mapping_ids,
            )
            .await?;
        }

        tx.commit().await.map_err(|e| {
            CoreError::DatabaseError(format!("Failed to commit create_credit_bucket: {}", e))
        })?;

        Ok(CreditBucketDetail {
            bucket,
            client_app_ids: input.client_app_ids,
            entitlement_mapping_ids: input.entitlement_mapping_ids,
        })
    }

    /// Get a single Credit Bucket with its coverage set and attached mappings.
    pub async fn get_credit_bucket(
        &self,
        realm_id: &str,
        bucket_id: Uuid,
    ) -> Result<Option<CreditBucketDetail>, CoreError> {
        let row = sqlx::query(
            "SELECT id, realm_id, bucket_key, name, description, display_order, \
                    receives_registration_credits, enabled \
             FROM credit_buckets WHERE realm_id = $1 AND id = $2",
        )
        .bind(realm_id)
        .bind(bucket_id)
        .fetch_optional(self.db.get_postgres_connection_pool())
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to fetch credit bucket: {}", e)))?;

        let Some(row) = row else {
            return Ok(None);
        };
        let bucket = Self::row_to_credit_bucket(&row);

        let client_app_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT client_app_id FROM credit_bucket_client_apps \
             WHERE realm_id = $1 AND bucket_id = $2 ORDER BY client_app_id",
        )
        .bind(realm_id)
        .bind(bucket_id)
        .fetch_all(self.db.get_postgres_connection_pool())
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to fetch coverage set: {}", e)))?;

        let entitlement_mapping_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM provider_entitlement_mappings \
             WHERE realm_id = $1 AND bucket_id = $2 ORDER BY id",
        )
        .bind(realm_id)
        .bind(bucket_id)
        .fetch_all(self.db.get_postgres_connection_pool())
        .await
        .map_err(|e| {
            CoreError::DatabaseError(format!("Failed to fetch attached mappings: {}", e))
        })?;

        Ok(Some(CreditBucketDetail {
            bucket,
            client_app_ids,
            entitlement_mapping_ids,
        }))
    }

    /// List Credit Buckets for a realm with aggregate counts.
    pub async fn list_credit_buckets(
        &self,
        realm_id: &str,
    ) -> Result<Vec<CreditBucketListItem>, CoreError> {
        use sqlx::Row;

        let rows = sqlx::query(
            "SELECT b.id, b.realm_id, b.bucket_key, b.name, b.description, b.display_order, \
                    b.receives_registration_credits, b.enabled, \
                    COALESCE(ca.covered_count, 0) AS covered_client_app_count, \
                    COALESCE(m.mapping_count, 0) AS entitlement_mapping_count \
             FROM credit_buckets b \
             LEFT JOIN ( \
                 SELECT bucket_id, COUNT(*) AS covered_count \
                 FROM credit_bucket_client_apps GROUP BY bucket_id \
             ) ca ON ca.bucket_id = b.id \
             LEFT JOIN ( \
                 SELECT bucket_id, COUNT(*) AS mapping_count \
                 FROM provider_entitlement_mappings WHERE bucket_id IS NOT NULL \
                 GROUP BY bucket_id \
             ) m ON m.bucket_id = b.id \
             WHERE b.realm_id = $1 \
             ORDER BY b.display_order ASC, b.created_at ASC",
        )
        .bind(realm_id)
        .fetch_all(self.db.get_postgres_connection_pool())
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to list credit buckets: {}", e)))?;

        let items = rows
            .iter()
            .map(|row| {
                let bucket = Self::row_to_credit_bucket(row);
                CreditBucketListItem {
                    bucket,
                    covered_client_app_count: row.get("covered_client_app_count"),
                    entitlement_mapping_count: row.get("entitlement_mapping_count"),
                }
            })
            .collect();

        Ok(items)
    }

    /// Update a Credit Bucket: base fields + coverage-set replace + attached-mapping
    /// move-in (NOT NULL `bucket_id`: mappings may join this bucket but not leave it
    /// via PUT — removal is rejected as `BucketOrphanMapping`) + registration-pool
    /// flag toggle (same uniqueness guard as create).
    pub async fn update_credit_bucket(
        &self,
        input: UpdateCreditBucketInput,
    ) -> Result<CreditBucketDetail, CreditBucketError> {
        let mut tx = self
            .db
            .get_postgres_connection_pool()
            .begin()
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        // Lock the bucket row and verify ownership.
        let exists: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM credit_buckets WHERE realm_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(&input.realm_id)
        .bind(input.bucket_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to lock credit bucket: {}", e)))?;
        if exists.is_none() {
            return Err(CoreError::NotFound.into());
        }

        // Update base fields. Registration-pool flag toggle may trip the partial
        // unique index → RegistrationPoolConflict.
        let row = sqlx::query(
            "UPDATE credit_buckets \
             SET name = $3, description = $4, display_order = $5, \
                 receives_registration_credits = $6, enabled = $7, updated_at = NOW() \
             WHERE realm_id = $1 AND id = $2 \
             RETURNING id, realm_id, bucket_key, name, description, display_order, \
                       receives_registration_credits, enabled",
        )
        .bind(&input.realm_id)
        .bind(input.bucket_id)
        .bind(&input.name)
        .bind(input.description.as_deref())
        .bind(input.display_order)
        .bind(input.receives_registration_credits)
        .bind(input.enabled)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| Self::classify_bucket_insert_error(&e, &input.realm_id))?;

        let bucket = Self::row_to_credit_bucket(&row);

        // Replace coverage set (delete + insert). CASCADE-safe since we hold the
        // bucket lock; existing wallets/ledgers are untouched.
        sqlx::query("DELETE FROM credit_bucket_client_apps WHERE bucket_id = $1")
            .bind(input.bucket_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to clear coverage set: {}", e))
            })?;

        for client_app_id in &input.client_app_ids {
            sqlx::query(
                "INSERT INTO credit_bucket_client_apps \
                 (bucket_id, client_app_id, realm_id, created_at) \
                 VALUES ($1, $2, $3, NOW()) \
                 ON CONFLICT (bucket_id, client_app_id) DO NOTHING",
            )
            .bind(input.bucket_id)
            .bind(client_app_id)
            .bind(&input.realm_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to insert coverage row: {}", e))
            })?;
        }

        // Attach mappings. `provider_entitlement_mappings.bucket_id` is NOT NULL
        // (commit `aa6cc2da`) and there is no default bucket, so a
        // mapping can only JOIN this bucket (move-in) — it cannot be removed via
        // this PUT, since detaching would orphan it (no NULL home). Reject any
        // shrink of the attached set with `bucket_orphan_mapping` (400). The
        // caller moves a mapping out by assigning it to another bucket's PUT.
        let current_attached: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM provider_entitlement_mappings \
             WHERE realm_id = $1 AND bucket_id = $2",
        )
        .bind(&input.realm_id)
        .bind(input.bucket_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| {
            CoreError::DatabaseError(format!("Failed to read attached mappings: {}", e))
        })?;
        let orphan_mapping_ids: Vec<Uuid> = current_attached
            .into_iter()
            .filter(|id| !input.entitlement_mapping_ids.contains(id))
            .collect();
        if !orphan_mapping_ids.is_empty() {
            return Err(CreditBucketError::BucketOrphanMapping {
                bucket_id: input.bucket_id,
                orphan_mapping_ids,
            });
        }

        // Re-attach (claim) the requested set for this bucket (realm-scoped).
        // Success here implies the new attached set == the requested set.
        if !input.entitlement_mapping_ids.is_empty() {
            Self::reattach_mappings_tx(
                &mut tx,
                input.bucket_id,
                &input.realm_id,
                &input.entitlement_mapping_ids,
            )
            .await?;
        }

        tx.commit().await.map_err(|e| {
            CoreError::DatabaseError(format!("Failed to commit update_credit_bucket: {}", e))
        })?;

        Ok(CreditBucketDetail {
            bucket,
            client_app_ids: input.client_app_ids,
            entitlement_mapping_ids: input.entitlement_mapping_ids,
        })
    }

    /// Delete a Credit Bucket. Refused with `BucketInUse` when in-flight
    /// subscriptions exist for this bucket OR any wallet still holds a balance.
    pub async fn delete_credit_bucket(
        &self,
        realm_id: &str,
        bucket_id: Uuid,
    ) -> Result<(), CreditBucketError> {
        let mut tx = self
            .db
            .get_postgres_connection_pool()
            .begin()
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        let exists: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM credit_buckets WHERE realm_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(realm_id)
        .bind(bucket_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to lock credit bucket: {}", e)))?;
        if exists.is_none() {
            return Err(CoreError::NotFound.into());
        }

        // In-flight subscriptions (active/trialing/past_due/scheduled_cancel/dispute
        // — match `SubscriptionStatus::has_access` semantics).
        let active_subscriptions: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM subscription \
             WHERE bucket_id = $1 AND status IN \
                   ('active', 'trialing', 'past_due', 'scheduled_cancel', 'dispute')",
        )
        .bind(bucket_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to count subscriptions: {}", e)))?;

        // Holders with remaining *derived available* balance.
        // The delete guard uses the SAME derived
        // availability predicate as `compute_bucket_available_balances`
        // (`status='active' AND remaining_amount>0 AND (effective_at IS NULL
        // OR effective_at<=NOW()) AND (expires_at IS NULL OR
        // expires_at>NOW())`) instead of the Stored `points_wallets.total_balance`
        // column. Future-effective pre-grant rows do NOT block delete here (they
        // are not yet spendable); they are swept by
        // `clear_deletable_bucket_references_tx` below. If the predicate text
        // drifts from the one in `points/postgres_repository.rs`, step 2 will
        // catch it.
        let holders_with_balance: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM points_credit_ledger \
             WHERE bucket_id = $1 \
               AND status = 'active' \
               AND remaining_amount > 0 \
               AND (effective_at IS NULL OR effective_at <= NOW()) \
               AND (expires_at IS NULL OR expires_at > NOW())",
        )
        .bind(bucket_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to count holders: {}", e)))?;

        if active_subscriptions > 0 || holders_with_balance > 0 {
            // Roll back before surfacing the structured error.
            let _ = tx.rollback().await;
            return Err(CreditBucketError::BucketInUse {
                bucket_id,
                active_subscriptions,
                holders_with_balance,
            });
        }

        Self::clear_deletable_bucket_references_tx(&mut tx, bucket_id).await?;

        // Safe to delete after clearing zero-balance points residue and the
        // non-active subscription / payment_attempt / mapping rows still bound
        // to the bucket. Coverage rows cascade from the bucket.
        sqlx::query("DELETE FROM credit_buckets WHERE realm_id = $1 AND id = $2")
            .bind(realm_id)
            .bind(bucket_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to delete credit bucket: {}", e))
            })?;

        tx.commit().await.map_err(|e| {
            CoreError::DatabaseError(format!("Failed to commit delete_credit_bucket: {}", e))
        })?;

        Ok(())
    }

    async fn clear_deletable_bucket_references_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        bucket_id: Uuid,
    ) -> Result<(), CoreError> {
        sqlx::query("DELETE FROM points_consumption_allocations WHERE bucket_id = $1")
            .bind(bucket_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!(
                    "Failed to delete bucket consumption allocations: {}",
                    e
                ))
            })?;

        sqlx::query("DELETE FROM points_transactions WHERE bucket_id = $1")
            .bind(bucket_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to delete bucket transactions: {}", e))
            })?;

        sqlx::query("DELETE FROM points_credit_ledger WHERE bucket_id = $1")
            .bind(bucket_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to delete bucket credit ledger: {}", e))
            })?;

        sqlx::query("DELETE FROM points_grant_schedules WHERE bucket_id = $1")
            .bind(bucket_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to delete bucket grant schedules: {}", e))
            })?;

        // `points_wallets.total_balance` was physically dropped; this is
        // a cleanup of orphan wallet rows for a deleted bucket,
        // NOT a balance-authority read. By this point all ledger rows for the
        // bucket are deleted above, so any remaining wallet rows are orphans
        // (their analytics are retained only for historical totals; with the
        // bucket gone they have no remaining referent). Delete unconditionally.
        sqlx::query("DELETE FROM points_wallets WHERE bucket_id = $1")
            .bind(bucket_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to delete orphan bucket wallets: {}", e))
            })?;

        // bucket_id is NOT NULL on subscription/payment_attempts/mappings, so
        // clearing references means deleting the rows still bound to this
        // bucket. Only non-active subscriptions reach here (active ones are
        // refused above); payment_attempts and mappings are residue that cannot
        // outlive their bucket under a NOT NULL constraint.
        sqlx::query(
            "DELETE FROM subscription \
             WHERE bucket_id = $1 AND status NOT IN \
                   ('active', 'trialing', 'past_due', 'scheduled_cancel', 'dispute')",
        )
        .bind(bucket_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| {
            CoreError::DatabaseError(format!(
                "Failed to delete inactive subscriptions bound to bucket: {}",
                e
            ))
        })?;

        sqlx::query("DELETE FROM payment_attempts WHERE bucket_id = $1")
            .bind(bucket_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!(
                    "Failed to delete payment attempts bound to bucket: {}",
                    e
                ))
            })?;

        sqlx::query("DELETE FROM provider_entitlement_mappings WHERE bucket_id = $1")
            .bind(bucket_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!(
                    "Failed to delete provider entitlement mappings bound to bucket: {}",
                    e
                ))
            })?;

        Ok(())
    }

    /// Overview matrix: per-bucket × credit-type aggregates (residual rows kept for
    /// disabled buckets) plus a SEPARATE grand total across all buckets.
    ///
    /// The per-bucket available balance is the derived
    /// SUM over `points_credit_ledger` using the SAME availability predicate as
    /// `compute_bucket_available_balances` in `points/postgres_repository.rs`
    /// (`status='active' AND remaining_amount>0 AND (effective_at IS NULL OR
    /// effective_at<=NOW()) AND (expires_at IS NULL OR expires_at>NOW())`),
    /// grouped by `(bucket_id, credit_type)`. This replaces the previous
    /// `LEFT JOIN points_wallets ... SUM(w.<x>_balance)` aggregation, which read
    /// Stored/GENERATED columns and would (a) leak future-effective pre-grant
    /// rows into the overview and (b) misjudge a bucket as in-use. If the
    /// predicate text drifts from the points repository, step 2 will catch
    /// it.
    pub async fn list_bucket_overview(
        &self,
        realm_id: &str,
    ) -> Result<CreditBucketOverview, CoreError> {
        use sqlx::Row;

        // Bucket metadata ordered for display.
        let bucket_rows = sqlx::query(
            "SELECT id AS bucket_id, name, enabled \
             FROM credit_buckets \
             WHERE realm_id = $1 \
             ORDER BY display_order ASC, created_at ASC",
        )
        .bind(realm_id)
        .fetch_all(self.db.get_postgres_connection_pool())
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to load bucket overview: {}", e)))?;

        // Derived per-(bucket_id, credit_type) available-balance aggregates using
        // the shared availability predicate (same text as
        // `compute_bucket_available_balances`).
        let agg_rows: Vec<(Uuid, String, i64)> = sqlx::query_as(
            "SELECT bucket_id, credit_type, COALESCE(SUM(remaining_amount), 0)::bigint AS available \
             FROM points_credit_ledger \
             WHERE realm_id = $1 \
               AND status = 'active' \
               AND remaining_amount > 0 \
               AND (effective_at IS NULL OR effective_at <= NOW()) \
               AND (expires_at IS NULL OR expires_at > NOW()) \
             GROUP BY bucket_id, credit_type",
        )
        .bind(realm_id)
        .fetch_all(self.db.get_postgres_connection_pool())
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to load bucket overview: {}", e)))?;

        // Index aggregates by bucket_id for O(1) lookup while walking buckets.
        let mut agg_by_bucket: std::collections::HashMap<
            Uuid,
            herald_domain::billing::credit_bucket::BucketByCreditType,
        > = std::collections::HashMap::new();
        for (bucket_id, credit_type, available) in agg_rows {
            let entry = agg_by_bucket.entry(bucket_id).or_default();
            match credit_type.as_str() {
                "topup_credit" => entry.topup = entry.topup.saturating_add(available),
                "subscription_credit" => {
                    entry.subscription = entry.subscription.saturating_add(available)
                }
                "granted_credit" => entry.granted = entry.granted.saturating_add(available),
                "registration_credit" => {
                    entry.registration = entry.registration.saturating_add(available)
                }
                "free_periodic_credit" => {
                    entry.free_periodic = entry.free_periodic.saturating_add(available)
                }
                other => {
                    return Err(CoreError::DatabaseError(format!(
                        "invalid credit_type in points_credit_ledger: {other}"
                    )));
                }
            }
        }

        let mut out_rows = Vec::with_capacity(bucket_rows.len());
        let mut grand_total_topup = 0i64;
        let mut grand_total_subscription = 0i64;
        let mut grand_total_registration = 0i64;
        let mut grand_total_free_periodic = 0i64;
        let mut grand_total_granted = 0i64;

        for row in &bucket_rows {
            let bucket_id: Uuid = row.get("bucket_id");
            let by_credit_type = agg_by_bucket.remove(&bucket_id).unwrap_or_default();

            grand_total_topup = grand_total_topup.saturating_add(by_credit_type.topup);
            grand_total_subscription =
                grand_total_subscription.saturating_add(by_credit_type.subscription);
            grand_total_registration =
                grand_total_registration.saturating_add(by_credit_type.registration);
            grand_total_free_periodic =
                grand_total_free_periodic.saturating_add(by_credit_type.free_periodic);
            grand_total_granted = grand_total_granted.saturating_add(by_credit_type.granted);

            out_rows.push(CreditBucketOverviewRow {
                bucket_id,
                name: row.get("name"),
                enabled: row.get("enabled"),
                bucket_total: by_credit_type.total(),
                by_credit_type,
            });
        }

        let grand_total = herald_domain::billing::credit_bucket::BucketByCreditType {
            topup: grand_total_topup,
            subscription: grand_total_subscription,
            registration: grand_total_registration,
            free_periodic: grand_total_free_periodic,
            granted: grand_total_granted,
        };

        Ok(CreditBucketOverview {
            rows: out_rows,
            grand_total,
        })
    }

    fn row_to_credit_bucket(row: &sqlx::postgres::PgRow) -> CreditBucket {
        use sqlx::Row;
        CreditBucket {
            id: row.get("id"),
            realm_id: row.get("realm_id"),
            bucket_key: row.get("bucket_key"),
            name: row.get("name"),
            description: row.get("description"),
            display_order: row.get("display_order"),
            receives_registration_credits: row.get("receives_registration_credits"),
            enabled: row.get("enabled"),
        }
    }

    /// Map an INSERT/UPDATE error to a structured bucket error.
    ///
    /// Distinguishes the two `credit_buckets` uniqueness violations by their
    /// constraint name:
    /// - partial unique index on `(realm_id) WHERE receives_registration_credits`
    ///   → `RegistrationPoolConflict` (409 `registration_pool_conflict`).
    /// - `UNIQUE(realm_id, bucket_key)` → `BucketKeyDuplicate`
    ///   (400 `bucket_key_duplicate`).
    ///
    /// Matching is intentionally robust to constraint-name drift: in production
    /// the migration assigns the explicit names `uq_credit_buckets_realm_key`
    /// and `uq_credit_buckets_registration_pool`, but a schema cloned via
    /// `CREATE TABLE ... (LIKE ... INCLUDING ALL)` (used by the test harness and
    /// by some pg_dump/restore flows) re-derives PostgreSQL's auto-generated
    /// names (`credit_buckets_realm_id_bucket_key_key`,
    /// `credit_buckets_realm_id_idx`). Both name sets are accepted, and the
    /// rendered message is inspected as a final fallback so the classification
    /// cannot silently degrade to a 500 if the driver omits `constraint()` or a
    /// future rename occurs.
    fn classify_bucket_insert_error(e: &sqlx::Error, realm_id: &str) -> CreditBucketError {
        let constraint = e.as_database_error().and_then(|db| db.constraint());
        let msg = e.to_string();

        match constraint
            .and_then(classify_bucket_constraint)
            .or_else(|| classify_from_message(&msg))
        {
            Some(BucketConstraintKind::RegistrationPool) => {
                CreditBucketError::RegistrationPoolConflict {
                    realm_id: realm_id.to_string(),
                }
            }
            Some(BucketConstraintKind::RealmKey) => CreditBucketError::BucketKeyDuplicate {
                realm_id: realm_id.to_string(),
            },
            None => CoreError::DatabaseError(msg).into(),
        }
    }

    /// Re-attach the listed mappings to `bucket_id` (realm-scoped). Mappings that do
    /// not exist or belong to a different realm are silently ignored — the handler
    /// is responsible for validating the requested set if strict semantics are
    /// required (currently we accept the best-effort attach).
    async fn reattach_mappings_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        bucket_id: Uuid,
        realm_id: &str,
        mapping_ids: &[Uuid],
    ) -> Result<(), CoreError> {
        for mapping_id in mapping_ids {
            sqlx::query(
                "UPDATE provider_entitlement_mappings \
                 SET bucket_id = $3, updated_at = NOW() \
                 WHERE realm_id = $1 AND id = $2",
            )
            .bind(realm_id)
            .bind(mapping_id)
            .bind(bucket_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to attach mapping to bucket: {}", e))
            })?;
        }
        Ok(())
    }
}

impl RegistrationPoolResolver for PostgresBillingRepository {
    /// Resolve the Realm's single registration-pool bucket.
    ///
    /// Relies on the partial unique index `uq_credit_buckets_registration_pool`
    /// (at most one row per realm with `receives_registration_credits=true`).
    /// Returns `Ok(None)` when no such bucket exists — callers must fail-safe
    /// (do not grant; do not fall back to an implicit pool).
    async fn resolve_registration_pool_bucket(
        &self,
        realm_id: &str,
    ) -> Result<Option<Uuid>, CoreError> {
        let bucket_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM credit_buckets \
             WHERE realm_id = $1 AND receives_registration_credits = true \
             LIMIT 1",
        )
        .bind(realm_id)
        .fetch_optional(self.db.get_postgres_connection_pool())
        .await
        .map_err(|e| {
            CoreError::DatabaseError(format!("Failed to resolve registration pool bucket: {}", e))
        })?;

        Ok(bucket_id)
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
            next_retry_at: Set(None),
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

    async fn cancel_subscriptions_by_external_id(
        &self,
        realm_id: &str,
        user_id: Uuid,
        external_subscription_id: &str,
    ) -> Result<u64, CoreError> {
        sqlx::query(
            "UPDATE subscription
             SET status = 'canceled', cancel_at = NOW(), updated_at = NOW()
             WHERE realm_id = $1 AND user_id = $2 AND external_subscription_id = $3
               AND status IN ('active', 'trialing', 'past_due', 'scheduled_cancel', 'pending')",
        )
        .bind(realm_id)
        .bind(user_id)
        .bind(external_subscription_id)
        .execute(self.db.get_postgres_connection_pool())
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))
        .map(|r| r.rows_affected())
    }

    async fn cancel_subscriptions_by_entitlement_key(
        &self,
        realm_id: &str,
        user_id: Uuid,
        entitlement_key: &str,
    ) -> Result<u64, CoreError> {
        sqlx::query(
            "UPDATE subscription
             SET status = 'canceled', cancel_at = NOW(), updated_at = NOW()
             WHERE realm_id = $1 AND user_id = $2 AND entitlement_key = $3
               AND status IN ('active', 'trialing', 'past_due', 'scheduled_cancel', 'pending')",
        )
        .bind(realm_id)
        .bind(user_id)
        .bind(entitlement_key)
        .execute(self.db.get_postgres_connection_pool())
        .await
        .map_err(|e| CoreError::DatabaseError(e.to_string()))
        .map(|r| r.rows_affected())
    }

    async fn list_active_subscriptions_by_user(
        &self,
        realm_id: &str,
        user_id: Uuid,
    ) -> Result<Vec<Subscription>, CoreError> {
        // In-effect statuses mirror SubscriptionStatus::has_access
        // (active / trialing / scheduled_cancel / dispute). The `status` column
        // stores the lowercase as_str text (see SubscriptionStatus::as_str), so
        // we filter on those exact text values.
        let results = subscription::Entity::find()
            .filter(subscription::Column::RealmId.eq(realm_id))
            .filter(subscription::Column::UserId.eq(user_id))
            .filter(subscription::Column::Status.is_in([
                "active",
                "trialing",
                "scheduled_cancel",
                "dispute",
            ]))
            .all(&self.db)
            .await?;

        results
            .into_iter()
            .map(Self::model_to_subscription)
            .collect()
    }

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
        active_model.bucket_id = update.bucket_id;
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
        // Try to find existing by the price-level unique constraint
        // (realm_id, payment_provider, external_product_id, external_price_id).
        // external_price_id IS NULL (Creem) dedups via NULLS NOT DISTINCT
        // (migration `20260607_product_reduce.sql`).
        let existing = self
            .find_entitlement_mapping_by_provider_product_price(
                &mapping.realm_id,
                &mapping.payment_provider,
                &mapping.external_product_id,
                mapping.external_price_id.as_deref(),
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

    async fn find_entitlement_mapping_by_provider_product_price(
        &self,
        realm_id: &str,
        payment_provider: &str,
        external_product_id: &str,
        external_price_id: Option<&str>,
    ) -> Result<Option<EntitlementMapping>, CoreError> {
        // NULL handling is encapsulated here, not at the call site.
        // Some(price_id) -> external_price_id = $x (Stripe)
        // None          -> external_price_id IS NULL (Creem; dedup via
        //                  NULLS NOT DISTINCT)
        let mut query = provider_entitlement_mapping::Entity::find()
            .filter(provider_entitlement_mapping::Column::RealmId.eq(realm_id))
            .filter(provider_entitlement_mapping::Column::PaymentProvider.eq(payment_provider))
            .filter(
                provider_entitlement_mapping::Column::ExternalProductId.eq(external_product_id),
            );
        query = match external_price_id {
            Some(pid) => {
                query.filter(provider_entitlement_mapping::Column::ExternalPriceId.eq(pid))
            }
            None => query.filter(provider_entitlement_mapping::Column::ExternalPriceId.is_null()),
        };

        let result = query
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

    async fn find_entitlement_mapping_by_key_price(
        &self,
        realm_id: &str,
        entitlement_key: &str,
        external_price_id: Option<&str>,
    ) -> Result<Option<EntitlementMapping>, CoreError> {
        // NULL handling encapsulated here (symmetric to
        // find_entitlement_mapping_by_provider_product_price).
        let mut query = provider_entitlement_mapping::Entity::find()
            .filter(provider_entitlement_mapping::Column::RealmId.eq(realm_id))
            .filter(provider_entitlement_mapping::Column::EntitlementKey.eq(entitlement_key));
        query = match external_price_id {
            Some(pid) => {
                query.filter(provider_entitlement_mapping::Column::ExternalPriceId.eq(pid))
            }
            None => query.filter(provider_entitlement_mapping::Column::ExternalPriceId.is_null()),
        };

        let result = query
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(result.map(Self::model_to_entitlement_mapping))
    }

    async fn list_entitlement_mappings_by_provider_product(
        &self,
        realm_id: &str,
        payment_provider: &str,
        external_product_id: &str,
    ) -> Result<Vec<EntitlementMapping>, CoreError> {
        let results = provider_entitlement_mapping::Entity::find()
            .filter(provider_entitlement_mapping::Column::RealmId.eq(realm_id))
            .filter(provider_entitlement_mapping::Column::PaymentProvider.eq(payment_provider))
            .filter(provider_entitlement_mapping::Column::ExternalProductId.eq(external_product_id))
            .all(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(Self::model_to_entitlement_mapping)
            .collect())
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

    async fn list_subscriptions(
        &self,
        realm_id: &str,
        entitlement_key: Option<&str>,
        status: Option<&str>,
        payment_provider: Option<&str>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<Subscription>, u64), CoreError> {
        let page = page.max(1);
        let offset = (page - 1) * page_size;

        // Build dynamic WHERE clause
        let mut conditions = vec!["realm_id = $1".to_string()];
        let mut param_idx = 2u32;

        let entitlement_key_param;
        let status_param;
        let payment_provider_param;

        if let Some(ek) = entitlement_key {
            conditions.push(format!("entitlement_key = ${}", param_idx));
            entitlement_key_param = Some(ek.to_string());
            param_idx += 1;
        } else {
            entitlement_key_param = None;
        }

        if let Some(s) = status {
            conditions.push(format!("status = ${}", param_idx));
            status_param = Some(s.to_string());
            param_idx += 1;
        } else {
            status_param = None;
        }

        if let Some(pp) = payment_provider {
            conditions.push(format!("payment_provider = ${}", param_idx));
            payment_provider_param = Some(pp.to_string());
            param_idx += 1;
        } else {
            payment_provider_param = None;
        }

        let where_clause = conditions.join(" AND ");
        let pool = self.db.get_postgres_connection_pool();

        // Count query
        let count_sql = format!("SELECT COUNT(*) FROM subscription WHERE {}", where_clause);
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql).bind(realm_id);
        if let Some(ref ek) = entitlement_key_param {
            count_query = count_query.bind(ek);
        }
        if let Some(ref s) = status_param {
            count_query = count_query.bind(s);
        }
        if let Some(ref pp) = payment_provider_param {
            count_query = count_query.bind(pp);
        }
        let total = count_query.fetch_one(pool).await.map_err(|e| {
            CoreError::DatabaseError(format!("Failed to count subscriptions: {}", e))
        })?;

        // Data query
        let data_sql = format!(
            "SELECT id, realm_id, user_id, external_subscription_id, external_product_id, \
             payment_provider, status, entitlement_key, billing_type, external_price_id, bucket_id, provider_metadata, \
             synced_at, current_period_start, current_period_end, cancel_at_period_end, \
             client_app_id, cancel_at, created_at, updated_at \
             FROM subscription WHERE {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
            where_clause,
            param_idx,
            param_idx + 1
        );
        let mut data_query = sqlx::query(&data_sql).bind(realm_id);
        if let Some(ref ek) = entitlement_key_param {
            data_query = data_query.bind(ek);
        }
        if let Some(ref s) = status_param {
            data_query = data_query.bind(s);
        }
        if let Some(ref pp) = payment_provider_param {
            data_query = data_query.bind(pp);
        }
        data_query = data_query.bind(page_size as i64).bind(offset as i64);

        let rows = data_query.fetch_all(pool).await.map_err(|e| {
            CoreError::DatabaseError(format!("Failed to list subscriptions: {}", e))
        })?;

        let subs: Vec<Subscription> = rows
            .iter()
            .map(|row| {
                use sqlx::Row;
                let status_str: String = row.get("status");
                Ok(Subscription {
                    id: row.get("id"),
                    realm_id: row.get("realm_id"),
                    user_id: row.get("user_id"),
                    external_subscription_id: row.get("external_subscription_id"),
                    external_product_id: row.get("external_product_id"),
                    payment_provider: row.get("payment_provider"),
                    status: status_str.parse()?,
                    entitlement_key: row.get("entitlement_key"),
                    billing_type: {
                        let bt: String = row.get("billing_type");
                        bt.parse()?
                    },
                    external_price_id: row.get("external_price_id"),
                    bucket_id: row.get("bucket_id"),
                    provider_metadata: row.get("provider_metadata"),
                    synced_at: row.get("synced_at"),
                    current_period_start: row.get("current_period_start"),
                    current_period_end: row.get("current_period_end"),
                    cancel_at_period_end: row.get("cancel_at_period_end"),
                    client_app_id: row.get("client_app_id"),
                    cancel_at: row.get("cancel_at"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                })
            })
            .collect::<Result<Vec<_>, CoreError>>()?;

        Ok((subs, total as u64))
    }

    async fn check_feature_facts(
        &self,
        realm_id: &str,
        pool: &sqlx::PgPool,
    ) -> Result<FeatureFacts, CoreError> {
        use sqlx::Row;

        let row = sqlx::query(
            r#"
            WITH configured_providers AS (
                SELECT 'stripe'
                WHERE EXISTS (
                    SELECT 1 FROM realm_config
                    WHERE realm_id = $1 AND config_type = 'stripe'
                      AND config_key = 'api_key' AND enabled = true
                )
                UNION ALL
                SELECT 'creem'
                WHERE EXISTS (
                    SELECT 1 FROM realm_config
                    WHERE realm_id = $1 AND config_type = 'creem'
                      AND config_key = 'api_key' AND enabled = true
                )
            )
            SELECT
                EXISTS (SELECT 1 FROM configured_providers) AS has_payment_providers,
                EXISTS (SELECT 1 FROM provider_entitlement_mappings WHERE realm_id = $1) AS has_entitlement_mappings,
                EXISTS (SELECT 1 FROM provider_entitlement_mappings WHERE realm_id = $1 AND enabled = true) AS has_enabled_mappings,
                EXISTS (SELECT 1 FROM provider_entitlement_mappings WHERE realm_id = $1 AND billing_type = 'one_time' AND enabled = true) AS has_one_time_mappings,
                EXISTS (SELECT 1 FROM provider_entitlement_mappings WHERE realm_id = $1 AND billing_type = 'recurring' AND enabled = true) AS has_recurring_mappings,
                EXISTS (SELECT 1 FROM invoice_seller_config WHERE realm_id = $1) AS has_invoice_seller_config,
                EXISTS (SELECT 1 FROM invoice WHERE realm_id = $1) AS has_invoices,
                EXISTS (SELECT 1 FROM subscription_history WHERE realm_id = $1) AS has_subscription_history
            "#,
        )
        .bind(realm_id)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            CoreError::DatabaseError(format!("Failed to load feature availability facts: {}", e))
        })?;

        Ok(FeatureFacts {
            has_payment_providers: row.get("has_payment_providers"),
            has_entitlement_mappings: row.get("has_entitlement_mappings"),
            has_enabled_mappings: row.get("has_enabled_mappings"),
            has_one_time_mappings: row.get("has_one_time_mappings"),
            has_recurring_mappings: row.get("has_recurring_mappings"),
            has_invoice_seller_config: row.get("has_invoice_seller_config"),
            has_invoices: row.get("has_invoices"),
            has_subscription_history: row.get("has_subscription_history"),
        })
    }

    /// Atomically batch-upsert all price rows for one product.
    ///
    /// Pipeline within a single sqlx transaction:
    /// 1. `SELECT ... FOR UPDATE` the mappings named in `updates` (lock the
    ///    group) and verify every `mapping_id` belongs to the
    ///    `(realm, provider, product)` group — else `MappingNotInGroup` (400).
    /// 2. Active-subscription lock: for every row transitioning
    ///    `enabled` true→false, count access-granting subscriptions anchored to
    ///    that mapping's `(realm, provider, product, external_price_id)`. Any >0
    ///    → roll back the whole tx and return `ActiveSubscriptionLock` (409).
    /// 3. Upsert each row (UPDATE existing in place; the batch is scoped to
    ///    already-synced price rows, so no INSERT path is exercised here).
    /// 4. Re-read the product's full latest price-row set and return it.
    async fn batch_update_mappings(
        &self,
        input: BatchUpdateMappingsInput,
    ) -> Result<BatchUpdateResult, BatchMappingError> {
        use sqlx::Row;

        if input.updates.is_empty() {
            // Nothing to write — return the current full set for the product.
            let prices = self
                .list_entitlement_mappings_by_provider_product(
                    &input.realm_id,
                    &input.payment_provider,
                    &input.external_product_id,
                )
                .await?;
            return Ok(BatchUpdateResult { saved: 0, prices });
        }

        let mut tx = self
            .db
            .get_postgres_connection_pool()
            .begin()
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to begin batch transaction: {}", e))
            })?;

        // 1. Lock + ownership check for every requested mapping_id.
        let requested_ids: Vec<Uuid> = input.updates.iter().map(|u| u.mapping_id).collect();
        let rows = sqlx::query(
            "SELECT id, external_price_id, enabled \
             FROM provider_entitlement_mappings \
             WHERE realm_id = $1 AND payment_provider = $2 AND external_product_id = $3 \
               AND id = ANY($4) FOR UPDATE",
        )
        .bind(&input.realm_id)
        .bind(&input.payment_provider)
        .bind(&input.external_product_id)
        .bind(&requested_ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| CoreError::DatabaseError(format!("Failed to lock batch mappings: {}", e)))?;

        if rows.len() != input.updates.len() {
            // Identify the first offender for a precise error.
            let found: std::collections::HashSet<Uuid> =
                rows.iter().map(|r| r.get::<Uuid, _>("id")).collect();
            let offender = input
                .updates
                .iter()
                .map(|u| u.mapping_id)
                .find(|id| !found.contains(id))
                .unwrap_or_else(|| input.updates[0].mapping_id);
            let _ = tx.rollback().await;
            return Err(BatchMappingError::MappingNotInGroup {
                mapping_id: offender,
                provider: input.payment_provider.clone(),
                product: input.external_product_id.clone(),
            });
        }

        // Index current state by id for diffing.
        let mut current_by_id: std::collections::HashMap<Uuid, (Option<String>, bool)> =
            std::collections::HashMap::with_capacity(input.updates.len());
        for r in rows {
            current_by_id.insert(
                r.get::<Uuid, _>("id"),
                (
                    r.get::<Option<String>, _>("external_price_id"),
                    r.get::<bool, _>("enabled"),
                ),
            );
        }

        // 2. Active-subscription lock: sum access-granting subscriptions across
        // every row that transitions enabled true→false. Any >0 rolls back the
        // WHOLE batch.
        // Collect both the non-null price ids (Stripe) and whether any
        // disabling row is price-less (Creem, NULL external_price_id). The
        // count query ORs the two matching branches so NULL-price subscriptions
        // are covered without a sentinel.
        let mut disabling_price_ids: Vec<String> = Vec::new();
        let mut disabling_includes_null_price = false;
        for u in &input.updates {
            let Some(false) = u.enabled else {
                continue;
            };
            let Some((price_id, was_enabled)) = current_by_id.get(&u.mapping_id) else {
                continue;
            };
            if *was_enabled {
                match price_id {
                    Some(pid) => disabling_price_ids.push(pid.clone()),
                    None => disabling_includes_null_price = true,
                }
            }
        }
        if !disabling_price_ids.is_empty() || disabling_includes_null_price {
            // Access-granting statuses mirror SubscriptionStatus::has_access().
            // Branch 1: subscriptions whose external_price_id is one of the
            // disabling non-null price ids. Branch 2 (only when a disabling row
            // is price-less): subscriptions with NULL external_price_id.
            let active_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM subscription \
                 WHERE realm_id = $1 \
                   AND payment_provider = $2 \
                   AND external_product_id = $3 \
                   AND status IN ('active','trialing','past_due','scheduled_cancel','dispute') \
                   AND ( \
                        external_price_id = ANY($4) \
                     OR ($5 AND external_price_id IS NULL) \
                   )",
            )
            .bind(&input.realm_id)
            .bind(&input.payment_provider)
            .bind(&input.external_product_id)
            .bind(&disabling_price_ids)
            .bind(disabling_includes_null_price)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!(
                    "Failed to count active subscriptions for batch: {}",
                    e
                ))
            })?;
            if active_count > 0 {
                let _ = tx.rollback().await;
                return Err(BatchMappingError::ActiveSubscriptionLock {
                    provider: input.payment_provider.clone(),
                    product: input.external_product_id.clone(),
                    active_subscriptions: active_count,
                });
            }
        }

        // 3. Upsert (UPDATE) each row in tx order. Fields the client omits
        // (`None`) are preserved via COALESCE — matches the single-PATCH contract
        // (entitlement_mapping_handlers.rs `update_entitlement_mapping`).
        // `quota_windows` is the exception: it is NOT COALESCE'd because the
        // caller must be able to CLEAR the column (write NULL) by passing
        // `Some([])`. So when the input carries `Some(windows)`, the column is
        // SET explicitly (serialized: empty ⟹ NULL, non-empty ⟹ JSONB); when
        // the input is `None`, the column is omitted from SET (leave unchanged).
        // The SQL string is therefore built per-row to include the
        // `quota_windows` clause only when the input provides it.
        let now = chrono::Utc::now();
        let mut saved: u32 = 0;
        for u in &input.updates {
            let billing_type_str = u.billing_type.as_deref();
            // Serialize `quota_windows` to the JSONB column shape. `None` ⟺ leave
            // unchanged (column omitted from SET). `Some` ⟺ SET explicitly so the
            // caller can clear to NULL via `Some([])`.
            let quota_windows_value: Option<Option<serde_json::Value>> = match &u.quota_windows {
                None => None,
                Some(windows) => {
                    let domain_windows: Vec<QuotaWindow> = windows
                        .iter()
                        .map(|w| QuotaWindow {
                            window_seconds: w.window_seconds,
                            limit: w.limit,
                            key: herald_domain::points::derive_window_key(w.window_seconds),
                        })
                        .collect();
                    let serialized = serialize_quota_windows_value(&domain_windows);
                    match serialized {
                        Ok(v) => Some(v),
                        Err(e) => {
                            let _ = tx.rollback().await;
                            return Err(BatchMappingError::Other(e));
                        }
                    }
                }
            };
            // `granted_role_ids` is a `UUID[]` column (NOT COALESCE'd, same reason
            // as `quota_windows`: the caller must be able to CLEAR to `{}` by
            // passing `Some([])`). `None` ⟺ leave unchanged (column omitted from
            // SET). sqlx encodes `Vec<Uuid>` → `uuid[]` (matches the account
            // `provider_ids` path). Clone to owned for the bind.
            let granted_role_ids_value: Option<Vec<Uuid>> =
                u.granted_role_ids.as_ref().map(|ids| ids.to_vec());

            // Build the UPDATE per-row with sqlx::QueryBuilder so placeholder
            // numbering is automatic and consistent with the clauses actually
            // emitted. COALESCE is used for fields the caller can leave unchanged
            // (`None` ⟺ preserve the DB value); the two optional-array columns
            // (`quota_windows`, `granted_role_ids`) are SET explicitly only when
            // the input provides them, so the caller can CLEAR them. `push_bind`
            // interleaves values in the exact order their `$n` placeholders appear
            // in the SQL text — SET clause first, then the WHERE anchors.
            let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
                "UPDATE provider_entitlement_mappings SET billing_type = COALESCE(",
            );
            qb.push_bind(billing_type_str.map(|s| s.to_string()));
            qb.push(", billing_type), points_per_period = COALESCE(");
            qb.push_bind(u.points_per_period.map(|v| v as i32));
            qb.push(", points_per_period), validity_days = COALESCE(");
            qb.push_bind(u.validity_days.map(|v| v as i32));
            qb.push(", validity_days), grant_on_subscribe = COALESCE(");
            qb.push_bind(u.grant_on_subscribe);
            qb.push(", grant_on_subscribe), enabled = COALESCE(");
            qb.push_bind(u.enabled);
            qb.push(", enabled)");
            if let Some(qw) = quota_windows_value {
                qb.push(", quota_windows = ");
                qb.push_bind(qw);
            }
            if let Some(role_ids) = granted_role_ids_value {
                qb.push(", granted_role_ids = ");
                qb.push_bind(role_ids);
            }
            qb.push(", updated_at = ");
            qb.push_bind(now);
            qb.push(" WHERE realm_id = ");
            qb.push_bind(input.realm_id.clone());
            qb.push(" AND payment_provider = ");
            qb.push_bind(input.payment_provider.clone());
            qb.push(" AND external_product_id = ");
            qb.push_bind(input.external_product_id.clone());
            qb.push(" AND id = ");
            qb.push_bind(u.mapping_id);

            let result = qb.build().execute(&mut *tx).await.map_err(|e| {
                CoreError::DatabaseError(format!("Failed to update mapping in batch: {}", e))
            })?;
            saved += result.rows_affected() as u32;
        }

        tx.commit().await.map_err(|e| {
            CoreError::DatabaseError(format!("Failed to commit batch_update_mappings: {}", e))
        })?;

        // 5. Re-read the product's full latest price-row set.
        let prices = self
            .list_entitlement_mappings_by_provider_product(
                &input.realm_id,
                &input.payment_provider,
                &input.external_product_id,
            )
            .await?;
        Ok(BatchUpdateResult { saved, prices })
    }
}

/// Which `credit_buckets` uniqueness violation a Postgres error refers to.
///
/// See `classify_bucket_insert_error` for the full rationale; this enum is the
/// pure (constraint-name → kind) projection so it can be unit-tested without a
/// live database.
enum BucketConstraintKind {
    /// `UNIQUE(realm_id, bucket_key)` collision → 400 `bucket_key_duplicate`.
    RealmKey,
    /// Partial unique index on `(realm_id) WHERE receives_registration_credits`
    /// collision → 409 `registration_pool_conflict`.
    RegistrationPool,
}

/// Map a Postgres constraint/index name to its bucket-error kind.
///
/// Accepts both the migration-assigned explicit names (`uq_credit_buckets_*`)
/// and PostgreSQL's auto-generated names (`credit_buckets_*`) produced when the
/// schema is cloned via `CREATE TABLE ... (LIKE ... INCLUDING ALL)` or restored
/// by pg_dump without preserving constraint names. This keeps
/// `classify_bucket_insert_error` stable across schema-name drift.
fn classify_bucket_constraint(constraint: &str) -> Option<BucketConstraintKind> {
    // Migration-assigned explicit names.
    match constraint {
        "uq_credit_buckets_realm_key" => return Some(BucketConstraintKind::RealmKey),
        "uq_credit_buckets_registration_pool" => {
            return Some(BucketConstraintKind::RegistrationPool);
        }
        _ => {}
    }
    // PostgreSQL auto-generated names. `<table>_<cols>_key` is the default for
    // an unnamed UNIQUE constraint; `<table>_<col>_idx` is the default for an
    // unnamed index. `credit_buckets_realm_id_idx` is the partial-unique index
    // in the cloned test schema (a plain non-unique index never raises a
    // unique-violation, so matching it here is safe).
    match constraint {
        "credit_buckets_realm_id_bucket_key_key" => Some(BucketConstraintKind::RealmKey),
        "credit_buckets_realm_id_idx" => Some(BucketConstraintKind::RegistrationPool),
        _ => None,
    }
}

/// Last-resort classification from the rendered Postgres error message.
///
/// Used when the driver omits `constraint()` (older sqlx/PG combos) or when an
/// unforeseen constraint name appears. Matches both name families quoted inside
/// the standard `duplicate key value violates unique constraint "<name>"` text.
fn classify_from_message(msg: &str) -> Option<BucketConstraintKind> {
    let is_dup = msg.contains("duplicate key value violates unique constraint")
        || msg.contains("duplicate key value");
    if !is_dup {
        return None;
    }
    if msg.contains("uq_credit_buckets_registration_pool")
        || msg.contains("credit_buckets_realm_id_idx")
        || msg.contains("receives_registration_credits")
    {
        return Some(BucketConstraintKind::RegistrationPool);
    }
    if msg.contains("uq_credit_buckets_realm_key")
        || msg.contains("credit_buckets_realm_id_bucket_key")
    {
        return Some(BucketConstraintKind::RealmKey);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard for P0-1: `credit_buckets` uniqueness violations
    /// must classify into `BucketKeyDuplicate` (→ 400) or `RegistrationPoolConflict`
    /// (→ 409) regardless of whether the runtime schema carries the
    /// migration-assigned explicit constraint names or PostgreSQL's
    /// auto-generated names (the latter appear when the schema is cloned via
    /// `CREATE TABLE ... (LIKE ... INCLUDING ALL)`, e.g. the test harness, and
    /// historically caused a silent 500 regression).
    #[test]
    fn classifies_bucket_constraint_names_for_both_name_families() {
        // Migration-assigned explicit names (production schema).
        let explicit = classify_bucket_constraint("uq_credit_buckets_realm_key");
        assert!(
            matches!(explicit, Some(BucketConstraintKind::RealmKey)),
            "explicit realm_key name must classify as RealmKey"
        );
        let explicit_reg = classify_bucket_constraint("uq_credit_buckets_registration_pool");
        assert!(
            matches!(explicit_reg, Some(BucketConstraintKind::RegistrationPool)),
            "explicit registration_pool name must classify as RegistrationPool"
        );

        // PostgreSQL auto-generated names (cloned/restored schema — the actual
        // runtime names observed in the failing scenarios).
        let auto = classify_bucket_constraint("credit_buckets_realm_id_bucket_key_key");
        assert!(
            matches!(auto, Some(BucketConstraintKind::RealmKey)),
            "auto-named realm+bucket_key unique must classify as RealmKey"
        );
        let auto_reg = classify_bucket_constraint("credit_buckets_realm_id_idx");
        assert!(
            matches!(auto_reg, Some(BucketConstraintKind::RegistrationPool)),
            "auto-named partial unique index must classify as RegistrationPool"
        );

        // Unrelated names must NOT be force-classified (they fall through to a
        // generic DB error → 500), so this guard also catches over-eager
        // matchers that would mask unrelated failures.
        assert!(
            classify_bucket_constraint("credit_buckets_pkey").is_none(),
            "primary-key violations must not be misclassified"
        );
    }

    /// The message-based fallback must catch the same two collisions when the
    /// driver omits `constraint()` (older sqlx/PG combos) — using the real
    /// runtime error text observed in the regression.
    #[test]
    fn classifies_from_runtime_duplicate_key_messages() {
        let realm_key_msg = "error returned from database: duplicate key value \
             violates unique constraint \"credit_buckets_realm_id_bucket_key_key\"";
        assert!(matches!(
            classify_from_message(realm_key_msg),
            Some(BucketConstraintKind::RealmKey)
        ));

        let reg_pool_msg = "error returned from database: duplicate key value \
             violates unique constraint \"credit_buckets_realm_id_idx\"";
        assert!(matches!(
            classify_from_message(reg_pool_msg),
            Some(BucketConstraintKind::RegistrationPool)
        ));

        // Explicit names in the message are also covered.
        let explicit_msg = "duplicate key value violates unique constraint \
             \"uq_credit_buckets_registration_pool\"";
        assert!(matches!(
            classify_from_message(explicit_msg),
            Some(BucketConstraintKind::RegistrationPool)
        ));

        // A non-duplicate error must not be classified.
        assert!(
            classify_from_message("relation does not exist").is_none(),
            "non-duplicate errors must not be force-classified"
        );
    }
}
