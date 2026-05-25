use chrono::Utc;
use uuid::Uuid;

use crate::billing::{
    HistoryEventType, Subscription, SubscriptionHistoryEvent, calculate_changes,
    detect_change_type, serialize_subscription_state,
};

/// Subscription history service for tracking subscription changes
///
/// This service provides methods to create history events when subscriptions
/// are created, updated, or canceled through webhooks or API calls.
pub struct SubscriptionHistoryService;

impl SubscriptionHistoryService {
    /// Create a history event for a new subscription
    pub fn create_subscription_created_event(
        subscription: &Subscription,
        actor: Option<String>,
    ) -> SubscriptionHistoryEvent {
        SubscriptionHistoryEvent {
            id: Uuid::now_v7().to_string(),
            subscription_id: subscription.id,
            event_type: HistoryEventType::Created,
            timestamp: Utc::now(),
            actor,
            changes: None,
            previous_state: None,
            new_state: Some(serialize_subscription_state(subscription)),
            realm_id: subscription.realm_id.clone(),
            created_at: Utc::now(),
        }
    }

    /// Create a history event for a subscription update
    pub fn create_subscription_updated_event(
        old_subscription: &Subscription,
        new_subscription: &Subscription,
        actor: Option<String>,
    ) -> SubscriptionHistoryEvent {
        let event_type = detect_change_type(old_subscription, new_subscription);
        let changes = calculate_changes(old_subscription, new_subscription);

        SubscriptionHistoryEvent {
            id: Uuid::now_v7().to_string(),
            subscription_id: new_subscription.id,
            event_type,
            timestamp: Utc::now(),
            actor,
            changes: Some(changes),
            previous_state: Some(serialize_subscription_state(old_subscription)),
            new_state: Some(serialize_subscription_state(new_subscription)),
            realm_id: new_subscription.realm_id.clone(),
            created_at: Utc::now(),
        }
    }

    /// Create a history event for a subscription cancellation
    pub fn create_subscription_canceled_event(
        subscription: &Subscription,
        cancel_at_period_end: bool,
        actor: Option<String>,
    ) -> SubscriptionHistoryEvent {
        let mut canceled_subscription = subscription.clone();
        canceled_subscription.status = crate::billing::entities::SubscriptionStatus::Canceled;
        if cancel_at_period_end && canceled_subscription.current_period_end.is_some() {
            canceled_subscription.cancel_at = canceled_subscription.current_period_end;
        } else {
            canceled_subscription.cancel_at = Some(Utc::now());
        }
        canceled_subscription.updated_at = Utc::now();

        SubscriptionHistoryEvent {
            id: Uuid::now_v7().to_string(),
            subscription_id: subscription.id,
            event_type: HistoryEventType::Canceled,
            timestamp: Utc::now(),
            actor,
            changes: Some(serde_json::json!({
                "cancel_at_period_end": cancel_at_period_end,
                "canceled_at": canceled_subscription.cancel_at,
            })),
            previous_state: Some(serialize_subscription_state(subscription)),
            new_state: Some(serialize_subscription_state(&canceled_subscription)),
            realm_id: subscription.realm_id.clone(),
            created_at: Utc::now(),
        }
    }

    /// Create a history event for a subscription expiration
    pub fn create_subscription_expired_event(
        subscription: &Subscription,
        actor: Option<String>,
    ) -> SubscriptionHistoryEvent {
        let mut expired_subscription = subscription.clone();
        expired_subscription.status = crate::billing::entities::SubscriptionStatus::Expired;
        expired_subscription.updated_at = Utc::now();

        SubscriptionHistoryEvent {
            id: Uuid::now_v7().to_string(),
            subscription_id: subscription.id,
            event_type: HistoryEventType::Expired,
            timestamp: Utc::now(),
            actor,
            changes: None,
            previous_state: Some(serialize_subscription_state(subscription)),
            new_state: Some(serialize_subscription_state(&expired_subscription)),
            realm_id: subscription.realm_id.clone(),
            created_at: Utc::now(),
        }
    }

    /// Create a history event for a subscription renewal
    pub fn create_subscription_renewed_event(
        subscription: &Subscription,
        actor: Option<String>,
    ) -> SubscriptionHistoryEvent {
        let mut renewed_subscription = subscription.clone();
        renewed_subscription.status = crate::billing::entities::SubscriptionStatus::Active;
        renewed_subscription.updated_at = Utc::now();

        SubscriptionHistoryEvent {
            id: Uuid::now_v7().to_string(),
            subscription_id: subscription.id,
            event_type: HistoryEventType::Renewed,
            timestamp: Utc::now(),
            actor,
            changes: None,
            previous_state: Some(serialize_subscription_state(subscription)),
            new_state: Some(serialize_subscription_state(&renewed_subscription)),
            realm_id: subscription.realm_id.clone(),
            created_at: Utc::now(),
        }
    }

    /// Create a history event for subscription reactivation
    pub fn create_subscription_reactivated_event(
        subscription: &Subscription,
        actor: Option<String>,
    ) -> SubscriptionHistoryEvent {
        let mut reactivated_subscription = subscription.clone();
        reactivated_subscription.status = crate::billing::entities::SubscriptionStatus::Active;
        reactivated_subscription.cancel_at = None;
        reactivated_subscription.updated_at = Utc::now();

        SubscriptionHistoryEvent {
            id: Uuid::now_v7().to_string(),
            subscription_id: subscription.id,
            event_type: HistoryEventType::Reactivated,
            timestamp: Utc::now(),
            actor,
            changes: None,
            previous_state: Some(serialize_subscription_state(subscription)),
            new_state: Some(serialize_subscription_state(&reactivated_subscription)),
            realm_id: subscription.realm_id.clone(),
            created_at: Utc::now(),
        }
    }

    /// Create a history event for a past due subscription
    pub fn create_subscription_past_due_event(
        subscription: &Subscription,
        actor: Option<String>,
    ) -> SubscriptionHistoryEvent {
        let mut past_due_subscription = subscription.clone();
        past_due_subscription.status = crate::billing::entities::SubscriptionStatus::PastDue;
        past_due_subscription.updated_at = Utc::now();

        SubscriptionHistoryEvent {
            id: Uuid::now_v7().to_string(),
            subscription_id: subscription.id,
            event_type: HistoryEventType::PastDue,
            timestamp: Utc::now(),
            actor,
            changes: None,
            previous_state: Some(serialize_subscription_state(subscription)),
            new_state: Some(serialize_subscription_state(&past_due_subscription)),
            realm_id: subscription.realm_id.clone(),
            created_at: Utc::now(),
        }
    }

    /// Create a history event for a disputed subscription with custom changes
    pub fn create_subscription_disputed_event(
        subscription: &Subscription,
        changes: serde_json::Value,
        actor: Option<String>,
    ) -> SubscriptionHistoryEvent {
        let mut disputed_subscription = subscription.clone();
        disputed_subscription.status = crate::billing::entities::SubscriptionStatus::Dispute;
        disputed_subscription.updated_at = Utc::now();

        SubscriptionHistoryEvent {
            id: Uuid::now_v7().to_string(),
            subscription_id: subscription.id,
            event_type: HistoryEventType::Disputed,
            timestamp: Utc::now(),
            actor,
            changes: Some(changes),
            previous_state: Some(serialize_subscription_state(subscription)),
            new_state: Some(serialize_subscription_state(&disputed_subscription)),
            realm_id: subscription.realm_id.clone(),
            created_at: Utc::now(),
        }
    }

    /// Create a history event for a subscription refund without changing subscription state
    pub fn create_subscription_refunded_event(
        subscription: &Subscription,
        changes: serde_json::Value,
        actor: Option<String>,
    ) -> SubscriptionHistoryEvent {
        SubscriptionHistoryEvent {
            id: Uuid::now_v7().to_string(),
            subscription_id: subscription.id,
            event_type: HistoryEventType::Refunded,
            timestamp: Utc::now(),
            actor,
            changes: Some(changes),
            previous_state: Some(serialize_subscription_state(subscription)),
            new_state: Some(serialize_subscription_state(subscription)),
            realm_id: subscription.realm_id.clone(),
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::billing::entities::{BillingPeriod, SubscriptionStatus, SubscriptionTier};

    fn create_test_subscription() -> Subscription {
        Subscription {
            id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            user_id: None,
            external_subscription_id: "creem-123".to_string(),
            external_product_id: "product-1".to_string(),
            payment_provider: "creem".to_string(),
            status: SubscriptionStatus::Active,
            tier: SubscriptionTier::Professional,
            current_period_start: Some(Utc::now()),
            current_period_end: Some(Utc::now() + chrono::Duration::days(30)),
            cancel_at_period_end: false,
            client_app_id: Some(Uuid::now_v7()),
            plan_id: Some(Uuid::now_v7()),
            billing_period: BillingPeriod::Monthly,
            cancel_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_create_subscription_created_event() {
        let subscription = create_test_subscription();
        let actor = "webhook".to_string();
        let event = SubscriptionHistoryService::create_subscription_created_event(
            &subscription,
            Some(actor.clone()),
        );

        assert_eq!(event.subscription_id, subscription.id);
        assert_eq!(event.event_type, HistoryEventType::Created);
        assert_eq!(event.actor, Some(actor));
        assert!(event.previous_state.is_none());
        assert!(event.new_state.is_some());
    }

    #[test]
    fn test_create_subscription_canceled_event() {
        let subscription = create_test_subscription();
        let actor = "user-123".to_string();
        let event = SubscriptionHistoryService::create_subscription_canceled_event(
            &subscription,
            true,
            Some(actor.clone()),
        );

        assert_eq!(event.subscription_id, subscription.id);
        assert_eq!(event.event_type, HistoryEventType::Canceled);
        assert_eq!(event.actor, Some(actor));
        assert!(event.previous_state.is_some());
        assert!(event.new_state.is_some());
    }

    #[test]
    fn test_create_subscription_refunded_event_keeps_subscription_state() {
        let subscription = create_test_subscription();
        let changes = serde_json::json!({
            "refund_id": "refund-123",
            "amount": 1000,
        });
        let event = SubscriptionHistoryService::create_subscription_refunded_event(
            &subscription,
            changes,
            Some("webhook".to_string()),
        );

        assert_eq!(event.subscription_id, subscription.id);
        assert_eq!(event.event_type, HistoryEventType::Refunded);
        assert_eq!(event.previous_state, event.new_state);
        assert!(event.changes.is_some());
    }
}
