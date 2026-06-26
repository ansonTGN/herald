use std::sync::Arc;

use uuid::Uuid;

use crate::authentication::Identity;
use crate::billing::entities::EntitlementMapping;
use crate::billing::policies::BillingPolicy;
use crate::billing::ports::BillingRepository;
use crate::common::entities::app_errors::CoreError;
use crate::common::policies::ensure_policy;
use crate::points::services::registration_pool_resolver::RegistrationPoolResolver;

/// A single price variant of a provider product.
///
/// Stripe exposes one `Price` per (product, currency, billing period) and gives
/// each a real `external_price_id`. Creem is product-level only and has no price
/// id, so `external_price_id` is `None` for Creem rows.
#[derive(Debug, Clone)]
pub struct ProviderPrice {
    pub external_price_id: Option<String>,
    pub price: Option<i64>,
    pub currency: Option<String>,
    pub billing_type: Option<String>,
    pub billing_period: Option<String>,
}

/// External provider product info returned by ProviderApiPort.
///
/// One product can carry multiple price variants (`prices`). A product with no
/// price info (Creem) carries an empty `prices` vec.
#[derive(Debug, Clone)]
pub struct ProviderProduct {
    pub external_product_id: String,
    pub name: String,
    pub description: Option<String>,
    pub prices: Vec<ProviderPrice>,
}

/// Stand-in `ProviderPrice` for products that carry no price variants at all
/// (Creem: product-level, no Stripe-style price object). Fields are all `None`,
/// which writes `external_price_id = NULL` and dedups via the `NULLS NOT
/// DISTINCT` unique constraint. A real `ProviderPrice` instance is
/// not `const`, so this is expressed as a plain static for the sync loop's
/// empty-prices fallback.
static NULL_PRICE: ProviderPrice = ProviderPrice {
    external_price_id: None,
    price: None,
    currency: None,
    billing_type: None,
    billing_period: None,
};

/// Result of a full provider sync operation
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub products_synced: usize,
    pub prices_synced: usize,
    pub sync_status: SyncStatus,
    pub error: Option<String>,
    pub partial_errors: Vec<PartialSyncError>,
}

/// Sync status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    Completed,
    Partial,
    Failed,
}

/// Partial sync error detail
#[derive(Debug, Clone)]
pub struct PartialSyncError {
    pub external_id: String,
    pub reason: String,
}

/// Port for accessing external provider APIs (Stripe, Creem, etc.)
/// Concrete implementations live in the infra layer.
pub trait ProviderApiPort: Send + Sync {
    /// Fetch all products from the provider for a given realm
    fn fetch_products(
        &self,
        realm_id: &str,
        payment_provider: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<ProviderProduct>, CoreError>> + Send + '_>,
    >;
}

/// ProviderProductSyncService - Syncs provider products into local entitlement mappings
///
/// Generic over:
/// - `R`: the billing repository (mapping upserts + lookups)
/// - `P`: the billing policy (permission gate)
/// - `A`: the external provider API (Stripe / Creem product fetch)
/// - `B`: the registration-pool bucket resolver used to bind newly-created
///   draft mappings to a valid per-realm credit bucket. `bucket_id` is NOT NULL
///   on `provider_entitlement_mappings` (commits aa6cc2da / f134dcf8 /
///   57c313ba), so a freshly-synced product with no pre-existing mapping must
///   still land in a real bucket. We reuse the existing
///   `RegistrationPoolResolver` pattern (the same port
///   `webhook_subscription_helpers::resolve_bucket_id_for_entitlement` and the
///   registration grant path rely on) to pick the realm's single registration
///   pool. There is no separate "default" / "draft" bucket concept by design.
pub struct ProviderProductSyncService<R, P, A, B>
where
    R: BillingRepository,
    P: BillingPolicy,
    A: ProviderApiPort,
    B: RegistrationPoolResolver,
{
    repository: Arc<R>,
    policy: Arc<P>,
    provider_api: Arc<A>,
    bucket_resolver: Arc<B>,
}

impl<R, P, A, B> ProviderProductSyncService<R, P, A, B>
where
    R: BillingRepository + Send + Sync,
    P: BillingPolicy,
    A: ProviderApiPort,
    B: RegistrationPoolResolver,
{
    pub fn new(
        repository: Arc<R>,
        policy: Arc<P>,
        provider_api: Arc<A>,
        bucket_resolver: Arc<B>,
    ) -> Self {
        Self {
            repository,
            policy,
            provider_api,
            bucket_resolver,
        }
    }

    /// Sync provider products into local entitlement mappings
    pub async fn sync_provider_products(
        &self,
        identity: Identity,
        realm_id: &str,
        payment_provider: &str,
    ) -> Result<SyncResult, CoreError> {
        ensure_policy(
            self.policy.can_manage_billing(identity.clone()).await,
            "Insufficient permissions to manage billing",
        )?;

        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access billing from a different realm".to_string(),
            ));
        }

        let products = self
            .provider_api
            .fetch_products(realm_id, payment_provider)
            .await?;

        let mut products_synced = 0usize;
        let mut prices_synced = 0usize;
        let mut partial_errors = Vec::new();

        for product in products {
            // A product with zero price variants still needs one mapping row
            // (Creem: product-level, no price concept). Treat the empty case as
            // a single NULL-price variant so Creem keeps producing exactly one
            // row with `external_price_id IS NULL` (the
            // NULLS NOT DISTINCT unique constraint dedups on it).
            let prices: Vec<&ProviderPrice> = if product.prices.is_empty() {
                vec![&NULL_PRICE]
            } else {
                product.prices.iter().collect()
            };

            let mut product_had_any_upsert = false;

            for price in prices {
                let external_price_id = price.external_price_id.as_deref();

                let existing = self
                    .repository
                    .find_entitlement_mapping_by_provider_product_price(
                        realm_id,
                        payment_provider,
                        &product.external_product_id,
                        external_price_id,
                    )
                    .await?;

                let (mapping_id, bucket_id, entitlement_key, draft_defaults) = match existing
                    .as_ref()
                {
                    Some(existing_mapping) => (
                        existing_mapping.id,
                        existing_mapping.bucket_id,
                        existing_mapping.entitlement_key.clone(),
                        None,
                    ),
                    None => {
                        // New provider product+price: create a draft mapping.
                        // `bucket_id` is NOT NULL (commits aa6cc2da /
                        // f134dcf8 / 57c313ba), so bind it to the realm's
                        // registration-pool bucket — the same bucket the
                        // registration/free-periodic grant path and the
                        // webhook entitlement resolver use. No
                        // "default"/"draft" bucket concept exists by design.
                        let bucket_id = self
                                .bucket_resolver
                                .resolve_registration_pool_bucket(realm_id)
                                .await
                                .map_err(|e| {
                                    tracing::error!(
                                        realm_id = %realm_id,
                                        external_product_id = %product.external_product_id,
                                        external_price_id = ?external_price_id,
                                        error = %e,
                                        "Failed to resolve registration-pool bucket during provider product sync"
                                    );
                                    e
                                })?
                                .ok_or_else(|| {
                                    // Fail loud: a realm with no registration-pool
                                    // bucket cannot accept newly-synced products.
                                    // The operator must configure a registration
                                    // pool first.
                                    CoreError::BadRequest(format!(
                                        "Cannot sync new provider product {}: realm '{}' has no registration-pool credit bucket; create one before syncing",
                                        product.external_product_id, realm_id
                                    ))
                                })?;
                        (
                            Uuid::now_v7(),
                            bucket_id,
                            draft_entitlement_key(&product.external_product_id),
                            Some(DraftDefaults::default()),
                        )
                    }
                };

                let draft = draft_defaults.unwrap_or_default();

                let mapping = EntitlementMapping {
                    id: mapping_id,
                    realm_id: realm_id.to_string(),
                    payment_provider: payment_provider.to_string(),
                    external_product_id: product.external_product_id.clone(),
                    external_price_id: price.external_price_id.clone(),
                    bucket_id,
                    entitlement_key,
                    billing_type: price
                        .billing_type
                        .as_deref()
                        .and_then(|s: &str| s.parse().ok()),
                    billing_period: price.billing_period.clone(),
                    points_per_period: draft.points_per_period,
                    grant_period_type: draft.grant_period_type,
                    validity_days: draft.validity_days,
                    grant_on_subscribe: draft.grant_on_subscribe,
                    max_periods: draft.max_periods,
                    enabled: draft.enabled,
                    provider_product_info: Some(serde_json::json!({
                        "name": product.name,
                        "description": product.description,
                        "price": price.price,
                        "currency": price.currency,
                        "billing_type": price.billing_type,
                        "billing_period": price.billing_period,
                    })),
                    synced_at: Some(chrono::Utc::now()),
                    created_at: existing
                        .as_ref()
                        .map(|m| m.created_at)
                        .unwrap_or_else(chrono::Utc::now),
                    updated_at: chrono::Utc::now(),
                };

                match self.repository.upsert_entitlement_mapping(mapping).await {
                    Ok(_) => {
                        // `prices_synced` counts price-level upsert rows:
                        // every successful price-level upsert,
                        // including disabled/draft/Creem-NULL-price rows,
                        // increments. `products_synced` counts distinct
                        // products with at least one successful upsert.
                        prices_synced += 1;
                        product_had_any_upsert = true;
                    }
                    Err(e) => {
                        let external_id = match external_price_id {
                            Some(p) => format!("{}#{}", product.external_product_id, p),
                            None => product.external_product_id.clone(),
                        };
                        partial_errors.push(PartialSyncError {
                            external_id,
                            reason: e.to_string(),
                        });
                    }
                }
            }

            if product_had_any_upsert {
                products_synced += 1;
            }
        }

        let (sync_status, error) = if partial_errors.is_empty() {
            (SyncStatus::Completed, None)
        } else if products_synced > 0 {
            (SyncStatus::Partial, None)
        } else {
            (
                SyncStatus::Failed,
                Some("All products failed to sync".to_string()),
            )
        };

        Ok(SyncResult {
            products_synced,
            prices_synced,
            sync_status,
            error,
            partial_errors,
        })
    }
}

/// Default field values for a newly-created (draft) entitlement mapping.
///
/// Drafts are unconfigured: disabled, no entitlement key, no grant policy.
/// The admin configures them via PATCH `/entitlement-mappings/{id}` (the
/// `UpdateEntitlementMappingRequest` overrides every field here). Matches the
/// "unconfigured mapping" shape the create/update handlers use as their
/// baseline.
#[derive(Debug, Clone, Default)]
struct DraftDefaults {
    points_per_period: Option<i64>,
    grant_period_type: Option<String>,
    validity_days: Option<i64>,
    grant_on_subscribe: bool,
    max_periods: Option<i64>,
    enabled: bool,
}

/// Build a deterministic placeholder `entitlement_key` for a freshly-synced
/// provider product that has not yet been configured by an operator.
///
/// Why this exists: `provider_entitlement_mappings.entitlement_key` has a
/// `CHECK (entitlement_key ~ '^[a-z0-9-]{1,64}$')` constraint (migration
/// `20260607_product_reduce.sql`) that rejects the empty string, so a draft
/// insert must already carry a key that satisfies the regex. The placeholder
/// is derived from the product's `external_product_id` so it is:
/// - stable across re-syncs (idempotent: the same external id yields the same
///   key, and dedup is by `(realm_id, provider, external_product_id,
///   external_price_id)` via `find_entitlement_mapping_by_provider_product_price`,
///   so this never creates a duplicate row),
/// - never consumed by `resolve_bucket_id_for_entitlement`, because drafts are
///   inserted with `enabled=false` and the resolver filters on `enabled`,
/// - overwritten by the operator's PATCH with the real Herald entitlement key
///   (e.g. `herald-live-creem-entitlement`) before the mapping is enabled.
///
/// Algorithm: lowercase, replace every non-[a-z0-9] char with `-`, collapse
/// runs of `-`, trim leading/trailing `-`, cap at 64 chars, fall back to
/// `"draft"` if the result is empty (e.g. the external id was all symbols).
fn draft_entitlement_key(external_product_id: &str) -> String {
    let mut slug = String::with_capacity(external_product_id.len());
    let mut prev_was_dash = false;
    for ch in external_product_id.chars() {
        // Lowercase ASCII letters stay (as lowercase); ASCII digits stay.
        // Every other char (uppercase letters are folded to lowercase first,
        // so they never reach the separator branch) becomes a single `-`,
        // collapsing consecutive separators.
        if ch.is_ascii_uppercase() {
            slug.push(ch.to_ascii_lowercase());
            prev_was_dash = false;
        } else if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            slug.push(ch);
            prev_was_dash = false;
        } else if !prev_was_dash {
            slug.push('-');
            prev_was_dash = true;
        }
        if slug.len() >= 64 {
            break;
        }
    }
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "draft".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::draft_entitlement_key;

    /// Mirrors the DB CHECK constraint `entitlement_key ~ '^[a-z0-9-]{1,64}$'`
    /// (migration `20260607_product_reduce.sql`) and the char-class validation
    /// the PATCH handler applies (`entitlement_mapping_handlers.rs`).
    fn satisfies_check(key: &str) -> bool {
        !key.is_empty()
            && key.len() <= 64
            && key
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    }

    /// The draft key MUST satisfy the DB CHECK constraint. A regression here is
    /// exactly what broke the demo sync (empty-string insert rejected by the
    /// CHECK). This test fails the moment the slug stops matching the regex.
    #[test]
    fn draft_entitlement_key_always_satisfies_check_constraint() {
        let cases = [
            "prod_abc123",
            "PROD-ABC-123",
            "creem::monthly::herald",
            "x",
            &"a".repeat(200),
            "___",
            "UPPER/CASE/ID",
            "12345",
            "café-résumé",
            "",
        ];
        for input in cases {
            let key = draft_entitlement_key(input);
            assert!(
                satisfies_check(&key),
                "draft key {key:?} derived from {input:?} violates CHECK ^[a-z0-9-]{{1,64}}$"
            );
        }
    }

    /// Re-sync must produce the SAME draft key for the SAME external id, so the
    /// existing dedup-by-provider-product path updates the row in place instead
    /// of creating a duplicate. Idempotency is the load-bearing property for
    /// the placeholder approach to stay collision-free.
    #[test]
    fn draft_entitlement_key_is_deterministic_across_calls() {
        let input = "Creem_Prod_Herald_Monthly_2026";
        assert_eq!(draft_entitlement_key(input), draft_entitlement_key(input));
    }

    /// Sanity-check the normalization for a representative external id.
    #[test]
    fn draft_entitlement_key_normalizes_realistic_external_id() {
        assert_eq!(
            draft_entitlement_key("prod_HeraldLive_001"),
            "prod-heraldlive-001"
        );
    }

    /// An external id made entirely of separator chars must still yield a
    /// valid (non-empty) key — the `"draft"` fallback — so the CHECK never
    /// rejects the insert.
    #[test]
    fn draft_entitlement_key_falls_back_when_input_is_all_separators() {
        assert_eq!(draft_entitlement_key("___"), "draft");
        assert_eq!(draft_entitlement_key(""), "draft");
    }

    /// The slug must be capped at 64 chars to satisfy the `{1,64}` bound.
    #[test]
    fn draft_entitlement_key_caps_at_64_chars() {
        let key = draft_entitlement_key(&"a".repeat(200));
        assert!(key.len() <= 64, "slug length {} exceeds 64", key.len());
    }
}

// Note on tests (AGENTS.md Rule 9, backend testing guide):
// The sync-creates-draft-mappings behavior is verified at the integration
// layer via `entitlement_mapping_crud_scenarios.rs` (which wires the real
// `PostgresBillingRepository` through `schema_test_context`). A domain-crate
// unit test would require mocking the ~30-method `BillingRepository` trait
// (RPITIT, not dyn-safe) plus constructing a full `Identity::User` — that is
// the "mechanical stub surface" the testing guide explicitly disallows, and it
// would not encode intent beyond what the existing scenario coverage does.
