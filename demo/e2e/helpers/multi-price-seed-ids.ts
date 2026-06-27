/**
 * Multi-Price Demo Seed Ids (support-multiple-price)
 *
 * Authoritative constants for the multi-price product seeded by
 * `scripts/lib/demo_seed.py::_ensure_multi_price_demo_data`. Consumers reference
 * these ids so the demo never hardcodes selector suffixes or API targets inline.
 *
 * Seed shape (load-bearing):
 * - ONE Stripe product `prod_stripe_multi_pro` with TWO price rows.
 * - Both price rows SHARE `entitlement_key = 'pro-plan'` (US-EM-008: monthly
 *   1000 / annual 12000 under one shared key is the load-bearing invariant — the
 *   webhook grant MUST resolve price-level strategy, not key-level, otherwise
 *   12000 == 1000 collapses).
 * - Monthly: billing_type `recurring`, billing_period `month`, points_per_period 1000.
 * - Annual:  billing_type `recurring`, billing_period `year`,  points_per_period 12000.
 * - Both enabled, both assigned to the realm-001 registration-pool bucket
 *   (`_default_bucket_id(POINTS_REALM_ID)`).
 *
 * These ids are placeholders (not real Stripe price ids). Non-live demos drive
 * them via the seeded catalog + the rewritten UI; LIVE demos create their own
 * real Stripe prices and do not depend on these constants.
 *
 * LOUD NOTE — `external_price_id` vs `mapping_id` testid fallback:
 * The master-detail price row + purchase price-card testids fall back to the
 * mapping id when `external_price_id` is NULL (Creem). For these
 * Stripe rows `external_price_id` is NON-NULL, so the suffix IS the price id
 * (`STRIPE_PRO_MONTHLY_PRICE_ID` / `STRIPE_PRO_ANNUAL_PRICE_ID`). Consumers must
 * NOT assume the suffix equals the mapping id for these rows.
 */

/**
 * Seeded multi-price product id (Stripe). Both price rows reference this product.
 */
export const STRIPE_MULTI_PRO_PRODUCT_ID = 'prod_stripe_multi_pro' as const

/**
 * Shared entitlement key for both price rows. The whole point of the multi-price
 * feature is that two prices can share one key while carrying DIFFERENT points
 * strategies — tests that assert grant amounts MUST pin the price, not the key.
 */
export const STRIPE_MULTI_PRO_SHARED_KEY = 'pro-plan' as const

/**
 * Monthly price id. billing_type=`recurring`, billing_period=`month`,
 * points_per_period=1000.
 */
export const STRIPE_PRO_MONTHLY_PRICE_ID = 'price_stripe_pro_monthly' as const

/**
 * Annual price id. billing_type=`recurring`, billing_period=`year`,
 * points_per_period=12000.
 *
 * The 12000-vs-1000 distinction under one shared key is the load-bearing
 * invariant for US-EM-008/009 grant assertions.
 */
export const STRIPE_PRO_ANNUAL_PRICE_ID = 'price_stripe_pro_annual' as const

/**
 * Seeded points strategy per price row. Exposed as a record so consumers can
 * assert "grant matches the selected price's strategy" without re-encoding the
 * numbers inline (which would silently pass if both rows ever drifted to the
 * same value).
 */
export const STRIPE_MULTI_PRO_POINTS_STRATEGY = {
  [STRIPE_PRO_MONTHLY_PRICE_ID]: 1000,
  [STRIPE_PRO_ANNUAL_PRICE_ID]: 12000,
} as const

/**
 * Realm the multi-price product is seeded into. Matches `POINTS_REALM_ID` in
 * `scripts/lib/demo_seed.py` (`realm-001`), the same realm as the existing
 * one-time mappings so the admin master-detail page + user purchase page render
 * them alongside the legacy catalog.
 */
export const MULTI_PRICE_REALM_ID = 'realm-001' as const

/**
 * Payment provider for the multi-price product. The feature is Stripe-first
 * (Creem is price-less and seeds a single NULL-price row).
 */
export const MULTI_PRICE_PAYMENT_PROVIDER = 'stripe' as const
