use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;

/// Subscription entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: Uuid,
    pub realm_id: String,
    pub user_id: Option<Uuid>,
    /// External subscription ID from payment provider (Stripe, Creem, etc.)
    pub external_subscription_id: String,
    /// External product ID from payment provider
    /// Required field - every subscription must have a product/pricing plan
    pub external_product_id: String,
    /// Payment provider type (stripe, creem)
    /// Used to determine which external IDs are populated
    pub payment_provider: String,
    pub status: SubscriptionStatus,
    /// Herald entitlement key replacing plan_id + tier
    pub entitlement_key: String,
    /// External price ID from the payment provider
    pub external_price_id: Option<String>,
    pub bucket_id: Option<Uuid>,
    /// Provider-specific metadata as JSON
    pub provider_metadata: Option<serde_json::Value>,
    /// Last sync timestamp from provider
    pub synced_at: Option<chrono::DateTime<chrono::Utc>>,
    pub current_period_start: Option<chrono::DateTime<chrono::Utc>>,
    pub current_period_end: Option<chrono::DateTime<chrono::Utc>>,
    pub cancel_at_period_end: bool,
    pub client_app_id: Option<Uuid>,
    pub cancel_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Subscription status - Creem official states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionStatus {
    // Creem official states
    Active,          // Subscription active, normally billed - has access
    Canceled,        // Canceled - no access
    Expired,         // Subscription expired - no access
    Incomplete,      // Payment needs to be completed within 23 hours - no access
    Paused,          // Paused - no access
    Trialing,        // Trial period - has access
    PastDue,         // Payment failed or overdue - no access
    ScheduledCancel, // Scheduled to cancel at period end - has access (until cancel date)
    Dispute, // Payment dispute/refund chargeback in progress - has access (during investigation)

    // Local extension state
    Pending, // Reserved for local extension
}

impl SubscriptionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubscriptionStatus::Active => "active",
            SubscriptionStatus::Canceled => "canceled",
            SubscriptionStatus::Expired => "expired",
            SubscriptionStatus::Incomplete => "incomplete",
            SubscriptionStatus::Pending => "pending",
            SubscriptionStatus::Trialing => "trialing",
            SubscriptionStatus::Paused => "paused",
            SubscriptionStatus::PastDue => "past_due",
            SubscriptionStatus::ScheduledCancel => "scheduled_cancel",
            SubscriptionStatus::Dispute => "dispute",
        }
    }

    /// Check if subscription has access based on status
    pub fn has_access(&self) -> bool {
        matches!(
            self,
            SubscriptionStatus::Active
                | SubscriptionStatus::Trialing
                | SubscriptionStatus::ScheduledCancel
                | SubscriptionStatus::Dispute
        )
    }

    pub fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            // From Pending
            (SubscriptionStatus::Pending, SubscriptionStatus::Active)
            | (SubscriptionStatus::Pending, SubscriptionStatus::Trialing)
            | (SubscriptionStatus::Pending, SubscriptionStatus::Incomplete)
            | (SubscriptionStatus::Pending, SubscriptionStatus::Canceled)
            | (SubscriptionStatus::Pending, SubscriptionStatus::Expired)
            | (SubscriptionStatus::Pending, SubscriptionStatus::Paused)
            | (SubscriptionStatus::Pending, SubscriptionStatus::PastDue)
            // From Incomplete
            | (SubscriptionStatus::Incomplete, SubscriptionStatus::Active)
            | (SubscriptionStatus::Incomplete, SubscriptionStatus::Canceled)
            | (SubscriptionStatus::Incomplete, SubscriptionStatus::Expired)
            // From Trialing
            | (SubscriptionStatus::Trialing, SubscriptionStatus::Active)
            | (SubscriptionStatus::Trialing, SubscriptionStatus::Canceled)
            | (SubscriptionStatus::Trialing, SubscriptionStatus::Expired)
            | (SubscriptionStatus::Trialing, SubscriptionStatus::Paused)
            | (SubscriptionStatus::Trialing, SubscriptionStatus::PastDue)
            | (SubscriptionStatus::Trialing, SubscriptionStatus::Dispute)
            // From Active
            | (SubscriptionStatus::Active, SubscriptionStatus::Canceled)
            | (SubscriptionStatus::Active, SubscriptionStatus::Expired)
            | (SubscriptionStatus::Active, SubscriptionStatus::Paused)
            | (SubscriptionStatus::Active, SubscriptionStatus::PastDue)
            | (SubscriptionStatus::Active, SubscriptionStatus::ScheduledCancel)
            | (SubscriptionStatus::Active, SubscriptionStatus::Dispute)
            // From Paused
            | (SubscriptionStatus::Paused, SubscriptionStatus::Active)
            | (SubscriptionStatus::Paused, SubscriptionStatus::Canceled)
            | (SubscriptionStatus::Paused, SubscriptionStatus::Expired)
            | (SubscriptionStatus::Paused, SubscriptionStatus::PastDue)
            | (SubscriptionStatus::Paused, SubscriptionStatus::Dispute)
            // From PastDue
            | (SubscriptionStatus::PastDue, SubscriptionStatus::Active)
            | (SubscriptionStatus::PastDue, SubscriptionStatus::Canceled)
            | (SubscriptionStatus::PastDue, SubscriptionStatus::Expired)
            | (SubscriptionStatus::PastDue, SubscriptionStatus::Dispute)
            | (SubscriptionStatus::PastDue, SubscriptionStatus::ScheduledCancel)
            // From Dispute
            | (SubscriptionStatus::Dispute, SubscriptionStatus::Active)
            | (SubscriptionStatus::Dispute, SubscriptionStatus::Canceled)
            | (SubscriptionStatus::Dispute, SubscriptionStatus::Expired)
            | (SubscriptionStatus::Dispute, SubscriptionStatus::PastDue)
            // From ScheduledCancel
            | (SubscriptionStatus::ScheduledCancel, SubscriptionStatus::Active)
            | (SubscriptionStatus::ScheduledCancel, SubscriptionStatus::Canceled)
            | (SubscriptionStatus::ScheduledCancel, SubscriptionStatus::Expired)
            // From Canceled
            | (SubscriptionStatus::Canceled, SubscriptionStatus::Expired) => true,
            _ => self == target,
        }
    }
}

impl std::str::FromStr for SubscriptionStatus {
    type Err = crate::common::entities::app_errors::CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "active" => Ok(SubscriptionStatus::Active),
            "canceled" => Ok(SubscriptionStatus::Canceled),
            "expired" => Ok(SubscriptionStatus::Expired),
            "incomplete" => Ok(SubscriptionStatus::Incomplete),
            "pending" => Ok(SubscriptionStatus::Pending),
            "trialing" => Ok(SubscriptionStatus::Trialing),
            "paused" => Ok(SubscriptionStatus::Paused),
            "past_due" => Ok(SubscriptionStatus::PastDue),
            "scheduled_cancel" => Ok(SubscriptionStatus::ScheduledCancel),
            "dispute" => Ok(SubscriptionStatus::Dispute),
            _ => Err(
                crate::common::entities::app_errors::CoreError::InvalidSubscriptionStatus(
                    s.to_string(),
                ),
            ),
        }
    }
}

/// Payment event entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentEvent {
    pub id: Uuid,
    pub realm_id: String,
    /// External event ID from payment provider (unique per provider)
    pub external_event_id: String,
    /// Payment provider type (creem, stripe, etc.)
    pub payment_provider: String,
    pub event_type: String,
    pub subscription_id: Option<Uuid>,
    pub payload: serde_json::Value,
    pub processed: bool,
    pub processing_started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// EntitlementMapping domain entity - maps provider products to Herald entitlement keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitlementMapping {
    pub id: Uuid,
    pub realm_id: String,
    pub payment_provider: String,
    pub external_product_id: String,
    pub external_price_id: Option<String>,
    pub bucket_id: Uuid,
    pub entitlement_key: String,
    pub billing_type: Option<BillingType>,
    pub billing_period: Option<String>,
    pub points_per_period: Option<i64>,
    pub grant_period_type: Option<String>,
    pub validity_days: Option<i64>,
    pub grant_on_subscribe: bool,
    pub max_periods: Option<i64>,
    pub enabled: bool,
    pub provider_product_info: Option<serde_json::Value>,
    pub synced_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl EntitlementMapping {
    /// Returns the billing kind string for use in provider metadata.
    ///
    /// Maps billing_type to the PRD-defined values: "subscription" for recurring,
    /// "one_time" for one-time. Defaults to "subscription" when unset.
    pub fn billing_kind(&self) -> String {
        match self.billing_type {
            Some(BillingType::OneTime) => "one_time".to_string(),
            _ => "subscription".to_string(),
        }
    }
}

/// Billing type enum for entitlement mappings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BillingType {
    Recurring,
    OneTime,
}

impl BillingType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BillingType::Recurring => "recurring",
            BillingType::OneTime => "one_time",
        }
    }
}

impl std::str::FromStr for BillingType {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "recurring" => Ok(BillingType::Recurring),
            "one_time" => Ok(BillingType::OneTime),
            _ => Err(CoreError::BadRequest(format!(
                "Invalid billing type: {}",
                s
            ))),
        }
    }
}

/// Aggregated feature availability facts for a realm.
/// Used to determine which billing UI features should be shown.
#[derive(Debug, Clone)]
pub struct FeatureFacts {
    pub has_payment_providers: bool,
    pub has_entitlement_mappings: bool,
    pub has_enabled_mappings: bool,
    pub has_one_time_mappings: bool,
    pub has_invoice_seller_config: bool,
    pub has_invoices: bool,
    pub has_subscription_history: bool,
}
