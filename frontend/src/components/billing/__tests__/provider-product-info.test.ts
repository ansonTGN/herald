import { describe, it, expect } from 'vitest'
import {
  readProviderProductInfo,
  mapBillingPeriodLabel,
  primaryProductLabel,
  isOneTimeMapping,
} from '../provider-product-info'
import { m } from '@/paraglide/messages'

// Unit-level contracts for the four pure helpers exported from
// provider-product-info.ts: the sole `unknown`-narrowing accessor
// (readProviderProductInfo) and the three derived label/rules helpers.
//
// These are pure functions with no React, no MSW, no rendering — they are
// high-value fast-feedback regression targets. We assert the CONTRACT
// (defensive JSON narrowing, snake_case→camelCase mapping, label fallback,
// period mapping, the one-time field-set rule) directly.

describe('readProviderProductInfo — defensive JSON narrowing', () => {
  it('returns {} for null', () => {
    expect(readProviderProductInfo(null)).toEqual({})
  })

  it.each([undefined, 42, 'string', true])('returns {} for non-object input (%s)', (input) => {
    expect(readProviderProductInfo(input)).toEqual({})
  })

  it('returns {} for an array input (typeof array === "object" but not a record)', () => {
    // Arrays pass the `typeof === 'object'` guard, so the accessor walks known
    // keys and finds none — every pick returns undefined, deep-equal to {}.
    expect(readProviderProductInfo([])).toEqual({})
  })

  it('reads known camelCase keys from a well-formed object', () => {
    const info = readProviderProductInfo({
      name: 'Pro',
      description: 'd',
      price: 999,
      currency: 'usd',
      billingType: 'recurring',
      billingPeriod: 'month',
      productMetadata: { tier: 'pro' },
      priceMetadata: { interval: 'month' },
    })

    // The accessor reads ONLY the documented backend JSONB keys. Four of those
    // keys (`name`, `description`, `price`, `currency`) are spelled identically
    // in camelCase and snake_case, so a well-formed camelCase object surfaces
    // those four verbatim. The other four backend keys are snake_case-only
    // (`billing_type`, `billing_period`, `product_metadata`, `price_metadata`),
    // so the camelCase variants on this object are NOT read and come back
    // undefined → the returned object deep-equals the four shared keys only.
    expect(info).toEqual({
      name: 'Pro',
      description: 'd',
      price: 999,
      currency: 'usd',
      billingType: undefined,
      billingPeriod: undefined,
      productMetadata: undefined,
      priceMetadata: undefined,
    })
  })

  it('maps backend snake_case JSONB keys to camelCase TS fields', () => {
    const info = readProviderProductInfo({
      name: 'Pro Plan',
      description: 'Pro tier',
      price: 1000,
      currency: 'usd',
      billing_type: 'one_time',
      billing_period: 'every-month',
      product_metadata: { tier: 'pro' },
      price_metadata: { interval: 'month' },
    })

    expect(info).toEqual({
      name: 'Pro Plan',
      description: 'Pro tier',
      price: 1000,
      currency: 'usd',
      billingType: 'one_time',
      billingPeriod: 'every-month',
      productMetadata: { tier: 'pro' },
      priceMetadata: { interval: 'month' },
    })
  })

  it('maps each snake_case JSONB key to its camelCase TS field individually', () => {
    // Pin each mapping independently so a regression on a single key surfaces
    // a precise failure (rather than only a whole-object deep-equal).
    const info = readProviderProductInfo({
      product_metadata: { a: 1 },
      price_metadata: { b: 2 },
      billing_type: 'recurring',
      billing_period: 'year',
    })

    expect(info.productMetadata).toEqual({ a: 1 })
    expect(info.priceMetadata).toEqual({ b: 2 })
    expect(info.billingType).toBe('recurring')
    expect(info.billingPeriod).toBe('year')
  })

  it('does not throw on unknown keys or nested garbage', () => {
    const info = readProviderProductInfo({
      name: 'Pro',
      __type: 'surprise',
      nested: { a: { b: null } },
      extra: 1,
    })

    expect(info).toEqual({ name: 'Pro' })
  })

  it('treats a malformed metadata value defensively (returns it as-is, no throw)', () => {
    // FE-D01 contract: the accessor shallow-reads known keys via `pick`, which
    // passes the value through unchanged (the `as T` cast is the ONLY
    // coercion). A non-object `product_metadata` is therefore returned
    // verbatim — the accessor does NOT validate/transform nested shapes, it
    // only narrows the top-level envelope. This is defensive in the sense
    // that it never throws; downstream consumers must tolerate the raw value.
    const asString = readProviderProductInfo({ product_metadata: 'not-an-object' })
    expect(asString.productMetadata).toBe('not-an-object')

    const asNumber = readProviderProductInfo({ product_metadata: 7 })
    expect(asNumber.productMetadata).toBe(7)

    // And a null metadata collapses to undefined (treated as an absent field),
    // so the returned object deep-equals {}.
    const asNull = readProviderProductInfo({ product_metadata: null })
    expect(asNull).toEqual({})
  })

  it('treats null values on known keys as absent (collapsed to undefined)', () => {
    // `pick` maps `null` → `undefined`, so explicit-null fields are dropped
    // from the deep-equal snapshot rather than preserved as `null`.
    const info = readProviderProductInfo({
      name: null,
      billing_type: null,
      billing_period: null,
      product_metadata: null,
    })

    expect(info).toEqual({})
  })
})

describe('mapBillingPeriodLabel', () => {
  it.each([
    ['every-month', m['billing.billing_period_month']()],
    ['month', m['billing.billing_period_month']()],
    ['every-year', m['billing.billing_period_year']()],
    ['year', m['billing.billing_period_year']()],
  ])('maps %s to the localized label', (input, expected) => {
    expect(mapBillingPeriodLabel(input)).toBe(expected)
  })

  it('returns the raw string for unmapped values', () => {
    expect(mapBillingPeriodLabel('quarterly')).toBe('quarterly')
  })

  it.each([null, undefined, ''])('returns empty string for null/undefined/empty (%s)', (input) => {
    expect(mapBillingPeriodLabel(input)).toBe('')
  })
})

describe('primaryProductLabel', () => {
  it('prefers productName when present', () => {
    expect(primaryProductLabel('Pro Plan', 'prod_1')).toBe('Pro Plan')
  })

  it.each([
    [undefined, 'prod_1'],
    [null, 'prod_1'],
  ] as const)(
    'falls back to externalProductId when name is missing (%s)',
    (name, externalProductId) => {
      expect(primaryProductLabel(name, externalProductId)).toBe('prod_1')
    }
  )

  it('treats an empty-string name as present (no fallback); the page composes `|| placeholder`', () => {
    // FE-D02 contract: `primaryProductLabel` uses nullish-coalescing
    // (`name ?? externalProductId ?? ''`), so an empty-string name is NOT
    // nullish and does not fall back here. The page renders the i18n placeholder
    // via `primaryProductLabel(...) || m['billing.product_name_empty']()`,
    // so empty-string still surfaces as the placeholder at the page layer.
    expect(primaryProductLabel('', 'prod_1')).toBe('')
  })

  it('returns empty string when both are missing', () => {
    expect(primaryProductLabel(undefined, undefined)).toBe('')
  })
})

describe('isOneTimeMapping', () => {
  it('returns true only for one_time', () => {
    expect(isOneTimeMapping('one_time')).toBe(true)
  })

  it.each(['recurring', null, undefined, '', 'subscription'])(
    'returns false for recurring, null, undefined, and unknown values (%s)',
    (input) => {
      expect(isOneTimeMapping(input)).toBe(false)
    }
  )
})
