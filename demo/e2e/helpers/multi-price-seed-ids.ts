/**
 * Multi-Price Demo Ids (support-multiple-price)
 *
 * ⚠️ The placeholder seed product/price ids (`prod_stripe_multi_pro`,
 * `price_stripe_pro_monthly`, `price_stripe_pro_annual`) were REMOVED from
 * `scripts/lib/demo_seed.py`. Entitlement mappings are no longer seeded as
 * placeholders — the catalog is pulled via provider sync from real Stripe.
 *
 * What remains here are the realm/provider constants that do NOT depend on
 * the placeholder seed. Real Stripe product/price ids are resolved at runtime:
 *   - LIVE tests use `ensureMultiPriceProduct()` (see
 *     `demo/e2e/helpers/multi-price-live-product.ts`) to create/reuse a real
 *     multi-price product and resolve its real price ids.
 *
 * Consumers that previously imported the placeholder id constants must switch
 * to runtime resolution against the synced catalog or the live helper.
 */

/**
 * Realm the multi-price product is consumed in. Matches `POINTS_REALM_ID` in
 * `scripts/lib/demo_seed.py` (`realm-001`).
 */
export const MULTI_PRICE_REALM_ID = 'realm-001' as const

/**
 * Payment provider for the multi-price product. The feature is Stripe-first
 * (Creem is price-less and seeds a single NULL-price row).
 */
export const MULTI_PRICE_PAYMENT_PROVIDER = 'stripe' as const
