// =============================================================================
// Credit Bucket Scenario Tests
// =============================================================================
//
// Scenario tests for the Credit Bucket feature (design
// `.ai/design/credit-bucket.md`). Authored by the backend-test slot.
//
// - BE-T01: consume multi-pool cross-bucket spread (A6, P0 core)
//   Module: `consume_multi_pool_scenarios`
//
// - BE-T02: fulfillment + subscription lifecycle bucket routing (A8/A7, §5.3/§5.5)
//   Module: `fulfillment_subscription_lifecycle_scenarios`
//
// Subsequent authoring items (BE-T02..BE-T05) append their modules here.
//
// =============================================================================

pub mod consume_multi_pool_scenarios;
pub mod fulfillment_subscription_lifecycle_scenarios;
// BE-T03: explicit bucketId grant + registration pool resolution
// (design §5.4 / §4.3.2 / decision A5)
pub mod grant_bucket_id_registration_pool_scenarios;
// BE-T04: Bucket directory CRUD + overview + delete intercept (§4.2.1/§4.2.2/§4.2.3, A4)
pub mod bucket_directory_crud_scenarios;
// BE-T05: per-bucket query surface — wallets grouped by bucket + crossBucketTotal,
// transactions bucketId field + bucketId filter, admin cross-tenant wallets (§4.2.3 / §6.1)
pub mod bucket_query_scenarios;
// BE-T10: bucket overview/delete derived availability predicate — future-effective
// rows excluded from overview available balance, do not block delete; subscription
// check independent; clear_deletable_bucket_references_tx sweeps future-effective
// residue (design §5.1 / §6.1 / §6.3 risk P1).
pub mod bucket_overview_delete_effective_at_scenarios;
