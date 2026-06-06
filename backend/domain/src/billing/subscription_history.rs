use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::billing::entities::Subscription;
use crate::common::entities::app_errors::CoreError;

/// Actor types for subscription history events
pub const ACTOR_WEBHOOK: &str = "webhook";
pub const ACTOR_SYSTEM: &str = "system";
pub const ACTOR_USER_PREFIX: &str = "user-";
pub const SUBSCRIPTION_STATUS_UNKNOWN: &str = "unknown";

/// History event types for subscription changes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryEventType {
    /// Subscription was created
    Created,
    /// Subscription was upgraded (entitlement_key changed to higher tier)
    Upgraded,
    /// Subscription was downgraded (entitlement_key changed to lower tier)
    Downgraded,
    /// Entitlement key changed (non-directional)
    EntitlementChanged,
    /// Subscription was canceled
    Canceled,
    /// Subscription expired
    Expired,
    /// Subscription was renewed
    Renewed,
    /// Subscription was reactivated after cancellation
    Reactivated,
    /// Subscription payment failed
    PastDue,
    /// Subscription dispute/chargeback created
    Disputed,
    /// Subscription payment was refunded
    Refunded,
    /// Subscription billing period changed
    BillingPeriodChanged,
}

impl HistoryEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            HistoryEventType::Created => "created",
            HistoryEventType::Upgraded => "upgraded",
            HistoryEventType::Downgraded => "downgraded",
            HistoryEventType::EntitlementChanged => "entitlement_changed",
            HistoryEventType::Canceled => "canceled",
            HistoryEventType::Expired => "expired",
            HistoryEventType::Renewed => "renewed",
            HistoryEventType::Reactivated => "reactivated",
            HistoryEventType::PastDue => "past_due",
            HistoryEventType::Disputed => "disputed",
            HistoryEventType::Refunded => "refunded",
            HistoryEventType::BillingPeriodChanged => "billing_period_changed",
        }
    }
}

impl std::str::FromStr for HistoryEventType {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "created" => Ok(HistoryEventType::Created),
            "upgraded" => Ok(HistoryEventType::Upgraded),
            "downgraded" => Ok(HistoryEventType::Downgraded),
            "entitlement_changed" => Ok(HistoryEventType::EntitlementChanged),
            "canceled" => Ok(HistoryEventType::Canceled),
            "expired" => Ok(HistoryEventType::Expired),
            "renewed" => Ok(HistoryEventType::Renewed),
            "reactivated" => Ok(HistoryEventType::Reactivated),
            "past_due" => Ok(HistoryEventType::PastDue),
            "disputed" => Ok(HistoryEventType::Disputed),
            "refunded" => Ok(HistoryEventType::Refunded),
            "billing_period_changed" => Ok(HistoryEventType::BillingPeriodChanged),
            _ => Err(CoreError::BadRequest(format!(
                "Invalid history event type: {}",
                s
            ))),
        }
    }
}

/// Sort order for history queries
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Query parameters for subscription history
#[derive(Debug, Deserialize)]
pub struct SubscriptionHistoryQuery {
    /// Filter by user ID (client_app_id)
    pub user_id: Option<Uuid>,
    /// Filter by entitlement key
    pub entitlement_key: Option<String>,
    /// Filter by event type
    pub event_type: Option<HistoryEventType>,
    /// Filter by subscription status
    pub subscription_status: Option<String>,
    /// Filter by date range start
    pub from_date: Option<DateTime<Utc>>,
    /// Filter by date range end
    pub to_date: Option<DateTime<Utc>>,
    /// Page number (1-based)
    pub page: Option<u64>,
    /// Number of items per page
    pub page_size: Option<u64>,
    /// Sort field
    pub sort_by: Option<String>,
    /// Sort order
    pub sort_order: Option<SortOrder>,
}

/// Subscription history event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionHistoryEvent {
    pub id: String,
    pub subscription_id: Uuid,
    pub event_type: HistoryEventType,
    pub timestamp: DateTime<Utc>,
    pub actor: Option<String>,
    pub changes: Option<serde_json::Value>,
    pub previous_state: Option<serde_json::Value>,
    pub new_state: Option<serde_json::Value>,
    pub realm_id: String,
    pub created_at: DateTime<Utc>,
}

/// Subscription state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionState {
    pub id: Uuid,
    pub realm_id: String,
    pub status: String,
    pub entitlement_key: String,
    /// External product ID tracks the provider's product/plan identifier.
    /// A change with the same entitlement_key indicates a billing period change.
    pub external_product_id: Option<String>,
    pub client_app_id: Option<Uuid>,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub cancel_at: Option<DateTime<Utc>>,
}

impl SubscriptionState {
    /// Serialize a Subscription domain entity to a state snapshot
    pub fn from_subscription(sub: &Subscription) -> Self {
        SubscriptionState {
            id: sub.id,
            realm_id: sub.realm_id.clone(),
            status: sub.status.as_str().to_string(),
            entitlement_key: sub.entitlement_key.clone(),
            external_product_id: Some(sub.external_product_id.clone()),
            client_app_id: sub.client_app_id,
            current_period_start: sub.current_period_start,
            current_period_end: sub.current_period_end,
            cancel_at_period_end: sub.cancel_at_period_end,
            cancel_at: sub.cancel_at,
        }
    }
}

/// Change details for subscription modifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionChanges {
    pub changed_fields: Vec<String>,
    pub previous_entitlement_key: Option<String>,
    pub new_entitlement_key: Option<String>,
    pub previous_status: Option<String>,
    pub new_status: Option<String>,
    pub previous_external_product_id: Option<String>,
    pub new_external_product_id: Option<String>,
}

impl SubscriptionChanges {
    /// Calculate changes between two subscription states
    pub fn from_states(old_state: &SubscriptionState, new_state: &SubscriptionState) -> Self {
        let mut changed_fields = Vec::new();

        if old_state.entitlement_key != new_state.entitlement_key {
            changed_fields.push("entitlement_key".to_string());
        }
        if old_state.status != new_state.status {
            changed_fields.push("status".to_string());
        }
        if old_state.external_product_id != new_state.external_product_id {
            changed_fields.push("external_product_id".to_string());
        }

        SubscriptionChanges {
            changed_fields,
            previous_entitlement_key: Some(old_state.entitlement_key.clone()),
            new_entitlement_key: Some(new_state.entitlement_key.clone()),
            previous_status: Some(old_state.status.clone()),
            new_status: Some(new_state.status.clone()),
            previous_external_product_id: old_state.external_product_id.clone(),
            new_external_product_id: new_state.external_product_id.clone(),
        }
    }
}

/// Helper function to detect the type of change between two subscriptions
pub fn detect_change_type(
    old_subscription: &Subscription,
    new_subscription: &Subscription,
) -> HistoryEventType {
    let old_state = SubscriptionState::from_subscription(old_subscription);
    let new_state = SubscriptionState::from_subscription(new_subscription);

    // Check for entitlement key changes
    if old_state.entitlement_key != new_state.entitlement_key {
        // Determine direction based on entitlement_key comparison.
        // Since entitlement keys are opaque strings, we default to EntitlementChanged.
        // Specific upgrade/downgrade detection should be done at the application layer
        // by comparing points_per_period values from the mapping table.
        // For backward compatibility with existing upgrade/downgrade detection,
        // we use a simple heuristic: compare the keys lexicographically.
        // The caller should override with specific tier knowledge when available.
        HistoryEventType::EntitlementChanged
    } else if old_state.status != new_state.status {
        match &new_subscription.status {
            crate::billing::entities::SubscriptionStatus::Canceled => HistoryEventType::Canceled,
            crate::billing::entities::SubscriptionStatus::Expired => HistoryEventType::Expired,
            crate::billing::entities::SubscriptionStatus::Active
                if matches!(
                    &old_subscription.status,
                    crate::billing::entities::SubscriptionStatus::Canceled
                        | crate::billing::entities::SubscriptionStatus::Paused
                        | crate::billing::entities::SubscriptionStatus::Expired
                ) =>
            {
                HistoryEventType::Reactivated
            }
            crate::billing::entities::SubscriptionStatus::Active => HistoryEventType::Renewed,
            _ => HistoryEventType::Created,
        }
    } else if old_state.external_product_id != new_state.external_product_id {
        // Same entitlement_key and status, but external_product_id changed.
        // This indicates a billing period change (e.g. monthly -> yearly on same tier).
        HistoryEventType::BillingPeriodChanged
    } else {
        HistoryEventType::Created
    }
}

/// Helper function to calculate changes between two subscriptions
pub fn calculate_changes(
    old_subscription: &Subscription,
    new_subscription: &Subscription,
) -> serde_json::Value {
    let old_state = SubscriptionState::from_subscription(old_subscription);
    let new_state = SubscriptionState::from_subscription(new_subscription);
    let changes = SubscriptionChanges::from_states(&old_state, &new_state);

    serde_json::to_value(changes).unwrap_or_else(|_| serde_json::json!({}))
}

/// Helper function to serialize subscription state
pub fn serialize_subscription_state(subscription: &Subscription) -> serde_json::Value {
    let state = SubscriptionState::from_subscription(subscription);
    serde_json::to_value(state).unwrap_or_else(|_| serde_json::json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::billing::entities::SubscriptionStatus;

    fn create_test_subscription(entitlement_key: &str) -> Subscription {
        Subscription {
            id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            user_id: None,
            external_subscription_id: "sub_test_1".to_string(),
            external_product_id: "product-1".to_string(),
            payment_provider: "creem".to_string(),
            status: SubscriptionStatus::Active,
            entitlement_key: entitlement_key.to_string(),
            external_price_id: None,
            provider_metadata: None,
            synced_at: None,
            current_period_start: None,
            current_period_end: None,
            cancel_at_period_end: false,
            client_app_id: None,
            cancel_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_detect_change_type_entitlement_changed() {
        let old = create_test_subscription("starter-plan");
        let mut new = old.clone();
        new.entitlement_key = "pro-plan".to_string();

        assert_eq!(
            detect_change_type(&old, &new),
            HistoryEventType::EntitlementChanged
        );
    }

    #[test]
    fn test_detect_change_type_canceled() {
        let old = create_test_subscription("pro-plan");
        let mut new = old.clone();
        new.status = SubscriptionStatus::Canceled;

        assert_eq!(detect_change_type(&old, &new), HistoryEventType::Canceled);
    }

    #[test]
    fn test_detect_change_type_same_entitlement_same_status() {
        let old = create_test_subscription("pro-plan");
        let new = old.clone();

        assert_eq!(detect_change_type(&old, &new), HistoryEventType::Created);
    }
}
