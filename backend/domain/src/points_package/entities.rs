// Points Package domain entities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Package type distinguishing standard from promotional packages
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageType {
    Standard,
    Promotional,
}

impl fmt::Display for PackageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageType::Standard => write!(f, "standard"),
            PackageType::Promotional => write!(f, "promotional"),
        }
    }
}

impl FromStr for PackageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "standard" => Ok(PackageType::Standard),
            "promotional" => Ok(PackageType::Promotional),
            _ => Err(format!("unknown package type: {s}")),
        }
    }
}

/// Points package - a purchasable product that grants topup_credit
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointsPackage {
    pub id: Uuid,
    pub realm_id: String,
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub points: i64,
    pub price: i64,
    pub currency: String,
    pub sort_order: i32,
    pub enabled: bool,
    pub package_type: PackageType,
    pub original_price: Option<i64>,
    pub promo_start_time: Option<DateTime<Utc>>,
    pub promo_end_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PointsPackage {
    /// Returns true when this is a promotional package and the current time
    /// falls within the promotional window (None bounds treated as open-ended).
    pub fn is_promo_active(&self) -> bool {
        if self.package_type != PackageType::Promotional {
            return false;
        }
        let now = Utc::now();
        let after_start = self.promo_start_time.is_none_or(|start| now >= start);
        let before_end = self.promo_end_time.is_none_or(|end| now <= end);
        after_start && before_end
    }

    /// Returns the discount percentage when original_price is set and greater
    /// than the selling price.
    pub fn discount_percent(&self) -> Option<i32> {
        self.original_price.and_then(|orig| {
            if orig > self.price {
                Some((((orig - self.price) as f64) / (orig as f64) * 100.0).round() as i32)
            } else {
                None
            }
        })
    }

    /// Returns true when the promotional period has ended.
    pub fn is_promo_expired(&self) -> bool {
        self.promo_end_time.is_some_and(|end| Utc::now() > end)
    }
}

/// Payment provider mapping for a points package
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointsPackagePaymentProvider {
    pub id: Uuid,
    pub points_package_id: Uuid,
    pub payment_provider: String, // "wechat", "stripe", "creem"
    pub enabled: bool,
    pub external_product_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
