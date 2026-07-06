/**
 * Factory fixtures for the price-granularity billing endpoints
 * (entitlement-mappings list / batch / sync / purchase-options / checkout).
 *
 * Field names are copied verbatim from the generated client
 * `frontend/src/lib/api-generated/types.gen.ts` — that file is AUTHORITATIVE.
 * Do not invent fields: there is no `unresolved` flag on
 * `EntitlementMappingResponse`; the webhook-unresolved state is DERIVED
 * client-side (see `unresolvedMappingList`).
 *
 * These are factory FUNCTIONS (per the testing guide), not bare constants,
 * so callers can override individual fields without mutating shared state.
 */

import type {
  BatchUpdateEntitlementMappingsResponse,
  EntitlementMappingResponse,
  MappingActiveSubscriptionLockErrorBody,
  PurchaseOptionView,
  SyncProviderResponse,
} from '@/lib/api-generated'
import type { ErrorResponse } from '@/lib/api-generated'

// ---------------------------------------------------------------------------
// EntitlementMappingResponse
// ---------------------------------------------------------------------------

/**
 * Build a single `EntitlementMappingResponse` with sensible defaults.
 *
 * Defaults describe an enabled recurring monthly Stripe price bound to the
 * `pro-plan` entitlement. Pass `overrides` to mutate any field; this is the
 * shape every other list factory composes from.
 */
export function makeMapping(
  overrides?: Partial<EntitlementMappingResponse>
): EntitlementMappingResponse {
  return {
    id: 'map_pro_monthly',
    entitlementKey: 'pro-plan',
    externalProductId: 'prod_pro',
    externalPriceId: 'price_pro_monthly',
    paymentProvider: 'stripe',
    billingType: 'recurring',
    billingPeriod: 'month',
    enabled: true,
    bucketId: 'bucket-default',
    grantOnSubscribe: true,
    pointsPerPeriod: null,
    validityDays: null,
    providerProductInfo: null,
    syncedAt: '2026-01-01T00:00:00Z',
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    ...overrides,
  }
}

/**
 * One multi-price product (`prod_pro`, 2 prices sharing `entitlementKey:
 * 'pro-plan'`: monthly + annual) plus one single-price Creem product
 * (`prod_starter`) whose provider exposes no price id — `externalPriceId`
 * is `null` per design A2 (do NOT synthesize a placeholder price id).
 */
export function multiPriceMappingList(): EntitlementMappingResponse[] {
  return [
    makeMapping({
      id: 'map_pro_monthly',
      entitlementKey: 'pro-plan',
      externalProductId: 'prod_pro',
      externalPriceId: 'price_pro_monthly',
      paymentProvider: 'stripe',
      billingType: 'recurring',
      billingPeriod: 'month',
      enabled: true,
    }),
    makeMapping({
      id: 'map_pro_annual',
      entitlementKey: 'pro-plan',
      externalProductId: 'prod_pro',
      externalPriceId: 'price_pro_annual',
      paymentProvider: 'stripe',
      billingType: 'recurring',
      billingPeriod: 'year',
      enabled: true,
    }),
    makeMapping({
      id: 'map_starter',
      entitlementKey: 'starter',
      externalProductId: 'prod_starter',
      // Creem price-less product: no Stripe price id.
      externalPriceId: null,
      paymentProvider: 'creem',
      billingType: 'one_time',
      billingPeriod: null,
      enabled: true,
    }),
  ]
}

/**
 * A multi-price product where the monthly price carries pre-existing quota
 * windows and the annual price has none (`null`). Used by the quota-editor
 * integration tests: the editor must seed the monthly row's windows and leave
 * the annual row empty.
 *
 * `quotaWindows` on the response is `EntitlementQuotaWindowDto[]` (carries a
 * display `key`); the editor only consumes `windowSeconds`/`limit` from it.
 */
export function multiPriceWithQuotaWindowsList(): EntitlementMappingResponse[] {
  return [
    makeMapping({
      id: 'map_pro_monthly',
      entitlementKey: 'pro-plan',
      externalProductId: 'prod_pro',
      externalPriceId: 'price_pro_monthly',
      paymentProvider: 'stripe',
      billingType: 'recurring',
      billingPeriod: 'month',
      enabled: true,
      quotaWindows: [
        { key: '1h', windowSeconds: 3600, limit: 100 },
        { key: '1d', windowSeconds: 86_400, limit: 1000 },
      ],
    }),
    makeMapping({
      id: 'map_pro_annual',
      entitlementKey: 'pro-plan',
      externalProductId: 'prod_pro',
      externalPriceId: 'price_pro_annual',
      paymentProvider: 'stripe',
      billingType: 'recurring',
      billingPeriod: 'year',
      enabled: true,
      quotaWindows: null,
    }),
  ]
}

/**
 * At least one row matches the DERIVED webhook-unresolved rule
 * `externalProductId` set AND `enabled === true` AND
 * (`!billingType` OR `pointsPerPeriod == null`).
 *
 * - `map_unresolved_no_billing_type`: enabled, has product, no billingType.
 * - `map_unresolved_no_points`: enabled, has product, billingType set but
 *   `pointsPerPeriod == null` (credit-strategy row not yet configured).
 *
 * A fully-resolved row is included as a negative control.
 */
export function unresolvedMappingList(): EntitlementMappingResponse[] {
  return [
    makeMapping({
      id: 'map_unresolved_no_billing_type',
      entitlementKey: 'unresolved-a',
      externalProductId: 'prod_unresolved',
      externalPriceId: 'price_unresolved_a',
      paymentProvider: 'stripe',
      billingType: null,
      billingPeriod: null,
      enabled: true,
      pointsPerPeriod: null,
    }),
    makeMapping({
      id: 'map_unresolved_no_points',
      entitlementKey: 'unresolved-b',
      externalProductId: 'prod_unresolved',
      externalPriceId: 'price_unresolved_b',
      paymentProvider: 'stripe',
      billingType: 'recurring',
      billingPeriod: 'month',
      enabled: true,
      pointsPerPeriod: null,
    }),
    // Negative control: fully resolved — must NOT be flagged unresolved.
    makeMapping({
      id: 'map_resolved',
      entitlementKey: 'resolved',
      externalProductId: 'prod_resolved',
      externalPriceId: 'price_resolved',
      paymentProvider: 'stripe',
      billingType: 'recurring',
      billingPeriod: 'month',
      enabled: true,
      pointsPerPeriod: 1000,
    }),
  ]
}

// ---------------------------------------------------------------------------
// Batch / sync response bodies
// ---------------------------------------------------------------------------

/**
 * Build a `BatchUpdateEntitlementMappingsResponse` echoing the product's full
 * latest price set and a `saved` count. Defaults to the `prod_pro` multi-price
 * set so batch-save happy-path tests get a realistic post-save snapshot.
 */
export function batchUpdateOkBody(
  overrides?: Partial<BatchUpdateEntitlementMappingsResponse>
): BatchUpdateEntitlementMappingsResponse {
  const defaults: BatchUpdateEntitlementMappingsResponse = {
    prices: multiPriceMappingList().filter((m) => m.externalProductId === 'prod_pro'),
    saved: 2,
  }
  return { ...defaults, ...overrides }
}

/**
 * 409 body for a batch save blocked by the active-subscription lock
 * The whole batch transaction is rolled back.
 */
export function protectedPrice409Body(
  activeSubscriptions: number
): MappingActiveSubscriptionLockErrorBody {
  return {
    code: 'mapping_in_use',
    activeSubscriptions,
  }
}

/**
 * 400 body for a batch save rejected by validation. Two canonical cases
 * entitlement-key regex violation and cross-product
 * shared-key rename. The backend ships these as the generic `ErrorResponse`
 * shape (`code: number, message: string`), NOT as the 409 lock body.
 */
export function batch400Body(
  message: string = 'Entitlement key does not match ^[a-z0-9-]{1,64}$'
): ErrorResponse {
  return {
    code: 400,
    message,
  }
}

/**
 * Provider sync result. `syncStatus` defaults to `'ok'`; pass `partialErrors`
 * / `error` for partial-failure scenarios.
 */
export function syncResult(overrides?: Partial<SyncProviderResponse>): SyncProviderResponse {
  return {
    productsSynced: 3,
    pricesSynced: 7,
    syncStatus: 'ok',
    error: null,
    partialErrors: [],
    ...overrides,
  }
}

// ---------------------------------------------------------------------------
// PurchaseOptionView / checkout
// ---------------------------------------------------------------------------

/**
 * Multi-price purchase options covering every card variant the purchase page
 * renders: monthly recurring, annual recurring, one_time,
 * and one DISABLED option (so the disabled-card reason path is exercised).
 */
export function purchaseOptionsList(): PurchaseOptionView[] {
  return [
    {
      mappingId: 'map_pro_monthly',
      entitlementKey: 'pro-plan',
      externalProductId: 'prod_pro',
      externalPriceId: 'price_pro_monthly',
      paymentProvider: 'stripe',
      billingType: 'recurring',
      billingPeriod: 'month',
      amount: 999,
      currency: 'usd',
      displayName: 'Pro Monthly',
      enabled: true,
      pointsPerPeriod: null,
    },
    {
      mappingId: 'map_pro_annual',
      entitlementKey: 'pro-plan',
      externalProductId: 'prod_pro',
      externalPriceId: 'price_pro_annual',
      paymentProvider: 'stripe',
      billingType: 'recurring',
      billingPeriod: 'year',
      amount: 9999,
      currency: 'usd',
      displayName: 'Pro Annual',
      enabled: true,
      pointsPerPeriod: null,
    },
    {
      mappingId: 'map_starter',
      entitlementKey: 'starter',
      externalProductId: 'prod_starter',
      // Creem price-less product: no Stripe price id (design A2).
      externalPriceId: null,
      paymentProvider: 'creem',
      billingType: 'one_time',
      billingPeriod: null,
      amount: 499,
      currency: 'usd',
      displayName: 'Starter One-Time',
      enabled: true,
      pointsPerPeriod: null,
    },
    {
      mappingId: 'map_disabled',
      entitlementKey: 'legacy',
      externalProductId: 'prod_legacy',
      externalPriceId: 'price_legacy',
      paymentProvider: 'stripe',
      billingType: 'recurring',
      billingPeriod: 'month',
      amount: 0,
      currency: 'usd',
      displayName: 'Legacy (disabled)',
      enabled: false,
      pointsPerPeriod: null,
    },
  ]
}
