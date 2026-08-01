/**
 * Mapping Catalog Resolution Helpers
 *
 * Shared runtime resolution for tests that previously depended on placeholder
 * seed ids (now removed from `demo_seed.py`). These helpers trigger a provider
 * sync, then locate mapping rows by real `externalProductId`/`externalPriceId`.
 *
 * Pattern (mirrors the live tests in `demo/e2e/live/`):
 *   1. seed provider credentials via `realm-seed.ts`
 *   2. POST /entitlement-mappings/sync to pull the real catalog
 *   3. GET /entitlement-mappings and match by real id
 */

import type { APIRequestContext } from '@playwright/test'
import { ensureMultiPriceProduct, type MultiPriceProduct } from './multi-price-live-product'

interface MappingListItem {
  id: string
  externalProductId: string
  externalPriceId: string | null
  entitlementKey: string
  enabled: boolean
}

interface PageLike {
  request: {
    post: (url: string, opts?: { data?: unknown }) => Promise<{ ok: () => boolean; text: () => Promise<string> }>
    get: (url: string) => Promise<{ ok: () => boolean; json: () => Promise<unknown> }>
  }
}

/**
 * Trigger a provider sync for the given realm/provider, then return the
 * (now real) mapping list. Throws loudly if sync fails.
 */
export async function syncAndListMappings(
  page: PageLike,
  baseUrl: string,
  realmId: string,
  provider: string,
): Promise<MappingListItem[]> {
  const syncResp = await page.request.post(
    `${baseUrl}/api/bill/${realmId}/entitlement-mappings/sync`,
    { data: { paymentProvider: provider } },
  )
  if (!syncResp.ok()) {
    throw new Error(`provider sync failed: ${await syncResp.text().catch(() => '')}`)
  }

  const mappingsResp = await page.request.get(
    `${baseUrl}/api/bill/${realmId}/entitlement-mappings?paymentProvider=${provider}`,
  )
  if (!mappingsResp.ok()) {
    throw new Error('listing entitlement-mappings failed')
  }
  const body = (await mappingsResp.json()) as { items?: MappingListItem[] } | MappingListItem[]
  return Array.isArray(body) ? body : (body.items ?? [])
}

/**
 * Find a mapping row by real externalProductId. Returns the first match or null.
 */
export function findMappingByProduct(
  items: MappingListItem[],
  externalProductId: string,
): MappingListItem | null {
  return items.find((m) => m.externalProductId === externalProductId) ?? null
}

/**
 * Find a mapping row by real externalPriceId. Returns the first match or null.
 */
export function findMappingByPrice(
  items: MappingListItem[],
  externalPriceId: string,
): MappingListItem | null {
  return items.find((m) => m.externalPriceId === externalPriceId) ?? null
}

/**
 * Resolved real multi-price catalog for a realm. Returned by
 * {@link ensureMultiPriceCatalog} for tests that previously used the
 * placeholder `prod_stripe_multi_pro` id.
 */
export interface MultiPriceCatalog {
  /** Real Stripe product (prod_*) and price (price_*) ids. */
  product: MultiPriceProduct
  /** Herald-side mapping list after sync (one row per price). */
  mappings: MappingListItem[]
  /** Monthly mapping row id (Herald id). */
  monthlyMappingId: string
  /** Annual mapping row id (Herald id). */
  annualMappingId: string
}

/**
 * High-level setup for tests needing a real multi-price catalog in a realm.
 *
 * 1. ensures (creates or reuses) the real Stripe multi-price product,
 * 2. seeds Stripe credentials into the realm,
 * 3. triggers a provider sync,
 * 4. resolves the two price-level mapping rows by real price id.
 *
 * Caller must pass an API request context authenticated as the realm admin.
 */
export async function ensureMultiPriceCatalog(
  request: APIRequestContext,
  opts: {
    baseUrl: string
    realmId: string
    stripeSecretKey: string
    stripePublishableKey: string
    stripeWebhookSecret: string
  },
): Promise<MultiPriceCatalog> {
  const { baseUrl, realmId, stripeSecretKey, stripePublishableKey, stripeWebhookSecret } = opts

  // 1. Ensure the real Stripe multi-price product exists.
  const product = await ensureMultiPriceProduct(stripeSecretKey)

  // 2. Seed Stripe credentials for the realm.
  const { seedStripeConfig } = await import('../secrets/realm-seed')
  await seedStripeConfig(request, realmId, {
    publishableKey: stripePublishableKey,
    secretKey: stripeSecretKey,
    webhookSecret: stripeWebhookSecret,
  })

  // 3. Sync the catalog into Herald.
  const syncResp = await request.post(`${baseUrl}/api/bill/${realmId}/entitlement-mappings/sync`, {
    data: { paymentProvider: 'stripe' },
  })
  if (!syncResp.ok()) {
    throw new Error(`provider sync failed: ${await syncResp.text().catch(() => '')}`)
  }

  // 4. Resolve the two price-level mapping rows.
  const listResp = await request.get(
    `${baseUrl}/api/bill/${realmId}/entitlement-mappings?paymentProvider=stripe`,
  )
  if (!listResp.ok()) {
    throw new Error('listing entitlement-mappings failed')
  }
  const body = (await listResp.json()) as { items?: MappingListItem[] } | MappingListItem[]
  const items: MappingListItem[] = Array.isArray(body) ? body : (body.items ?? [])

  const monthly = items.find((m) => m.externalPriceId === product.monthlyPriceId)
  const annual = items.find((m) => m.externalPriceId === product.annualPriceId)
  if (!monthly) {
    throw new Error(
      `synced monthly price-level mapping not found (price=${product.monthlyPriceId})`,
    )
  }
  if (!annual) {
    throw new Error(
      `synced annual price-level mapping not found (price=${product.annualPriceId})`,
    )
  }

  return {
    product,
    mappings: items,
    monthlyMappingId: monthly.id,
    annualMappingId: annual.id,
  }
}

export type { MappingListItem, PageLike }
