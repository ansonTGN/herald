//! Google Play Developer API serde models.
//!
//! These are hand-written, deliberately minimal views over the six Play
//! Developer API responses Herald consumes. They use `#[serde(default)]` on
//! every struct so that **unknown fields are tolerated** (Google evolves the
//! API without notice) and missing fields fall back to `None` / empty rather
//! than failing the whole parse. `#[serde(rename_all = "camelCase")]` matches
//! Google's JSON casing. Only the fields the IAP fulfillment /
//! reconciliation paths actually read are declared.
//!
//! Reference:
//! - <https://developers.google.com/android-publisher/api-ref/rest/v3/purchases.subscriptionsv2>
//! - <https://developers.google.com/android-publisher/api-ref/rest/v3/purchases.products>
//! - <https://developers.google.com/android-publisher/api-ref/rest/v3/purchases.voidedpurchases>

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

/// Deserialize a Google API millisecond field that may arrive as either a JSON
/// number (`1700000000000`) or a JSON string (`"1700000000000"`). Google's Play
/// Developer API uses both forms across endpoints / versions, so the model
/// tolerates either. Returns `None` for absent fields.
fn flexible_millis<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumOrStr {
        Num(i64),
        Str(String),
    }
    match Option::<NumOrStr>::deserialize(deserializer)? {
        Some(NumOrStr::Num(n)) => Ok(Some(n)),
        Some(NumOrStr::Str(s)) => s
            .parse::<i64>()
            .map(Some)
            .map_err(|e| Error::custom(format!("invalid millis string: {e}"))),
        None => Ok(None),
    }
}

/// `purchases.subscriptionsv2.get` response.
///
/// Fields required by the reconciliation job (state / acknowledgement / ownership)
/// are declared; everything else is ignored via `#[serde(default)]`.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct SubscriptionPurchaseV2 {
    /// Current lifecycle state of the subscription.
    pub subscription_state: Option<String>,
    /// Acknowledgement state: `ACKNOWLEDGEMENT_STATE_ACKNOWLEDGED` etc.
    pub acknowledgement_state: Option<String>,
    /// Line items (the product(s) purchased). Tolerated as a vec of unknown shape.
    pub line_items: Vec<SubscriptionLineItem>,
    /// Opaque token linking this purchase to the user (Herald reads this for
    /// ownership checks).
    pub obfuscated_external_account_id: Option<String>,
    /// ISO-8601 time the subscription was granted.
    pub start_time: Option<DateTime<Utc>>,
    /// Most recent state change time.
    pub latest_update_time: Option<DateTime<Utc>>,
    /// Previously-linked purchase token (migration / upgrade scenarios).
    pub linked_purchase_token: Option<String>,
    /// Test purchase flag.
    pub test_purchase: Option<TestPurchase>,
    /// Whether the subscription is entitled to a grace period.
    pub grace_period_expire_time: Option<DateTime<Utc>>,
    /// Region code (tolerated).
    pub region_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct SubscriptionLineItem {
    /// Product ID (SKU) purchased.
    pub product_id: Option<String>,
    /// Purchase type: e.g. `SUBSCRIPTION`.
    pub purchase_type: Option<String>,
    /// State of this line item.
    pub state: Option<String>,
    /// Acknowledgement state of this line item.
    pub acknowledgement_state: Option<String>,
    /// Expiry of the current entitlement window.
    pub expiry_time: Option<DateTime<Utc>>,
    /// Auto-renewing plan details (when present).
    pub auto_renewing_plan: Option<AutoRenewingPlan>,
    /// Prepaid plan details (when present).
    pub prepaid_plan: Option<PrepaidPlan>,
    /// Offer details (when present).
    pub offer_details: Option<OfferDetails>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct AutoRenewingPlan {
    pub auto_renewal_enabled: bool,
    pub recurring_price: Option<Price>,
    pub price_change_details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct PrepaidPlan {
    pub allow_extend_after_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct OfferDetails {
    pub offer_id: Option<String>,
    pub base_plan_id: Option<String>,
    pub offer_tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Price {
    pub price_micros: Option<String>,
    pub currency_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct TestPurchase {
    pub purchase_time: Option<DateTime<Utc>>,
}

/// `purchases.products.get` response (one-time / consumable products).
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ProductPurchase {
    /// Consumption state: 0 = yet to be consumed, 1 = consumed.
    pub consumption_state: Option<i64>,
    /// Acknowledgement state: 0 = yet to be acknowledged, 1 = acknowledged.
    pub acknowledgement_state: Option<i64>,
    /// Product (SKU) ID.
    pub product_id: Option<String>,
    /// Purchase state: 0 = purchased, 1 = canceled.
    pub purchase_state: Option<i64>,
    /// Opaque account id set by the client (ownership check).
    pub obfuscated_external_account_id: Option<String>,
    /// Opaque profile id set by the client.
    pub obfuscated_external_profile_id: Option<String>,
    /// ISO-8601 purchase time (Google sends this as a string-wrapped integer).
    #[serde(default, deserialize_with = "flexible_millis")]
    pub purchase_time_millis: Option<i64>,
    /// Developer payload (deprecated but tolerated).
    pub developer_payload: Option<String>,
    /// Order id (Google Play order).
    pub order_id: Option<String>,
    /// Region code.
    pub region_code: Option<String>,
    /// Quantity purchased.
    pub quantity: Option<i64>,
}

/// `purchases.voidedpurchases.list` response (refund / chargeback polling).
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct VoidedPurchasesList {
    pub voided_purchases: Vec<VoidedPurchase>,
    /// Opaque page token for the next page (empty / absent when done).
    pub page_info: Option<PageInfo>,
    /// Legacy pagination token (some API versions).
    pub token_pagination: Option<TokenPagination>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct VoidedPurchase {
    /// Purchase token identifying the original purchase.
    pub purchase_token: Option<String>,
    /// Purchase type: 0 = product, 1 = subscription.
    pub purchase_type: Option<i64>,
    /// Voided time in millis since epoch (Google sends this as a string-wrapped integer).
    #[serde(default, deserialize_with = "flexible_millis")]
    pub voided_time_millis: Option<i64>,
    /// Order id associated with the voided purchase.
    pub order_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct PageInfo {
    pub total_result_count: Option<i64>,
    pub result_count_per_page: Option<i64>,
    pub start_index: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct TokenPagination {
    pub next_page_token: Option<String>,
    pub previous_page_token: Option<String>,
}
