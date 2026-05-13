use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::handlers::determine_tier_from_product;
use herald_api_base::application::http::state::AppState;
use herald_core::domain::billing::{
    ACTOR_WEBHOOK, BillingPeriod, BillingRepository, HistoryEventType, Subscription,
    SubscriptionHistoryEvent, SubscriptionTier, calculate_changes, serialize_subscription_state,
};
use herald_core::domain::common::entities::app_errors::CoreError;

pub(crate) struct SyncSubscriptionInput {
    pub provider: &'static str,
    pub realm_id: String,
    pub user_id: Option<Uuid>,
    pub external_subscription_id: String,
    pub external_product_id: String,
    pub client_app_id: Option<Uuid>,
    pub plan_id: Option<Uuid>,
    pub status: herald_core::domain::billing::SubscriptionStatus,
    pub billing_period: BillingPeriod,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub cancel_at: Option<DateTime<Utc>>,
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
        plan_id,
        status,
        billing_period,
        current_period_start,
        current_period_end,
        cancel_at_period_end,
        cancel_at,
    } = input;

    let existing = {
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
    };

    if existing.is_none() && client_app_id.is_none() && external_subscription_id.is_empty() {
        return Ok(None);
    }

    if external_subscription_id.is_empty() {
        return Err(CoreError::BadRequest(
            "Missing external_subscription_id".to_string(),
        ));
    }

    let tier = if external_product_id.is_empty() {
        SubscriptionTier::Starter
    } else {
        determine_tier_from_product(&external_product_id)
    };
    let now = Utc::now();

    if let Some(mut subscription) = existing {
        let previous = subscription.clone();

        subscription.external_subscription_id = external_subscription_id.clone();
        subscription.external_product_id = external_product_id.clone();
        subscription.payment_provider = provider.to_string();
        subscription.status = status;
        subscription.tier = tier;
        if let Some(user_id) = user_id {
            subscription.user_id = Some(user_id);
        }
        subscription.current_period_start = current_period_start.or(previous.current_period_start);
        subscription.current_period_end = current_period_end.or(previous.current_period_end);
        subscription.cancel_at_period_end = cancel_at_period_end;
        subscription.cancel_at = cancel_at;
        if let Some(plan_id) = plan_id {
            subscription.plan_id = Some(plan_id);
        }
        subscription.billing_period = billing_period;
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
            tier,
            current_period_start,
            current_period_end,
            cancel_at_period_end,
            client_app_id,
            plan_id,
            billing_period,
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
