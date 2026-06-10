use chrono::{DateTime, Utc};
use sea_orm::DatabaseTransaction;
use serde_json::Value;
use uuid::Uuid;

use herald_api_base::application::http::state::AppState;
use herald_core::domain::billing::{
    ACTOR_WEBHOOK, BillingRepository, HistoryEventType, Subscription, SubscriptionHistoryEvent,
    calculate_changes, serialize_subscription_state,
};
use herald_core::domain::common::entities::app_errors::CoreError;
use herald_core::infrastructure::billing::PostgresBillingRepository;

pub(crate) struct SyncSubscriptionInput {
    pub provider: &'static str,
    pub realm_id: String,
    pub user_id: Option<Uuid>,
    pub external_subscription_id: String,
    pub external_product_id: String,
    pub client_app_id: Option<Uuid>,
    pub entitlement_key: String,
    pub external_price_id: Option<String>,
    pub provider_metadata: Option<Value>,
    pub status: herald_core::domain::billing::SubscriptionStatus,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub cancel_at: Option<DateTime<Utc>>,
    /// Pre-fetched subscription to avoid a redundant DB lookup.
    /// When provided, `sync_subscription` skips the `find_by_external_subscription_id` query.
    pub existing_subscription: Option<Subscription>,
}

pub(crate) async fn save_subscription_history(
    app_state: &AppState,
    previous: Option<&Subscription>,
    current: &Subscription,
    event_type: HistoryEventType,
) -> Result<(), CoreError> {
    let history_event = SubscriptionHistoryEvent {
        id: Uuid::now_v7().to_string(),
        subscription_id: current.id,
        event_type,
        timestamp: Utc::now(),
        actor: Some(ACTOR_WEBHOOK.to_string()),
        changes: previous.map(|previous| calculate_changes(previous, current)),
        previous_state: previous.map(serialize_subscription_state),
        new_state: Some(serialize_subscription_state(current)),
        realm_id: current.realm_id.clone(),
        created_at: Utc::now(),
    };

    app_state
        .billing_repository
        .save_history_event(history_event)
        .await?;

    Ok(())
}

pub(crate) async fn save_subscription_history_in_txn(
    billing_repo: &PostgresBillingRepository,
    txn: &DatabaseTransaction,
    previous: Option<&Subscription>,
    current: &Subscription,
    event_type: HistoryEventType,
) -> Result<(), CoreError> {
    let history_event = SubscriptionHistoryEvent {
        id: Uuid::now_v7().to_string(),
        subscription_id: current.id,
        event_type,
        timestamp: Utc::now(),
        actor: Some(ACTOR_WEBHOOK.to_string()),
        changes: previous.map(|previous| calculate_changes(previous, current)),
        previous_state: previous.map(serialize_subscription_state),
        new_state: Some(serialize_subscription_state(current)),
        realm_id: current.realm_id.clone(),
        created_at: Utc::now(),
    };

    let _ = billing_repo;
    PostgresBillingRepository::save_history_event_conn(txn, history_event).await?;

    Ok(())
}

/// Resolve entitlement_key using fallback chain:
/// 1. Use provided herald_entitlement_key from metadata (if non-empty)
/// 2. Look up local mapping by provider + external_product_id
/// 3. Fail loud with diagnostic log
pub(crate) async fn resolve_entitlement_key(
    app_state: &AppState,
    realm_id: &str,
    provider: &str,
    external_product_id: &str,
    metadata_entitlement_key: Option<&str>,
) -> Result<String, CoreError> {
    // Step 1: Use metadata entitlement_key if present and non-empty
    if let Some(key) = metadata_entitlement_key
        && !key.is_empty()
    {
        tracing::info!(
            realm_id = %realm_id,
            provider = %provider,
            entitlement_key = %key,
            "Resolved entitlement_key from webhook metadata"
        );
        return Ok(key.to_string());
    }

    // Step 2: Fallback to local mapping by provider + external_product_id
    let mapping = app_state
        .billing_repository
        .find_entitlement_mapping_by_provider_product(realm_id, provider, external_product_id)
        .await?;

    if let Some(mapping) = mapping {
        tracing::info!(
            realm_id = %realm_id,
            provider = %provider,
            external_product_id = %external_product_id,
            entitlement_key = %mapping.entitlement_key,
            "Resolved entitlement_key from local mapping fallback"
        );
        return Ok(mapping.entitlement_key);
    }

    // Step 3: Fail loud
    tracing::error!(
        realm_id = %realm_id,
        provider = %provider,
        external_product_id = %external_product_id,
        "Cannot resolve entitlement_key: no herald_entitlement_key in metadata and no local mapping found"
    );
    Err(CoreError::BadRequest(format!(
        "Cannot resolve entitlement_key for provider={} external_product_id={}: no metadata key and no local mapping",
        provider, external_product_id
    )))
}

pub(crate) async fn sync_subscription(
    app_state: &AppState,
    input: SyncSubscriptionInput,
) -> Result<Option<(Subscription, Option<Subscription>)>, CoreError> {
    let SyncSubscriptionInput {
        provider,
        realm_id,
        user_id,
        external_subscription_id,
        external_product_id,
        client_app_id,
        entitlement_key,
        external_price_id,
        provider_metadata,
        status,
        current_period_start,
        current_period_end,
        cancel_at_period_end,
        cancel_at,
        existing_subscription,
    } = input;

    let existing = {
        if let Some(prefetched) = existing_subscription {
            // Use the pre-fetched subscription to avoid a redundant DB query
            Some(prefetched)
        } else {
            let existing = app_state
                .billing_repository
                .find_by_external_subscription_id(&external_subscription_id, provider)
                .await?;

            if existing.is_some() {
                existing
            } else if let Some(client_app_id) = client_app_id {
                app_state
                    .billing_repository
                    .find_subscription_by_client_app_id(client_app_id)
                    .await?
            } else {
                None
            }
        }
    };

    if existing.is_none() && client_app_id.is_none() && external_subscription_id.is_empty() {
        return Ok(None);
    }

    if external_subscription_id.is_empty() {
        return Err(CoreError::BadRequest(
            "Missing external_subscription_id".to_string(),
        ));
    }

    let now = Utc::now();

    if let Some(mut subscription) = existing {
        let previous = subscription.clone();

        subscription.external_subscription_id = external_subscription_id.clone();
        subscription.external_product_id = external_product_id.clone();
        subscription.payment_provider = provider.to_string();
        subscription.status = status;
        subscription.entitlement_key = if entitlement_key.is_empty() {
            subscription.entitlement_key.clone()
        } else {
            entitlement_key
        };
        subscription.external_price_id = external_price_id.or(subscription.external_price_id);
        subscription.provider_metadata = provider_metadata.or(subscription.provider_metadata);
        subscription.synced_at = Some(now);
        if let Some(user_id) = user_id {
            subscription.user_id = Some(user_id);
        }
        subscription.current_period_start = current_period_start.or(previous.current_period_start);
        subscription.current_period_end = current_period_end.or(previous.current_period_end);
        subscription.cancel_at_period_end = cancel_at_period_end;
        subscription.cancel_at = cancel_at;
        if client_app_id.is_some() {
            subscription.client_app_id = client_app_id;
        }
        subscription.updated_at = now;

        let updated = app_state
            .billing_repository
            .update_subscription(subscription)
            .await?;
        Ok(Some((updated, Some(previous))))
    } else {
        let subscription = Subscription {
            id: Uuid::now_v7(),
            realm_id,
            user_id,
            external_subscription_id,
            external_product_id,
            payment_provider: provider.to_string(),
            status,
            entitlement_key,
            external_price_id,
            provider_metadata,
            synced_at: Some(now),
            current_period_start,
            current_period_end,
            cancel_at_period_end,
            client_app_id,
            cancel_at,
            created_at: now,
            updated_at: now,
        };

        let created = app_state
            .billing_repository
            .create_subscription(subscription)
            .await?;
        Ok(Some((created, None)))
    }
}

pub(crate) async fn sync_subscription_in_txn(
    txn: &DatabaseTransaction,
    input: SyncSubscriptionInput,
) -> Result<Option<(Subscription, Option<Subscription>)>, CoreError> {
    let SyncSubscriptionInput {
        provider,
        realm_id,
        user_id,
        external_subscription_id,
        external_product_id,
        client_app_id,
        entitlement_key,
        external_price_id,
        provider_metadata,
        status,
        current_period_start,
        current_period_end,
        cancel_at_period_end,
        cancel_at,
        existing_subscription,
    } = input;

    let existing = {
        if let Some(prefetched) = existing_subscription {
            Some(prefetched)
        } else {
            let existing = PostgresBillingRepository::find_by_external_subscription_id_conn(
                txn,
                &external_subscription_id,
                provider,
            )
            .await?;

            if existing.is_some() {
                existing
            } else if let Some(client_app_id) = client_app_id {
                PostgresBillingRepository::find_subscription_by_client_app_id_conn(
                    txn,
                    client_app_id,
                )
                .await?
            } else {
                None
            }
        }
    };

    if existing.is_none() && client_app_id.is_none() && external_subscription_id.is_empty() {
        return Ok(None);
    }

    if external_subscription_id.is_empty() {
        return Err(CoreError::BadRequest(
            "Missing external_subscription_id".to_string(),
        ));
    }

    let now = Utc::now();

    if let Some(mut subscription) = existing {
        let previous = subscription.clone();

        subscription.external_subscription_id = external_subscription_id.clone();
        subscription.external_product_id = external_product_id.clone();
        subscription.payment_provider = provider.to_string();
        subscription.status = status;
        subscription.entitlement_key = if entitlement_key.is_empty() {
            subscription.entitlement_key.clone()
        } else {
            entitlement_key
        };
        subscription.external_price_id = external_price_id.or(subscription.external_price_id);
        subscription.provider_metadata = provider_metadata.or(subscription.provider_metadata);
        subscription.synced_at = Some(now);
        if let Some(user_id) = user_id {
            subscription.user_id = Some(user_id);
        }
        subscription.current_period_start = current_period_start.or(previous.current_period_start);
        subscription.current_period_end = current_period_end.or(previous.current_period_end);
        subscription.cancel_at_period_end = cancel_at_period_end;
        subscription.cancel_at = cancel_at;
        if client_app_id.is_some() {
            subscription.client_app_id = client_app_id;
        }
        subscription.updated_at = now;

        let updated =
            PostgresBillingRepository::update_subscription_conn(txn, subscription).await?;
        Ok(Some((updated, Some(previous))))
    } else {
        let subscription = Subscription {
            id: Uuid::now_v7(),
            realm_id,
            user_id,
            external_subscription_id,
            external_product_id,
            payment_provider: provider.to_string(),
            status,
            entitlement_key,
            external_price_id,
            provider_metadata,
            synced_at: Some(now),
            current_period_start,
            current_period_end,
            cancel_at_period_end,
            client_app_id,
            cancel_at,
            created_at: now,
            updated_at: now,
        };

        let created =
            PostgresBillingRepository::create_subscription_conn(txn, subscription).await?;
        Ok(Some((created, None)))
    }
}
