use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;

/// Helper for parsing string enums with a default error message
fn parse_enum<T, F>(s: &str, error_prefix: &str, variants: F) -> Result<T, CoreError>
where
    F: Fn(&str) -> Option<T>,
{
    variants(s).ok_or_else(|| CoreError::BadRequest(format!("{}: {}", error_prefix, s)))
}

/// Plan type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlanType {
    Monthly,
    Yearly,
}

impl PlanType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlanType::Monthly => "monthly",
            PlanType::Yearly => "yearly",
        }
    }
}

impl std::str::FromStr for PlanType {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_enum(
            s.to_lowercase().as_str(),
            "Invalid plan type",
            |s| match s {
                "monthly" => Some(PlanType::Monthly),
                "yearly" => Some(PlanType::Yearly),
                _ => None,
            },
        )
    }
}

/// Payment provider enum
///
/// DEPRECATED: This enum is hardcoded and doesn't allow dynamic payment providers.
/// Use String type for payment_provider field instead. Values should be read from
/// realm_config.config_type (ConfigType::Creem, ConfigType::Stripe, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PaymentProvider {
    Creem,
    Stripe,
}

impl PaymentProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            PaymentProvider::Creem => "creem",
            PaymentProvider::Stripe => "stripe",
        }
    }
}

impl std::str::FromStr for PaymentProvider {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_enum(
            s.to_lowercase().as_str(),
            "Invalid payment provider",
            |s| match s {
                "creem" => Some(PaymentProvider::Creem),
                "stripe" => Some(PaymentProvider::Stripe),
                _ => None,
            },
        )
    }
}

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
    pub tier: SubscriptionTier,
    pub current_period_start: Option<chrono::DateTime<chrono::Utc>>,
    pub current_period_end: Option<chrono::DateTime<chrono::Utc>>,
    pub cancel_at_period_end: bool,
    pub client_app_id: Option<Uuid>,
    pub plan_id: Option<Uuid>,
    pub billing_period: BillingPeriod,
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

/// Subscription tier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionTier {
    #[default]
    Free,
    Starter,
    Professional,
    Enterprise,
}

impl SubscriptionTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubscriptionTier::Free => "free",
            SubscriptionTier::Starter => "starter",
            SubscriptionTier::Professional => "professional",
            SubscriptionTier::Enterprise => "enterprise",
        }
    }
}

impl std::str::FromStr for SubscriptionTier {
    type Err = crate::common::entities::app_errors::CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "free" => Ok(SubscriptionTier::Free),
            "starter" => Ok(SubscriptionTier::Starter),
            "professional" => Ok(SubscriptionTier::Professional),
            "enterprise" => Ok(SubscriptionTier::Enterprise),
            _ => Err(crate::common::entities::app_errors::CoreError::BadRequest(
                format!("Invalid subscription tier: {}", s),
            )),
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

/// Plan domain entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: Uuid,
    pub realm_id: String,
    pub name: String,
    pub title: String,
    pub description: Option<String>,

    // Pricing information
    #[serde(rename = "type")]
    pub r#type: PlanType,
    pub price: i32,       // Price in cents
    pub currency: String, // USD, EUR, CNY

    // NOTE: Payment provider fields removed - see PlanPaymentProvider entity
    // payment_provider, external_product_id, external_price_id are now
    // managed through the PlanPaymentProvider entity

    // Checkout URL (third-party payment page)
    pub checkout_url: Option<String>,

    // Plan status
    pub active: bool,
    pub trial_days: i32,
    pub sort_order: i32,
    pub product_id: Uuid,

    // Timestamps
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Plan Payment Provider mapping entity
///
/// This entity manages the relationship between Plans and Payment Providers,
/// allowing a single Plan to support multiple payment platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPaymentProvider {
    pub id: Uuid,
    pub plan_id: Uuid,
    /// Payment provider name (dynamic: stripe, creem, shopify, etc.)
    /// NOT an enum - supports future payment providers without database migration
    pub payment_provider: String,
    /// External product ID from the payment provider
    pub external_product_id: String,
    /// External price ID from the payment provider (optional)
    pub external_price_id: Option<String>,
    /// Whether this mapping is enabled for checkout
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Client App Plan junction domain entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientAppPlan {
    pub id: Uuid,
    pub client_app_id: Uuid,
    pub plan_id: Uuid,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Billing period enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BillingPeriod {
    Monthly,
    Yearly,
}

impl BillingPeriod {
    pub fn as_str(&self) -> &'static str {
        match self {
            BillingPeriod::Monthly => "monthly",
            BillingPeriod::Yearly => "yearly",
        }
    }
}

impl std::fmt::Display for BillingPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<String> for BillingPeriod {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "monthly" => BillingPeriod::Monthly,
            "yearly" => BillingPeriod::Yearly,
            _ => BillingPeriod::Monthly, // Default
        }
    }
}

impl From<&str> for BillingPeriod {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "monthly" => BillingPeriod::Monthly,
            "yearly" => BillingPeriod::Yearly,
            _ => BillingPeriod::Monthly, // Default
        }
    }
}

// Product Entity

/// Product domain entity - catalog object for organizing Plans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: Uuid,
    pub realm_id: String,
    pub code: String,
    pub title: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub plans_count: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
