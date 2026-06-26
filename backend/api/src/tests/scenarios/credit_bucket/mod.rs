// =============================================================================
// Credit Bucket Scenario Tests
// =============================================================================
//
// Scenario tests for the Credit Bucket feature (design
// `.ai/design/credit-bucket.md`). Authored by the backend-test slot.
//
// - consume multi-pool cross-bucket spread (P0 core)
//   Module: `consume_multi_pool_scenarios`
//
// - fulfillment + subscription lifecycle bucket routing
//   Module: `fulfillment_subscription_lifecycle_scenarios`
//
// Subsequent authoring items append their modules here.
//
// =============================================================================

pub mod consume_multi_pool_scenarios;
pub mod fulfillment_subscription_lifecycle_scenarios;
// explicit bucketId grant + registration pool resolution
// (design `.ai/design/credit-bucket.md`)
pub mod grant_bucket_id_registration_pool_scenarios;
// Bucket directory CRUD + overview + delete intercept
pub mod bucket_directory_crud_scenarios;
// per-bucket query surface — wallets grouped by bucket + crossBucketTotal,
// transactions bucketId field + bucketId filter, admin cross-tenant wallets
pub mod bucket_query_scenarios;
// bucket overview/delete derived availability predicate — future-effective
// rows excluded from overview available balance, do not block delete; subscription
// check independent; clear_deletable_bucket_references_tx sweeps future-effective
// residue (risk P1).
pub mod bucket_overview_delete_effective_at_scenarios;
