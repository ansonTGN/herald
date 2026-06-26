// Registration Pool Resolver
//
// Per design (credit-bucket): the target Bucket for registration
// and free periodic system grants is the single Bucket in the Realm flagged with
// `receives_registration_credits = true` (at most one per Realm, enforced by the
// partial unique index `uq_credit_buckets_registration_pool`).
//
// This trait defines only the domain port. The DB lookup implementation lives in
// the infra layer. `RegistrationService` consumes this port to
// resolve the registration pool Bucket before granting.

use std::future::Future;

use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;

/// Resolves the registration-pool Bucket for a Realm.
///
/// Returns:
/// - `Ok(Some(bucket_id))` when exactly one Bucket in the Realm is flagged as the
///   registration credits receiver.
/// - `Ok(None)` when no such Bucket exists; callers must fail-safe (do not grant,
///   do not silently fall back to any implicit pool).
pub trait RegistrationPoolResolver: Send + Sync {
    fn resolve_registration_pool_bucket(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = Result<Option<Uuid>, CoreError>> + Send;
}
