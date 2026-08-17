import type { PurchaseOptionView } from '@/lib/api-generated'
import { normalizeCurrencyCode } from '@/lib/currency-utils'

/**
 * Currency selection helpers for the purchase page.
 *
 * Catalog rows synced from Stripe carry lowercase currency codes in
 * `provider_product_info` ("usd"). Every grouping key below is
 * case-insensitive and normalized to uppercase.
 *
 * The code validation/normalization primitives live in `@/lib/currency-utils`.
 */

/** A single currency slice of one entitlement's price rows. */
export interface CurrencyOptionGroup {
  /** Uppercase currency code; `''` groups rows that carry no currency. */
  currency: string
  options: PurchaseOptionView[]
}

/** One entitlement's rows plus its currency slices. */
export interface EntitlementOptionGroup {
  entitlementKey: string
  displayName: string
  options: PurchaseOptionView[]
  currencyGroups: CurrencyOptionGroup[]
  /** Uppercase currency keys that actually carry rows, in first-seen order. */
  currencies: string[]
}

/**
 * Group an entitlement's price rows by currency. Rows are bucketed under the
 * uppercase-normalized code (so "usd" and "USD" land in the same group) and
 * groups keep first-seen order. Rows without a currency form the `''` group.
 */
export function groupByCurrency(options: PurchaseOptionView[]): CurrencyOptionGroup[] {
  const groups = new Map<string, PurchaseOptionView[]>()
  for (const option of options) {
    const key = normalizeCurrencyCode(option.currency)
    const bucket = groups.get(key)
    if (bucket) {
      bucket.push(option)
    } else {
      groups.set(key, [option])
    }
  }
  return [...groups.entries()].map(([currency, groupOptions]) => ({
    currency,
    options: groupOptions,
  }))
}

/**
 * Split the purchase page's flat price-row list into per-entitlement groups.
 * The display name is the first non-empty `displayName` in the group (rows of
 * one entitlement share the synced product name).
 */
export function groupByEntitlement(options: PurchaseOptionView[]): EntitlementOptionGroup[] {
  const byKey = new Map<string, PurchaseOptionView[]>()
  for (const option of options) {
    const bucket = byKey.get(option.entitlementKey)
    if (bucket) {
      bucket.push(option)
    } else {
      byKey.set(option.entitlementKey, [option])
    }
  }
  return [...byKey.entries()].map(([entitlementKey, groupOptions]) => {
    const currencyGroups = groupByCurrency(groupOptions)
    return {
      entitlementKey,
      displayName: groupOptions.find((o) => o.displayName)?.displayName ?? entitlementKey,
      options: groupOptions,
      currencyGroups,
      currencies: currencyGroups.map((g) => g.currency).filter((c) => c !== ''),
    }
  })
}

/**
 * Whether the currency switcher applies to an entitlement: only when every row
 * is Stripe-priced (the sole provider whose prices Herald resolves) AND the
 * rows span at least two currencies. Creem / Apple / Google / WeChat rows are
 * store-priced and degrade to a flat single-price list; a single-currency
 * Stripe product has nothing to switch between.
 */
export function isCurrencySwitchable(group: EntitlementOptionGroup): boolean {
  const allStripe = group.options.every((o) => o.paymentProvider === 'stripe')
  return allStripe && group.currencies.length >= 2
}
