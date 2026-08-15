/**
 * Shared `PurchaseOptionView` factory for the purchase-page / currency-grouping
 * Vitest suites. Centralizing the baseline row (Stripe USD monthly, one point
 * rule-free) lets a test swap a single field while the rest stays stable, and
 * keeps new `PurchaseOptionView` fields from having to be re-defaulted in every
 * test file's local copy.
 */

import type { PurchaseOptionView } from '@/lib/api-generated'

export function makePurchaseOption(overrides: Partial<PurchaseOptionView>): PurchaseOptionView {
  return {
    mappingId: overrides.mappingId ?? 'map-1',
    externalProductId: 'prod_1',
    externalPriceId: overrides.externalPriceId ?? 'price_1',
    paymentProvider: overrides.paymentProvider ?? 'stripe',
    entitlementKey: overrides.entitlementKey ?? 'pro-plan',
    billingType: overrides.billingType ?? 'recurring',
    billingPeriod: overrides.billingPeriod ?? 'month',
    displayName: overrides.displayName ?? 'Pro',
    amount: overrides.amount ?? 1000,
    currency: overrides.currency ?? 'usd',
    pointRules: overrides.pointRules ?? [],
    enabled: overrides.enabled ?? true,
    ...overrides,
  } as PurchaseOptionView
}
