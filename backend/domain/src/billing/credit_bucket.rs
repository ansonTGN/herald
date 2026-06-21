// Credit Bucket domain entities
//
// Per design `.ai/design/credit-bucket.md` §4.2.2/§4.2.3: the Credit Bucket is the
// unit of points-pool isolation. This module defines the domain DTOs returned by the
// infra-layer bucket directory CRUD (BE-D07) and consumed by api-billing handlers
// (BE-D08/BE-D09). There is intentionally NO `is_default` field (design A4: no
// default bucket concept). The registration-pool flag `receives_registration_credits`
// is the only system-grant routing signal and is enforced unique-per-realm by the
// partial unique index `uq_credit_buckets_registration_pool`.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;

/// Credit Bucket catalog row.
///
/// Mirrors `credit_buckets` table columns (minus audit timestamps which are not
/// surfaced to API consumers in §4.2.3 list/shape).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditBucket {
    pub id: Uuid,
    pub realm_id: String,
    /// Matches `^[a-z0-9-]{1,64}$` (DB CHECK constraint `chk_credit_buckets_key`).
    pub bucket_key: String,
    pub name: String,
    pub description: Option<String>,
    pub display_order: i32,
    /// Whether this bucket is the Realm's single registration/free-periodic grant
    /// receiver (partial unique index `uq_credit_buckets_registration_pool`).
    pub receives_registration_credits: bool,
    pub enabled: bool,
}

/// Input for creating a Credit Bucket.
///
/// The coverage set (`client_app_ids`) MUST be non-empty (handler enforces 400 on
/// empty; design §4.2.2). `entitlement_mapping_ids` is optional: when present, the
/// listed mappings are re-attached to this bucket (their `bucket_id` is set).
#[derive(Debug, Clone)]
pub struct CreateCreditBucketInput {
    pub realm_id: String,
    pub bucket_key: String,
    pub name: String,
    pub description: Option<String>,
    pub display_order: i32,
    pub receives_registration_credits: bool,
    pub enabled: bool,
    /// Coverage set — at least one entry required.
    pub client_app_ids: Vec<Uuid>,
    /// Optional mappings to attach (set their `bucket_id` to this bucket).
    pub entitlement_mapping_ids: Vec<Uuid>,
}

/// Input for updating a Credit Bucket.
///
/// All provided fields replace the stored state (coverage set and attached mappings
/// are fully replaced, not merged — design A7 "coverage-set changes do not
/// retroactively reclaim balances" still holds: only future routing is affected).
#[derive(Debug, Clone)]
pub struct UpdateCreditBucketInput {
    pub realm_id: String,
    pub bucket_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub display_order: i32,
    pub receives_registration_credits: bool,
    pub enabled: bool,
    /// Replacement coverage set — at least one entry required.
    pub client_app_ids: Vec<Uuid>,
    /// Replacement attached-mapping set (may be empty).
    pub entitlement_mapping_ids: Vec<Uuid>,
}

/// Detail view: bucket plus explicit client app ids plus attached mapping ids.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditBucketDetail {
    #[serde(flatten)]
    pub bucket: CreditBucket,
    pub client_app_ids: Vec<Uuid>,
    pub entitlement_mapping_ids: Vec<Uuid>,
}

/// List-item view with aggregate counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditBucketListItem {
    #[serde(flatten)]
    pub bucket: CreditBucket,
    pub covered_client_app_count: i64,
    pub entitlement_mapping_count: i64,
}

/// Per-credit-type balance totals for a single bucket (overview / wallets).
///
/// Keys follow the `credit_type` DB enum values; missing types default to 0.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BucketByCreditType {
    pub topup: i64,
    pub subscription: i64,
    pub registration: i64,
    pub free_periodic: i64,
    pub granted: i64,
}

impl BucketByCreditType {
    pub fn total(&self) -> i64 {
        self.topup
            .saturating_add(self.subscription)
            .saturating_add(self.registration)
            .saturating_add(self.free_periodic)
            .saturating_add(self.granted)
    }
}

/// One row in the overview matrix (per bucket × credit type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditBucketOverviewRow {
    pub bucket_id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub by_credit_type: BucketByCreditType,
    pub bucket_total: i64,
}

/// Result of `list_bucket_overview`: rows per bucket (residual rows kept for
/// disabled buckets) + a SEPARATE grand total across all buckets.
///
/// `grand_total` is intentionally a sibling field of `rows`, NOT mixed into the
/// rows array (design §4.2.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditBucketOverview {
    pub rows: Vec<CreditBucketOverviewRow>,
    pub grand_total: BucketByCreditType,
}

/// Credit Bucket directory operation errors.
///
/// These carry structured bodies so api-billing handlers (BE-D08/BE-D09) can emit
/// the exact §4.2.3 error contracts. Convertible to `CoreError` for uniform
/// propagation through the repository layer; handlers translate back via
/// `ApiError::conflict_json` with the structured payload.
#[derive(Debug, Clone, Error)]
pub enum CreditBucketError {
    /// `bucket_key` collides with another bucket in the same realm (unique index
    /// `uq_credit_buckets_realm_key`). HTTP 400 `bucket_key_duplicate`.
    #[error("bucket_key_duplicate: bucketKey already exists in realm {realm_id}")]
    BucketKeyDuplicate { realm_id: String },

    /// `receives_registration_credits = true` collides with another bucket in the
    /// same realm (partial unique index `uq_credit_buckets_registration_pool`).
    /// HTTP 409 `registration_pool_conflict`.
    #[error(
        "registration pool conflict: another bucket in realm {realm_id} already receives registration credits"
    )]
    RegistrationPoolConflict { realm_id: String },

    /// Delete refused: bucket is in use by in-flight subscriptions or wallets with
    /// remaining balance. HTTP 409 `bucket_in_use` with structured body.
    #[error(
        "credit bucket {bucket_id} is in use ({active_subscriptions} active subscriptions, {holders_with_balance} wallets with balance)"
    )]
    BucketInUse {
        bucket_id: Uuid,
        active_subscriptions: i64,
        holders_with_balance: i64,
    },

    /// Update refused: the PUT's `entitlement_mapping_ids` would remove one or
    /// more mappings currently attached to this bucket. `provider_entitlement_mappings.
    /// bucket_id` is NOT NULL (commit `aa6cc2da`) and there is no default bucket
    /// (design A4), so a detached mapping has no legal home — removal is rejected.
    /// To move a mapping out, assign it to another bucket via that bucket's PUT.
    /// HTTP 400 `bucket_orphan_mapping`.
    #[error(
        "bucket_orphan_mapping: removing mappings {orphan_mapping_ids:?} from bucket {bucket_id} would leave them unassigned (bucket_id is NOT NULL)"
    )]
    BucketOrphanMapping {
        bucket_id: Uuid,
        orphan_mapping_ids: Vec<Uuid>,
    },

    /// Transparent passthrough for non-structured errors (not-found, DB errors).
    /// Handlers map this back to the wrapped `CoreError` for status selection.
    #[error(transparent)]
    Other(#[from] CoreError),
}

impl From<CreditBucketError> for CoreError {
    fn from(err: CreditBucketError) -> Self {
        match err {
            CreditBucketError::Other(inner) => inner,
            // BucketKeyDuplicate is a 400 (bad request), not a 409 conflict —
            // mirror api-billing's structured-body mapping for generic propagation.
            CreditBucketError::BucketKeyDuplicate { realm_id: _ } => {
                CoreError::BadRequest(err.to_string())
            }
            // BucketOrphanMapping is a 400 (bad request), not a 409 conflict —
            // must precede the `other` catch-all which maps to Conflict.
            CreditBucketError::BucketOrphanMapping {
                bucket_id: _,
                orphan_mapping_ids: _,
            } => CoreError::BadRequest(err.to_string()),
            // Preserve the structured message; handlers that need the structured
            // body should match on CreditBucketError directly before converting.
            other => CoreError::Conflict(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `BucketByCreditType::total()` must sum all five credit-type balances.
    /// This is the value surfaced as `bucketTotal` and aggregated into
    /// `grand_total` — a wrong sum would mislead admin overview / user balances.
    #[test]
    fn bucket_by_credit_type_total_sums_all_types() {
        let by_type = BucketByCreditType {
            topup: 100,
            subscription: 200,
            registration: 50,
            free_periodic: 30,
            granted: 20,
        };
        assert_eq!(by_type.total(), 400);

        // Default (all zero) must report zero, not None/panic.
        assert_eq!(BucketByCreditType::default().total(), 0);
    }

    /// Saturating add guards against i64 overflow when summing across many wallets
    /// in `list_bucket_overview`. A naive `+` would panic in debug / wrap in
    /// release; saturating caps at i64::MAX.
    #[test]
    fn bucket_by_credit_type_total_saturates_on_overflow() {
        let by_type = BucketByCreditType {
            topup: i64::MAX - 10,
            subscription: 100,
            registration: 0,
            free_periodic: 0,
            granted: 0,
        };
        assert_eq!(by_type.total(), i64::MAX);
    }

    /// Design §4.2.3 invariant: `grand_total` is a SEPARATE field of
    /// `CreditBucketOverview`, NOT a synthesized extra row appended to `rows`.
    /// Building an overview must not mutate the rows vector with the total.
    #[test]
    fn overview_keeps_grand_total_separate_from_rows() {
        let overview = CreditBucketOverview {
            rows: vec![CreditBucketOverviewRow {
                bucket_id: Uuid::nil(),
                name: "bucket-a".into(),
                enabled: true,
                by_credit_type: BucketByCreditType {
                    topup: 10,
                    ..Default::default()
                },
                bucket_total: 10,
            }],
            grand_total: BucketByCreditType {
                topup: 10,
                ..Default::default()
            },
        };

        // Exactly one row (the bucket), grand_total is its own field.
        assert_eq!(overview.rows.len(), 1);
        assert_eq!(overview.grand_total.total(), 10);
        assert_eq!(
            overview.rows.last().unwrap().bucket_id,
            Uuid::nil(),
            "no grand-total row appended"
        );
    }

    /// `CreditBucketError::Other` preserves the wrapped `CoreError` on round-trip
    /// through `From<CreditBucketError> for CoreError`, so not-found / DB errors
    /// retain their original status mapping when propagated generically.
    #[test]
    fn credit_bucket_error_other_round_trips_core_error() {
        let original = CoreError::NotFound;
        let bucket_err: CreditBucketError = original.clone().into();
        let back: CoreError = bucket_err.into();
        assert_eq!(back, original);
    }

    /// Structured variants must NOT collapse to `CoreError::NotFound` — they are
    /// conflicts (HTTP 409) so handlers can map them to `registration_pool_conflict`
    /// / `bucket_in_use` bodies.
    #[test]
    fn structured_bucket_errors_map_to_conflict_status() {
        let conflict = CreditBucketError::RegistrationPoolConflict {
            realm_id: "r1".into(),
        };
        let core: CoreError = conflict.into();
        assert!(matches!(core, CoreError::Conflict(_)));

        let in_use = CreditBucketError::BucketInUse {
            bucket_id: Uuid::nil(),
            active_subscriptions: 1,
            holders_with_balance: 2,
        };
        let core: CoreError = in_use.into();
        assert!(matches!(core, CoreError::Conflict(_)));
    }
}
