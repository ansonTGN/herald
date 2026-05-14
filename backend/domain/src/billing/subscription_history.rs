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
    /// Subscription was upgraded to a higher tier
    Upgraded,
    /// Subscription was downgraded to a lower tier
    Downgraded,
    /// Subscription was canceled
    Canceled,
    /// Subscription expired
    Expired,
    /// Subscription was renewed
    Renewed,
    /// Subscription was reactivated after cancellation
    Reactivated,
    /// Billing period was changed (monthly <-> yearly)
    BillingPeriodChanged,
    /// Subscription payment failed
    PastDue,
    /// Subscription dispute/chargeback created
    Disputed,
}

impl HistoryEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            HistoryEventType::Created => "created",
            HistoryEventType::Upgraded => "upgraded",
            HistoryEventType::Downgraded => "downgraded",
            HistoryEventType::Canceled => "canceled",
            HistoryEventType::Expired => "expired",
            HistoryEventType::Renewed => "renewed",
            HistoryEventType::Reactivated => "reactivated",
            HistoryEventType::BillingPeriodChanged => "billing_period_changed",
            HistoryEventType::PastDue => "past_due",
            HistoryEventType::Disputed => "disputed",
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
            "canceled" => Ok(HistoryEventType::Canceled),
            "expired" => Ok(HistoryEventType::Expired),
            "renewed" => Ok(HistoryEventType::Renewed),
            "reactivated" => Ok(HistoryEventType::Reactivated),
            "billing_period_changed" => Ok(HistoryEventType::BillingPeriodChanged),
            "past_due" => Ok(HistoryEventType::PastDue),
            "disputed" => Ok(HistoryEventType::Disputed),
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
    /// Filter by plan ID
    pub plan_id: Option<Uuid>,
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
    pub tier: String,
    pub plan_id: Option<Uuid>,
    pub client_app_id: Option<Uuid>,
    pub billing_period: String,
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
            tier: sub.tier.as_str().to_string(),
            plan_id: sub.plan_id,
            client_app_id: sub.client_app_id,
            billing_period: sub.billing_period.as_str().to_string(),
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
    pub previous_plan_id: Option<Uuid>,
    pub new_plan_id: Option<Uuid>,
    pub previous_status: Option<String>,
    pub new_status: Option<String>,
    pub previous_tier: Option<String>,
    pub new_tier: Option<String>,
    pub previous_billing_period: Option<String>,
    pub new_billing_period: Option<String>,
}

impl SubscriptionChanges {
    /// Calculate changes between two subscription states
    pub fn from_states(old_state: &SubscriptionState, new_state: &SubscriptionState) -> Self {
        let mut changed_fields = Vec::new();

        if old_state.plan_id != new_state.plan_id {
            changed_fields.push("plan_id".to_string());
        }
        if old_state.status != new_state.status {
            changed_fields.push("status".to_string());
        }
        if old_state.tier != new_state.tier {
            changed_fields.push("tier".to_string());
        }
        if old_state.billing_period != new_state.billing_period {
            changed_fields.push("billing_period".to_string());
        }

        SubscriptionChanges {
            changed_fields,
            previous_plan_id: old_state.plan_id,
            new_plan_id: new_state.plan_id,
            previous_status: Some(old_state.status.clone()),
            new_status: Some(new_state.status.clone()),
            previous_tier: Some(old_state.tier.clone()),
            new_tier: Some(new_state.tier.clone()),
            previous_billing_period: Some(old_state.billing_period.clone()),
            new_billing_period: Some(new_state.billing_period.clone()),
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

    // Check for plan changes (upgrade/downgrade)
    if old_state.plan_id != new_state.plan_id || old_state.tier != new_state.tier {
        // Simple heuristic: if price increased, it's an upgrade; otherwise downgrade
        // In a real implementation, you'd query plan prices from the database
        // For now, we'll assume plan change direction based on tier comparison
        let old_tier = &old_subscription.tier;
        let new_tier = &new_subscription.tier;

        match (old_tier, new_tier) {
            (crate::billing::entities::SubscriptionTier::Free, _) => HistoryEventType::Upgraded,
            (_, crate::billing::entities::SubscriptionTier::Enterprise) => {
                HistoryEventType::Upgraded
            }
            (
                crate::billing::entities::SubscriptionTier::Professional,
                crate::billing::entities::SubscriptionTier::Free,
            )
            | (
                crate::billing::entities::SubscriptionTier::Starter,
                crate::billing::entities::SubscriptionTier::Free,
            ) => HistoryEventType::Downgraded,
            _ => HistoryEventType::Upgraded, // Default to upgrade for ambiguous cases
        }
    } else if old_state.billing_period != new_state.billing_period {
        HistoryEventType::BillingPeriodChanged
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
            _ => HistoryEventType::Created, // Default for other status changes
        }
    } else {
        // No significant change detected, default to created
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
    use crate::billing::entities::{BillingPeriod, SubscriptionStatus, SubscriptionTier};

    #[test]
    fn test_detect_change_type_upgrade() {
        let old = Subscription {
            id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            user_id: None,
            external_subscription_id: "sub_test_1".to_string(),
            external_product_id: "product-1".to_string(),
            payment_provider: "creem".to_string(),
            status: SubscriptionStatus::Active,
            tier: SubscriptionTier::Free,
            current_period_start: None,
            current_period_end: None,
            cancel_at_period_end: false,
            client_app_id: None,
            plan_id: Some(Uuid::now_v7()),
            billing_period: BillingPeriod::Monthly,
            cancel_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut new = old.clone();
        new.tier = SubscriptionTier::Professional;

        assert_eq!(detect_change_type(&old, &new), HistoryEventType::Upgraded);
    }

    #[test]
    fn test_detect_change_type_canceled() {
        let old = Subscription {
            id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            user_id: None,
            external_subscription_id: "sub_test_2".to_string(),
            external_product_id: "product-1".to_string(),
            payment_provider: "creem".to_string(),
            status: SubscriptionStatus::Active,
            tier: SubscriptionTier::Professional,
            current_period_start: None,
            current_period_end: None,
            cancel_at_period_end: false,
            client_app_id: None,
            plan_id: Some(Uuid::now_v7()),
            billing_period: BillingPeriod::Monthly,
            cancel_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut new = old.clone();
        new.status = SubscriptionStatus::Canceled;

        assert_eq!(detect_change_type(&old, &new), HistoryEventType::Canceled);
    }

    #[test]
    fn test_detect_change_type_billing_period() {
        let old = Subscription {
            id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            user_id: None,
            external_subscription_id: "sub_test_3".to_string(),
            external_product_id: "product-1".to_string(),
            payment_provider: "creem".to_string(),
            status: SubscriptionStatus::Active,
            tier: SubscriptionTier::Professional,
            current_period_start: None,
            current_period_end: None,
            cancel_at_period_end: false,
            client_app_id: None,
            plan_id: Some(Uuid::now_v7()),
            billing_period: BillingPeriod::Monthly,
            cancel_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut new = old.clone();
        new.billing_period = BillingPeriod::Yearly;

        assert_eq!(
            detect_change_type(&old, &new),
            HistoryEventType::BillingPeriodChanged
        );
    }
}
