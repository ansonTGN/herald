/**
 * Multi-Currency Live Product Helper
 *
 * Shared utility for demo tests that need a real Stripe product spanning
 * MULTIPLE CURRENCIES on one entitlement ("Herald Live Multi-Currency Pro"):
 *   - USD monthly recurring price (1000 = $10.00)
 *   - USD annual  recurring price (12000 = $120.00)
 *   - EUR monthly recurring price (1000 = €10.00)
 *   - CNY monthly recurring price (6800 = ¥68.00)
 *
 * The USD month + USD year pair reproduces the "same currency, multiple
 * billing periods" catalog shape (a currency narrows candidates but never
 * uniquely identifies a price row), while EUR/CNY provide the alternative
 * currency groups for switcher/highlight assertions.
 *
 * Mirrors `multi-price-live-product.ts` (create-or-reuse + reactivate
 * lifecycle); kept as a separate file so the single-currency USD-only helper
 * stays untouched. The product/price ids are REAL Stripe ids resolved at
 * runtime — never placeholder seed ids.
 */

export type PurchasePeriod = 'month' | 'year'

export const MULTI_CURRENCY_PRODUCT_NAME = 'Herald Live Multi-Currency Pro'

/** Stable key for each (currency, interval) price slot of the product. */
export type MultiCurrencyPriceSlot = 'usd-month' | 'usd-year' | 'eur-month' | 'cny-month'

interface PriceSpec {
  slot: MultiCurrencyPriceSlot
  currency: string
  interval: PurchasePeriod
  unitAmount: number
}

/**
 * Design-pinned price matrix (NOT seed ids). Currency codes are lowercase
 * because Stripe stores/returns lowercase currency codes on Price objects.
 */
export const MULTI_CURRENCY_PRICE_SPECS: readonly PriceSpec[] = [
  { slot: 'usd-month', currency: 'usd', interval: 'month', unitAmount: 1000 },
  { slot: 'usd-year', currency: 'usd', interval: 'year', unitAmount: 12000 },
  { slot: 'eur-month', currency: 'eur', interval: 'month', unitAmount: 1000 },
  { slot: 'cny-month', currency: 'cny', interval: 'month', unitAmount: 6800 },
]

export interface MultiCurrencyProduct {
  /** Real Stripe product id (prod_*). */
  productId: string
  /** Real Stripe price id (price_*) per (currency, interval) slot. */
  priceIds: Record<MultiCurrencyPriceSlot, string>
  /** Whether this run created the product (vs reused an existing one). */
  created: boolean
}

interface StripePrice {
  id: string
  type: string
  currency: string
  recurring?: { interval: string }
  active: boolean
}

async function stripeFormPost(
  secretKey: string,
  path: string,
  params: URLSearchParams,
): Promise<{ ok: boolean; status: number; text: () => Promise<string> }> {
  const resp = await fetch(`https://api.stripe.com/v1/${path}`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${secretKey}`,
      'Content-Type': 'application/x-www-form-urlencoded',
    },
    body: params.toString(),
  })
  return {
    ok: resp.ok,
    status: resp.status,
    text: () => resp.text(),
  }
}

async function listProducts(
  secretKey: string,
): Promise<Array<{ id: string; name: string; active: boolean }>> {
  const resp = await fetch('https://api.stripe.com/v1/products?limit=100', {
    headers: { Authorization: `Bearer ${secretKey}` },
  })
  if (!resp.ok) {
    throw new Error(`Stripe list products failed: ${resp.status} ${await resp.text()}`)
  }
  const body = (await resp.json()) as { data: Array<{ id: string; name: string; active: boolean }> }
  return body.data
}

async function listPrices(secretKey: string, productId: string): Promise<StripePrice[]> {
  const resp = await fetch(`https://api.stripe.com/v1/prices?product=${productId}&limit=100`, {
    headers: { Authorization: `Bearer ${secretKey}` },
  })
  if (!resp.ok) {
    throw new Error(`Stripe list prices failed: ${resp.status} ${await resp.text()}`)
  }
  const body = (await resp.json()) as { data: StripePrice[] }
  return body.data
}

/**
 * Verify (via Stripe API) whether the multi-currency product already exists in
 * the test account; if so, reuse its per-slot price ids (creating any missing
 * price). Otherwise create the product + all four prices.
 */
export async function ensureMultiCurrencyProduct(
  secretKey: string,
): Promise<MultiCurrencyProduct> {
  const existing = (await listProducts(secretKey)).find(
    (p) => p.name === MULTI_CURRENCY_PRODUCT_NAME,
  )

  let productId: string
  let created: boolean

  if (existing) {
    // Herald's provider sync only fetches `active=true` products (see
    // `multi-price-live-product.ts` for the same reactivate-on-reuse rule).
    if (!existing.active) {
      const reactivate = await stripeFormPost(
        secretKey,
        `products/${existing.id}`,
        new URLSearchParams({ active: 'true' }),
      )
      if (!reactivate.ok) {
        throw new Error(
          `Stripe reactivate product failed: ${reactivate.status} ${await reactivate.text()}`,
        )
      }
    }
    productId = existing.id
    created = false
  } else {
    const create = await stripeFormPost(
      secretKey,
      'products',
      new URLSearchParams({ name: MULTI_CURRENCY_PRODUCT_NAME }),
    )
    if (!create.ok) {
      throw new Error(`Stripe create product failed: ${create.status} ${await create.text()}`)
    }
    productId = (JSON.parse(await create.text()) as { id: string }).id
    created = true
  }

  // Match the backend sync contract: Herald only syncs `active=true` Stripe
  // prices, so slot lookups must pick active prices or create missing ones.
  const prices = await listPrices(secretKey, productId)
  const priceIds = {} as Record<MultiCurrencyPriceSlot, string>
  for (const spec of MULTI_CURRENCY_PRICE_SPECS) {
    const match = prices.find(
      (pr) =>
        pr.type === 'recurring' &&
        pr.recurring?.interval === spec.interval &&
        pr.currency === spec.currency &&
        pr.active === true,
    )
    if (match) {
      priceIds[spec.slot] = match.id
      continue
    }
    const priceResp = await stripeFormPost(
      secretKey,
      'prices',
      new URLSearchParams({
        product: productId,
        currency: spec.currency,
        unit_amount: String(spec.unitAmount),
        'recurring[interval]': spec.interval,
      }),
    )
    if (!priceResp.ok) {
      throw new Error(
        `Stripe create ${spec.currency}/${spec.interval} price failed: ` +
          `${priceResp.status} ${await priceResp.text()}`,
      )
    }
    priceIds[spec.slot] = (JSON.parse(await priceResp.text()) as { id: string }).id
  }

  return { productId, priceIds, created }
}
