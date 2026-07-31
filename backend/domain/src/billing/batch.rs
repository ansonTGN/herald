//! Batch price-mapping write types.
//!
//! Single-transaction upsert of ALL price rows for a product; any row violating
//! the active-subscription lock rolls back the whole transaction and surfaces a
//! structured 409.

use uuid::Uuid;

use crate::billing::entities::EntitlementMapping;
use crate::points::RuleUpsert;

/// One price-mapping row within a batch save (domain input mirror of the API
/// `PriceMappingUpdate` DTO). Fields are `Option<T>` so the caller can
/// distinguish "leave unchanged" (`None`) from "set" (`Some`).
#[derive(Debug, Clone)]
pub struct PriceMappingUpdateInput {
    pub mapping_id: Uuid,
    pub billing_type: Option<String>,
    pub enabled: Option<bool>,
    /// Points distribution rules owned by this mapping (upsert set, design
    /// §4.2.2). `None` ⟺ leave the existing rule set untouched;
    /// `Some(rules)` ⟺ upsert the given rules under this mapping (rules in the
    /// set with `id = None` are created, `id = Some(existing)` are updated,
    /// rules absent from the set are left untouched — disabling requires
    /// explicit `enabled = false`). Non-empty / present triggers the
    /// `points.manage` credit-field permission gate.
    pub point_rules: Option<Vec<RuleUpsert>>,
    /// Role-grant config dimension. Same 3-state semantics as
    /// `point_rules`: `None` ⟺ leave unchanged; `Some(vec![])` ⟺ clear (no
    /// role grant — pure points / payment record); `Some(non-empty)` ⟺ set.
    /// Non-empty triggers realm-membership validation (all role IDs must belong
    /// to the mapping's realm); does NOT require `roles.manage` (configuring a
    /// mapping ≠ assigning permissions).
    pub granted_role_ids: Option<Vec<Uuid>>,
}

/// Request payload for a batch price-mapping save scoped to one product.
#[derive(Debug, Clone)]
pub struct BatchUpdateMappingsInput {
    pub realm_id: String,
    pub payment_provider: String,
    pub external_product_id: String,
    pub updates: Vec<PriceMappingUpdateInput>,
}

/// Result of a successful batch save: the count of rows written and the
/// product's full latest set of price rows.
#[derive(Debug, Clone)]
pub struct BatchUpdateResult {
    pub saved: u32,
    pub prices: Vec<EntitlementMapping>,
}

/// Structured errors from a batch price-mapping save. The handler maps these to
/// HTTP statuses (400 / 409); infra/database failures stay `Other(CoreError)`.
#[derive(Debug, thiserror::Error)]
pub enum BatchMappingError {
    /// A `mapping_id` in the request does not belong to the
    /// `(realm, provider, product)` group — cross-product/realm tampering
    /// attempt. → 400.
    #[error(
        "mapping {mapping_id} does not belong to provider '{provider}' product '{product}' in this realm"
    )]
    MappingNotInGroup {
        mapping_id: Uuid,
        provider: String,
        product: String,
    },

    /// One or more rows transition `enabled` true→false while their mapping has
    /// active (access-granting) subscriptions. The WHOLE transaction is rolled
    /// back. → 409 with `{ activeSubscriptions }`.
    #[error(
        "cannot disable mapping(s) with active subscriptions (provider '{provider}', product '{product}')"
    )]
    ActiveSubscriptionLock {
        provider: String,
        product: String,
        active_subscriptions: i64,
    },

    /// A `granted_role_ids` entry does not belong to the mapping's realm —
    /// cross-realm role_id in the config. → 400 with
    /// `{ code: "role_not_in_realm", roleId, realmId }` (design §4.2.2 / §5.2).
    #[error("role {role_id} does not belong to realm {realm_id}")]
    RoleNotInRealm { role_id: Uuid, realm_id: String },

    /// Infrastructure / unexpected failure. Preserves the wrapped `CoreError`
    /// status mapping (404 / 500 / …).
    #[error(transparent)]
    Other(#[from] crate::common::entities::app_errors::CoreError),
}

/// Validate that every role ID in `role_ids` belongs to `realm_id`.
///
/// Reuses the existing role-read capability (`RolePolicyRepository::get_roles_by_ids`)
/// rather than introducing a new trait. Returns the first role that either does
/// not exist or belongs to a different realm as `BatchMappingError::RoleNotInRealm`
/// (design §5.2). A role that simply does not exist is also treated as
/// "not in realm" — the caller cannot configure a grant for an unknown role.
///
/// `role_ids` is expected to be non-empty; an empty slice is a no-op `Ok(())`.
pub async fn validate_granted_role_ids<R>(
    realm_id: &str,
    role_ids: &[Uuid],
    role_repo: &R,
) -> Result<(), BatchMappingError>
where
    R: crate::user::admin_ports::RolePolicyRepository + ?Sized,
{
    use std::collections::HashSet;

    if role_ids.is_empty() {
        return Ok(());
    }
    let roles = role_repo
        .get_roles_by_ids(role_ids)
        .await
        .map_err(|e| BatchMappingError::Other(e.into()))?;

    let realm_ok: HashSet<Uuid> = roles
        .iter()
        .filter(|r| r.realm_id == realm_id)
        .map(|r| r.id)
        .collect();

    // First offender: a requested id that is either entirely missing or belongs
    // to another realm. Iterate in request order for a deterministic message.
    for id in role_ids {
        if !realm_ok.contains(id) {
            return Err(BatchMappingError::RoleNotInRealm {
                role_id: *id,
                realm_id: realm_id.to_string(),
            });
        }
    }
    Ok(())
}
