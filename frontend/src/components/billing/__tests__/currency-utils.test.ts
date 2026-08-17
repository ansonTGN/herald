import { describe, it, expect } from 'vitest'
import { isValidCurrencyCode, normalizeCurrencyCode } from '@/lib/currency-utils'
import { groupByCurrency, groupByEntitlement, isCurrencySwitchable } from '../currency-utils'
import { makePurchaseOption as makeOption } from '@/test/fixtures/purchase-option'

// Contracts for the currency-selection helpers. The load-bearing rules:
//   - Stripe-synced catalog rows carry lowercase codes ("usd") — grouping
//     must be case-insensitive and normalized to uppercase.
//   - Only all-Stripe multi-currency entitlements get a currency switcher;
//     store-priced providers and single-currency products degrade.
//   - There is no default currency: the user explicitly picks one before
//     price rows show (a single currency is the only choice).

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
    // switcher segments for the same currency.
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
