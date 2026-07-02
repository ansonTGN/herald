/**
 * Multi-Price Live Product Helper
 *
 * Shared utility for LIVE demo tests that need a real Stripe multi-price
 * product ("Herald Live Multi-Price Pro") with two recurring prices sharing
 * the `pro-plan` entitlement key:
 *   - monthly recurring price (1000 = $10.00, 1000 points strategy)
 *   - annual  recurring price (12000 = $120.00, 12000 points strategy)
 *
 * Extracted from `us-em-009-multiple-price-checkout-live.e2e.ts` so migrated
 * non-live→live tests (multiple-price-entitlement-mapping, points-quota-*) can
 * reuse the same create-or-reuse + archive lifecycle.
 *
 * The product/price ids are REAL Stripe ids resolved at runtime — never the
 * placeholder seed ids that were removed from `demo_seed.py`.
 */

export type PurchasePeriod = 'month' | 'year'

export const MULTI_PRICE_PRODUCT_NAME = 'Herald Live Multi-Price Pro'
export const MULTI_PRICE_SHARED_KEY = 'pro-plan'
export const MONTHLY_POINTS_STRATEGY = 1000
export const ANNUAL_POINTS_STRATEGY = 12000

export interface MultiPriceProduct {
  /** Real Stripe product id (prod_*). */
  productId: string
  /** Real Stripe monthly recurring price id (price_*). */
  monthlyPriceId: string
  /** Real Stripe annual recurring price id (price_*). */
  annualPriceId: string
  /** Whether this run created the product (vs reused an existing one). */
  created: boolean
}

/**
 * Verify (via Stripe API) whether a multi-price "pro-plan" product already
 * exists in the test account; if so, reuse its monthly/annual recurring price
 * ids. Otherwise create the product + two recurring prices.
 */
export async function ensureMultiPriceProduct(
  secretKey: string,
): Promise<MultiPriceProduct> {
  // 1. Look for an existing product of that name.
  const listResp = await fetch('https://api.stripe.com/v1/products?limit=100', {
    headers: { Authorization: `Bearer ${secretKey}` },
  })
  if (!listResp.ok) {
    throw new Error(`Stripe list products failed: ${listResp.status} ${await listResp.text()}`)
  }
  const listBody = (await listResp.json()) as {
    data: Array<{ id: string; name: string; active: boolean }>
  }
  const existing = listBody.data.find((p) => p.name === MULTI_PRICE_PRODUCT_NAME)

  if (existing) {
    // The sibling live test `us-em-009-multiple-price-checkout-live` archives
    // this product during its own cleanup, and Herald's provider sync only
    // fetches `active=true` products (see backend/infra/src/billing/
    // provider_product_api.rs fetch_stripe_products). Re-activate the product
    // if a prior run archived it, so reuse stays consistent with the sync.
    if (!existing.active) {
      const reactivateResp = await fetch(
        `https://api.stripe.com/v1/products/${existing.id}`,
        {
          method: 'POST',
          headers: {
            Authorization: `Bearer ${secretKey}`,
            'Content-Type': 'application/x-www-form-urlencoded',
          },
          body: new URLSearchParams({ active: 'true' }).toString(),
        },
      )
      if (!reactivateResp.ok) {
        throw new Error(
          `Stripe reactivate product failed: ${reactivateResp.status} ${await reactivateResp.text()}`,
        )
      }
    }

    const pricesResp = await fetch(
      `https://api.stripe.com/v1/prices?product=${existing.id}&limit=100`,
      { headers: { Authorization: `Bearer ${secretKey}` } },
    )
    if (!pricesResp.ok) {
      throw new Error(`Stripe list prices failed: ${pricesResp.status} ${await pricesResp.text()}`)
    }
    const pricesBody = (await pricesResp.json()) as {
      data: Array<{
        id: string
        type: string
        recurring?: { interval: string }
        unit_amount: number
        active: boolean
      }>
    }
    // Match the backend sync contract (Herald only syncs `active=true`
    // Stripe prices — see backend/infra/src/billing/provider_product_api.rs
    // fetch_stripe_products). Picking an inactive price here would let the
    // helper hand the sync an id it never upserts, so the subsequent mapping
    // lookup fails.
    const monthly = pricesBody.data.find(
      (pr) =>
        pr.type === 'recurring' &&
        pr.recurring?.interval === 'month' &&
        pr.active === true,
    )
    const annual = pricesBody.data.find(
      (pr) =>
        pr.type === 'recurring' &&
        pr.recurring?.interval === 'year' &&
        pr.active === true,
    )
    if (monthly && annual) {
      return {
        productId: existing.id,
        monthlyPriceId: monthly.id,
        annualPriceId: annual.id,
        created: false,
      }
    }
    // Product exists but missing one price — create the missing one(s) on it.
    const monthlyFinal =
      monthly?.id ??
      (await createRecurringPrice(secretKey, existing.id, 'month', MONTHLY_POINTS_STRATEGY)).id
    const annualFinal =
      annual?.id ??
      (await createRecurringPrice(secretKey, existing.id, 'year', ANNUAL_POINTS_STRATEGY)).id
    return {
      productId: existing.id,
      monthlyPriceId: monthlyFinal,
      annualPriceId: annualFinal,
      created: false,
    }
  }

  // 2. Create the product.
  const productResp = await fetch('https://api.stripe.com/v1/products', {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${secretKey}`,
      'Content-Type': 'application/x-www-form-urlencoded',
    },
    body: new URLSearchParams({ name: MULTI_PRICE_PRODUCT_NAME }).toString(),
  })
  if (!productResp.ok) {
    throw new Error(`Stripe create product failed: ${productResp.status} ${await productResp.text()}`)
  }
  const product = (await productResp.json()) as { id: string }
  const monthlyCreated = await createRecurringPrice(
    secretKey,
    product.id,
    'month',
    MONTHLY_POINTS_STRATEGY,
  )
  const annualCreated = await createRecurringPrice(
    secretKey,
    product.id,
    'year',
    ANNUAL_POINTS_STRATEGY,
  )
  return {
    productId: product.id,
    monthlyPriceId: monthlyCreated.id,
    annualPriceId: annualCreated.id,
    created: true,
  }
}

export async function createRecurringPrice(
  secretKey: string,
  productId: string,
  interval: PurchasePeriod,
  unitAmount: number,
): Promise<{ id: string }> {
  const params = new URLSearchParams({
    product: productId,
    currency: 'usd',
    unit_amount: String(unitAmount),
    'recurring[interval]': interval,
  })
  const resp = await fetch('https://api.stripe.com/v1/prices', {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${secretKey}`,
      'Content-Type': 'application/x-www-form-urlencoded',
    },
    body: params.toString(),
  })
  if (!resp.ok) {
    throw new Error(
      `Stripe create ${interval} price failed: ${resp.status} ${await resp.text()}`,
    )
  }
  return (await resp.json()) as { id: string }
}

/**
 * Archive the product (and thereby deactivate its prices) for cleanup. Stripe
 * does not delete products/prices; archiving is the supported cleanup.
 */
export async function archiveProduct(secretKey: string, productId: string): Promise<void> {
  const resp = await fetch(`https://api.stripe.com/v1/products/${productId}`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${secretKey}`,
      'Content-Type': 'application/x-www-form-urlencoded',
    },
    body: new URLSearchParams({ active: 'false' }).toString(),
  })
  if (!resp.ok) {
    console.error(`[cleanup] Stripe archive product ${productId} failed: ${resp.status}`)
  }
}
