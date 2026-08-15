import { describe, it, expect } from 'vitest'
import { isValidCurrencyCode, normalizeCurrencyCode } from '@/lib/currency-utils'
import {
  groupByCurrency,
  groupByEntitlement,
  resolveEffectivePreferredCurrency,
  resolveHighlightedCurrency,
  isCurrencySwitchable,
  isPreferredCurrencyUnavailable,
} from '../currency-utils'
import { makePurchaseOption as makeOption } from '@/test/fixtures/purchase-option'

// Contracts for the currency-selection helpers. The load-bearing rules:
//   - Stripe-synced catalog rows carry lowercase codes ("usd") while
//     preferences are uppercase ISO ("USD") — grouping and matching must be
//     case-insensitive and normalized to uppercase.
//   - The display-side highlight chain is user preferred → realm default →
//     first available; it must never filter what the user can buy.
//   - Only all-Stripe multi-currency entitlements get a currency switcher;
//     store-priced providers and single-currency products degrade.

describe('isValidCurrencyCode', () => {
  it.each(['USD', 'EUR', 'CNY'])('accepts a real uppercase ISO code (%s)', (code) => {
    expect(isValidCurrencyCode(code)).toBe(true)
  })

  it.each([
    'usd', // lowercase must be rejected: ISO codes are uppercase-only
    'US',
    'USDD',
    '美元',
    '',
    ' US',
  ])('rejects a malformed code (%s)', (code) => {
    expect(isValidCurrencyCode(code)).toBe(false)
  })

  it.each(['XXX', 'XTS'])('rejects the reserved code %s', (code) => {
    expect(isValidCurrencyCode(code)).toBe(false)
  })
})

describe('normalizeCurrencyCode', () => {
  it('uppercases and trims a catalog code', () => {
    expect(normalizeCurrencyCode(' usd ')).toBe('USD')
  })

  it('returns an empty key for null/undefined', () => {
    expect(normalizeCurrencyCode(null)).toBe('')
    expect(normalizeCurrencyCode(undefined)).toBe('')
  })
})

describe('groupByCurrency', () => {
  it('merges lowercase and uppercase spellings of one currency into a single group', () => {
    const groups = groupByCurrency([
      makeOption({ mappingId: 'm-1', currency: 'usd', billingPeriod: 'month' }),
      makeOption({ mappingId: 'm-2', currency: 'USD', billingPeriod: 'year' }),
    ])

    // One group with both rows: a "usd" + "USD" split would render two
    // switcher segments for the same currency and break preference matching.
    expect(groups).toHaveLength(1)
    expect(groups[0].currency).toBe('USD')
    expect(groups[0].options.map((o) => o.mappingId)).toEqual(['m-1', 'm-2'])
  })

  it('keeps first-seen group order and buckets currency-less rows under ""', () => {
    const groups = groupByCurrency([
      makeOption({ mappingId: 'm-eur', currency: 'eur' }),
      makeOption({ mappingId: 'm-none', currency: null }),
      makeOption({ mappingId: 'm-usd', currency: 'usd' }),
    ])

    expect(groups.map((g) => g.currency)).toEqual(['EUR', '', 'USD'])
  })
})

describe('groupByEntitlement', () => {
  it('splits rows by entitlement key and falls back to the key as display name', () => {
    const groups = groupByEntitlement([
      makeOption({ entitlementKey: 'pro', displayName: 'Pro', mappingId: 'm-1' }),
      makeOption({ entitlementKey: 'pro', displayName: 'Pro', mappingId: 'm-2' }),
      makeOption({ entitlementKey: 'starter', displayName: null, mappingId: 'm-3' }),
    ])

    expect(groups.map((g) => g.entitlementKey)).toEqual(['pro', 'starter'])
    expect(groups[0].options).toHaveLength(2)
    expect(groups[1].displayName).toBe('starter')
  })

  it('exposes the priced currencies (excluding the currency-less bucket)', () => {
    const groups = groupByEntitlement([
      makeOption({ entitlementKey: 'pro', currency: 'usd' }),
      makeOption({ entitlementKey: 'pro', currency: 'eur' }),
      makeOption({ entitlementKey: 'pro', currency: null, paymentProvider: 'creem' }),
    ])

    expect(groups[0].currencies).toEqual(['USD', 'EUR'])
  })
})

describe('resolveEffectivePreferredCurrency', () => {
  it('prefers the user override over the realm default', () => {
    expect(resolveEffectivePreferredCurrency('CNY', 'USD')).toBe('CNY')
  })

  it('falls back to the realm default when the override is unset', () => {
    expect(resolveEffectivePreferredCurrency(null, 'usd')).toBe('USD')
  })

  it('returns null when neither is configured', () => {
    expect(resolveEffectivePreferredCurrency(null, null)).toBeNull()
  })
})

describe('resolveHighlightedCurrency', () => {
  const available = ['USD', 'EUR']

  it('highlights the user preferred currency when available (case-insensitive)', () => {
    expect(resolveHighlightedCurrency(available, 'eur', 'USD')).toBe('EUR')
  })

  it('falls back to the realm default when the preference is unavailable', () => {
    expect(resolveHighlightedCurrency(available, 'CNY', 'usd')).toBe('USD')
  })

  it('falls back to the first available currency when neither preference matches', () => {
    expect(resolveHighlightedCurrency(available, 'CNY', 'JPY')).toBe('USD')
  })

  it('returns undefined when no priced currency exists', () => {
    expect(resolveHighlightedCurrency([], 'CNY', 'USD')).toBeUndefined()
  })
})

describe('isCurrencySwitchable', () => {
  it('is true for an all-Stripe entitlement spanning multiple currencies', () => {
    const [group] = groupByEntitlement([
      makeOption({ entitlementKey: 'pro', currency: 'usd' }),
      makeOption({ entitlementKey: 'pro', currency: 'eur' }),
    ])
    expect(isCurrencySwitchable(group)).toBe(true)
  })

  it('is false for a single-currency Stripe entitlement (nothing to switch)', () => {
    const [group] = groupByEntitlement([
      makeOption({ entitlementKey: 'pro', currency: 'usd', billingPeriod: 'month' }),
      makeOption({ entitlementKey: 'pro', currency: 'usd', billingPeriod: 'year' }),
    ])
    expect(isCurrencySwitchable(group)).toBe(false)
  })

  it('is false for a store-priced (Creem/IAP/WeChat) entitlement', () => {
    const [group] = groupByEntitlement([
      makeOption({ entitlementKey: 'pack', paymentProvider: 'creem', currency: 'usd' }),
      makeOption({ entitlementKey: 'pack', paymentProvider: 'creem', currency: 'eur' }),
    ])
    expect(isCurrencySwitchable(group)).toBe(false)
  })

  it('is false for a mixed-provider entitlement (degrades rather than hiding rows)', () => {
    const [group] = groupByEntitlement([
      makeOption({ entitlementKey: 'pack', paymentProvider: 'stripe', currency: 'usd' }),
      makeOption({ entitlementKey: 'pack', paymentProvider: 'wechat', currency: null }),
    ])
    expect(isCurrencySwitchable(group)).toBe(false)
  })
})

describe('isPreferredCurrencyUnavailable', () => {
  it('is true when an effective preference exists but the product lacks it', () => {
    expect(isPreferredCurrencyUnavailable(['USD', 'EUR'], 'CNY', null)).toBe(true)
    expect(isPreferredCurrencyUnavailable(['USD', 'EUR'], null, 'CNY')).toBe(true)
  })

  it('is false when the effective preference is available', () => {
    expect(isPreferredCurrencyUnavailable(['USD', 'EUR'], 'usd', null)).toBe(false)
  })

  it('is false when no preference is configured at all', () => {
    expect(isPreferredCurrencyUnavailable(['USD'], null, null)).toBe(false)
  })
})
