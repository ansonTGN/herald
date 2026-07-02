//! Realm-level invoice eligibility evaluation.
//!
//! Provides a backend-computed eligibility *result* that the frontend consumes
//! to gate Create/Apply invoice buttons BEFORE submit (policy=none, missing
//! seller config), instead of relying on post-submit backend rejection.
//!
//! Regular users consume this result; they do NOT read admin config/policy
//! APIs directly. The realm-level evaluation is wired into `feature-availability`
//! so no separate realm-level endpoint is added.
//!
//! ## Single home for all eligibility judgments
//!
//! Both the realm-level judgment (`evaluate_realm_invoice_eligibility`) and the
//! per-resource judgment (`determine_invoice_apply_route`) live here so the
//! read-path rules do not diverge from the write-path validators
//! (`validate_not_creem_mor`, `validate_invoice_policy_allows_creation`) in
//! `herald_core::domain::billing::invoice_service`. The pure
//! `determine_invoice_apply_route` is the only place encoding the
//! "External-if-synced" decision; the per-resource endpoint resolves the facts
//! (ownership/provider/policy/seller/external) and delegates to it.

use herald_api_base::application::http::server::api_entities::ApiError;
use herald_api_base::application::http::state::AppState;

use crate::invoice_handlers::get_invoice_policy;

/// Realm-level invoice eligibility summary.
///
/// Surfaced to regular users via `feature-availability.invoiceEligibility`.
/// The `reason` field is `None` when everything is configured and allowed.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceEligibilitySummary {
    /// Whether the realm has an invoice seller config saved.
    pub has_seller_config: bool,
    /// Invoice policy: "provider_first" | "manual_only" | "none".
    /// Defaults to "provider_first" when unconfigured (same as mutation paths).
    pub policy: String,
    /// Realm-level: manual invoice creation is allowed (`policy != "none"`).
    pub can_create_manual_invoice: bool,
    /// Realm-level: applying for an invoice is allowed. Equal to
    /// `can_create_manual_invoice`; per-resource route checks are a later phase.
    pub can_apply_invoice: bool,
    /// Human-readable reason when eligibility is limited, else `None`.
    pub reason: Option<String>,
}

/// Evaluate realm-level invoice eligibility.
///
/// Reuses the policy-reading logic from `invoice_handlers::get_invoice_policy`
/// (no duplicated SQL/realm_config read) and the seller-config fact already
/// loaded by `feature-availability` (no second seller-config query).
///
/// Reason rules:
/// - `policy == "none"`        => "Realm does not issue Herald invoices"
/// - `!has_seller_config`      => "Seller information not configured"
/// - otherwise                 => `None`
pub async fn evaluate_realm_invoice_eligibility(
    state: &AppState,
    realm_id: &str,
    has_seller_config: bool,
) -> Result<InvoiceEligibilitySummary, ApiError> {
    let policy_config = get_invoice_policy(state, realm_id).await?;
    let policy = policy_config.policy.clone();

    let can_create_manual_invoice = policy != "none";
    // Realm-level: applying mirrors manual-creation eligibility. Per-resource
    // route checks (provider_first + provider capability) are a later phase.
    let can_apply_invoice = can_create_manual_invoice;

    let reason = if policy == "none" {
        Some("Realm does not issue Herald invoices".to_string())
    } else if !has_seller_config {
        Some("Seller information not configured".to_string())
    } else {
        None
    };

    Ok(InvoiceEligibilitySummary {
        has_seller_config,
        policy,
        can_create_manual_invoice,
        can_apply_invoice,
        reason,
    })
}

// =============================================================================
// Per-resource apply-eligibility (Phase B of P0-2)
// =============================================================================
//
// See `.ai/future/invoice_ux.md` P0-2 and the "External-if-synced" decision in
// the design. The per-resource endpoint (GET
// /api/bill/{realmId}/my/invoices/apply-eligibility) resolves the facts and
// delegates here. Keeping this pure makes the rules trivially unit-testable and
// guarantees the read-path and write-path (`apply_invoice` →
// `validate_invoice_creation_policy` + seller-config check) stay in lockstep.

/// Verdict returned by [`determine_invoice_apply_route`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRouteVerdict {
    /// `"external_provider" | "manual_fallback" | "disabled"`.
    pub route: String,
    pub can_apply: bool,
    pub reason: Option<String>,
}

/// Decide the per-resource invoice apply route from resolved facts.
///
/// Inputs are exactly the facts the endpoint resolves before calling this:
/// - `provider`              — resolved `payment_provider` (Stripe/Shopify/WeChat/Creem/None).
/// - `policy`                — `provider_first` / `manual_only` / `none`.
/// - `has_seller_config`     — `find_seller_config(realm)` returned `Some`.
/// - `has_external_invoice`  — an invoice with `source = external_sync` exists
///   for this resource's id (matched on `payment_attempt_id` or
///   `subscription_id`).
///
/// Rules are mutually exclusive and evaluated in this order:
///
/// 1. `policy == "none"`     => `disabled` (Herald invoices off)
/// 2. `provider == Some("creem")` => `disabled` (Creem acts as MoR; mirrors
///    `validate_not_creem_mor` in the write path)
/// 3. `provider == Some("stripe")` => `external_provider` (Stripe invoices are
///    pushed via webhook; users must never apply manually. Read-only regardless
///    of whether the webhook has landed yet, so users are pointed to
///    "My Invoices" instead of a manual apply form.)
/// 4. `!has_seller_config`   => `disabled` (mirrors the `apply_invoice` 400 path)
/// 5. `has_external_invoice` => `external_provider` (read-only — a provider
///    invoice already exists; do not offer a duplicate Herald invoice)
/// 6. otherwise              => `manual_fallback, canApply=true`
///    (Shopify/WeChat/no provider WITHOUT an external invoice still permit a
///    manual Herald invoice — the manual fallback.)
pub(crate) fn determine_invoice_apply_route(
    provider: Option<&str>,
    policy: &str,
    has_seller_config: bool,
    has_external_invoice: bool,
) -> ApplyRouteVerdict {
    // Rule 1: realm policy disables Herald invoices entirely.
    if policy == "none" {
        return ApplyRouteVerdict {
            route: "disabled".to_string(),
            can_apply: false,
            reason: Some("Invoice creation is disabled by policy".to_string()),
        };
    }

    // Rule 2: Creem is Merchant of Record — Herald must not create a competing
    // invoice. Mirrors `validate_not_creem_mor` in the write path.
    if provider == Some("creem") {
        return ApplyRouteVerdict {
            route: "disabled".to_string(),
            can_apply: false,
            reason: Some(
                "Creem transactions are managed by Creem as Merchant of Record".to_string(),
            ),
        };
    }

    // Rule 3: Stripe invoices are pushed via webhook — users must never apply
    // manually. Surface as read-only external_provider regardless of whether
    // the webhook has landed yet; the frontend shows "Managed by Stripe — see
    // My Invoices." and points users to their invoice list.
    if provider == Some("stripe") {
        return ApplyRouteVerdict {
            route: "external_provider".to_string(),
            can_apply: false,
            reason: None,
        };
    }

    // Rule 4: no seller info configured — mirrors the `apply_invoice` 400 path.
    if !has_seller_config {
        return ApplyRouteVerdict {
            route: "disabled".to_string(),
            can_apply: false,
            reason: Some(
                "No seller configuration found for this realm. An admin must configure seller info first."
                    .to_string(),
            ),
        };
    }

    // Rule 5: an externally-synced invoice already exists for this resource —
    // read-only. Do not offer a duplicate Herald invoice.
    if has_external_invoice {
        let provider_label = provider.unwrap_or("the provider");
        return ApplyRouteVerdict {
            route: "external_provider".to_string(),
            can_apply: false,
            reason: Some(format!(
                "An invoice from {} already exists for this resource.",
                provider_label
            )),
        };
    }

    // Rule 6: non-MoR provider (Shopify/WeChat) or no provider, with seller
    // config and no external invoice. Manual fallback remains available.
    ApplyRouteVerdict {
        route: "manual_fallback".to_string(),
        can_apply: true,
        reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Boundary tests for `determine_invoice_apply_route` (Rule 9 — encode WHY).
    // These guard against divergence between the read-path rules and the
    // write-path validators. If anyone tightens or loosens the rules here,
    // `validate_invoice_creation_policy` / `apply_invoice` must be re-checked.

    #[test]
    fn policy_none_disables_apply() {
        let v = determine_invoice_apply_route(Some("stripe"), "none", true, false);
        assert_eq!(v.route, "disabled");
        assert!(!v.can_apply);
        assert!(v.reason.as_deref().unwrap().contains("disabled by policy"));
    }

    #[test]
    fn creem_provider_disables_apply_regardless_of_policy() {
        // Even with seller config and no external invoice, Creem is MoR.
        for policy in ["provider_first", "manual_only"] {
            let v = determine_invoice_apply_route(Some("creem"), policy, true, false);
            assert_eq!(v.route, "disabled", "policy={}", policy);
            assert!(!v.can_apply);
            assert!(v.reason.as_deref().unwrap().contains("Merchant of Record"));
        }
    }

    #[test]
    fn missing_seller_config_disables_apply() {
        // Use no provider so the verdict reaches the seller-config rule. (A
        // Stripe provider would short-circuit to external_provider earlier.)
        let v = determine_invoice_apply_route(None, "provider_first", false, false);
        assert_eq!(v.route, "disabled");
        assert!(!v.can_apply);
        assert!(v.reason.as_deref().unwrap().contains("seller"));
    }

    #[test]
    fn stripe_is_external_provider_regardless_of_external_invoice() {
        // CRITICAL invariant: Stripe invoices are pushed via webhook, so users
        // must NEVER be offered a manual apply — regardless of whether the
        // webhook has landed yet. Stripe always routes to external_provider
        // (read-only) for any non-`none` policy, with or without an existing
        // external invoice.
        for policy in ["provider_first", "manual_only"] {
            for has_external in [false, true] {
                let v = determine_invoice_apply_route(Some("stripe"), policy, true, has_external);
                assert_eq!(
                    v.route, "external_provider",
                    "policy={policy} has_external={has_external}"
                );
                assert!(!v.can_apply, "policy={policy} has_external={has_external}");
                // The frontend renders the generic "Managed by Stripe — see
                // My Invoices." text from the route+provider; reason is null.
                assert!(
                    v.reason.is_none(),
                    "policy={policy} has_external={has_external}"
                );
            }
        }
    }

    #[test]
    fn no_provider_with_seller_is_manual_fallback() {
        // manual_only + no provider + seller configured => manual fallback.
        let v = determine_invoice_apply_route(None, "manual_only", true, false);
        assert_eq!(v.route, "manual_fallback");
        assert!(v.can_apply);
    }

    #[test]
    fn external_provider_label_falls_back_when_provider_none() {
        // Resource somehow has an external invoice but no resolved provider
        // (should not happen in practice, but the verdict must not panic).
        let v = determine_invoice_apply_route(None, "provider_first", true, true);
        assert_eq!(v.route, "external_provider");
        assert!(v.reason.as_deref().unwrap().contains("the provider"));
    }

    #[test]
    fn policy_none_takes_precedence_over_creem() {
        // Rule 1 is checked before Rule 2: policy=none + creem => disabled by
        // policy (either reason is correct; precedence is the invariant).
        let v = determine_invoice_apply_route(Some("creem"), "none", true, false);
        assert_eq!(v.route, "disabled");
        assert!(!v.can_apply);
    }
}
