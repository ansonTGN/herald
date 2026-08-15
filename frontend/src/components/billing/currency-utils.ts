import type { PurchaseOptionView } from '@/lib/api-generated'
import { normalizeCurrencyCode } from '@/lib/currency-utils'

/**
 * Currency selection helpers for the purchase page and the currency preference
 * forms.
 *
 * Catalog rows synced from Stripe carry lowercase currency codes in
 * `provider_product_info` ("usd"), while user preferences and the realm
 * default currency are validated uppercase ISO codes ("USD"). Every grouping
 * key and preference match below is therefore case-insensitive and normalized
 * to uppercase.
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
 * Effective preferred currency: the user's override when set, otherwise the
 * realm default. `null` when neither is configured.
 */
export function resolveEffectivePreferredCurrency(
  userPreferred: string | null | undefined,
  realmDefault: string | null | undefined
): string | null {
  const user = normalizeCurrencyCode(userPreferred)
  if (user !== '') return user
  const realm = normalizeCurrencyCode(realmDefault)
  if (realm !== '') return realm
  return null
}

/**
 * Display-side highlight chain: user preferred currency, then the realm
 * default, then the first available currency. Returns `undefined` when the
 * entitlement has no priced currency at all (store-priced rows only).
 *
 * Programmatic default resolution deliberately does NOT use this chain — the
 * browser purchase flow always submits an explicit mapping id.
 */
export function resolveHighlightedCurrency(
  availableCurrencies: string[],
  userPreferred: string | null | undefined,
  realmDefault: string | null | undefined
): string | undefined {
  const normalized = availableCurrencies.map(normalizeCurrencyCode)
  const user = normalizeCurrencyCode(userPreferred)
  if (user !== '' && normalized.includes(user)) return user
  const realm = normalizeCurrencyCode(realmDefault)
  if (realm !== '' && normalized.includes(realm)) return realm
  return normalized[0]
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

/**
 * Whether the user's effective preferred currency is configured but absent
 * from this entitlement's available currencies (drives the "preferred currency
 * not available" hint — all currencies stay selectable).
 */
export function isPreferredCurrencyUnavailable(
  availableCurrencies: string[],
  userPreferred: string | null | undefined,
  realmDefault: string | null | undefined
): boolean {
  const effective = resolveEffectivePreferredCurrency(userPreferred, realmDefault)
  if (effective === null) return false
  return !availableCurrencies.map(normalizeCurrencyCode).includes(effective)
}
