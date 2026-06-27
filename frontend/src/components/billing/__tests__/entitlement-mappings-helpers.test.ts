import { describe, it, expect } from 'vitest'
import { groupByProduct, groupByEntitlementKey } from '../entitlement-mapping-grouping'
import { deriveSharedKeyColor } from '../shared-key-color'
import { makeMapping, multiPriceMappingList } from '@/test/fixtures/entitlement-mappings'

// Unit-level contracts for the admin master-detail grouping + shared-key color
// helpers. The dev component test (entitlement-mappings-page.test.tsx) exercises
// these only via rendered DOM; here we assert the CONTRACT directly — ordering,
// grouping shape, determinism, boundary (empty). No MSW, no rendering.

describe('grouping multi-price mappings by product', () => {
  describe('groupByProduct', () => {
    it('groups a flat multi-price list into one entry per (provider, product)', () => {
      const items = multiPriceMappingList()

      const groups = groupByProduct(items)

      // prod_pro (monthly + annual) collapses to one entry; prod_starter is its own.
      expect(groups).toHaveLength(2)
      expect(groups[0].paymentProvider).toBe('stripe')
      expect(groups[0].externalProductId).toBe('prod_pro')
      expect(groups[0].prices).toHaveLength(2)
      // The price-less Creem product is its own group despite externalPriceId: null.
      expect(groups[1].paymentProvider).toBe('creem')
      expect(groups[1].externalProductId).toBe('prod_starter')
      expect(groups[1].prices).toHaveLength(1)
    })

    it('preserves first-seen group order regardless of later rows', () => {
      // Deliberately reorder so prod_starter appears first in the input.
      const items = [
        makeMapping({ externalProductId: 'prod_starter', paymentProvider: 'creem' }),
        makeMapping({ externalProductId: 'prod_pro', paymentProvider: 'stripe' }),
        makeMapping({
          externalProductId: 'prod_pro',
          externalPriceId: 'price_pro_annual',
          paymentProvider: 'stripe',
        }),
      ]

      const groups = groupByProduct(items)

      // First-seen order: starter group must come first even though the flat
      // list is alphabetically different.
      expect(groups.map((g) => g.externalProductId)).toEqual(['prod_starter', 'prod_pro'])
    })

    it('keeps each group prices in input order', () => {
      const monthly = makeMapping({
        externalPriceId: 'price_pro_monthly',
        billingPeriod: 'month',
      })
      const annual = makeMapping({
        externalPriceId: 'price_pro_annual',
        billingPeriod: 'year',
      })

      const groups = groupByProduct([annual, monthly])

      expect(groups[0].prices[0].externalPriceId).toBe('price_pro_annual')
      expect(groups[0].prices[1].externalPriceId).toBe('price_pro_monthly')
    })

    it('returns an empty array for empty input (boundary)', () => {
      expect(groupByProduct([])).toEqual([])
    })
  })

  describe('groupByEntitlementKey', () => {
    it('groups a product prices by entitlementKey (monthly+annual share one group)', () => {
      const prodPro = multiPriceMappingList().filter((m) => m.externalProductId === 'prod_pro')

      const keyGroups = groupByEntitlementKey(prodPro)

      // Both prod_pro prices share the pro-plan key → a single group.
      expect(keyGroups).toHaveLength(1)
      expect(keyGroups[0].entitlementKey).toBe('pro-plan')
      expect(keyGroups[0].prices).toHaveLength(2)
    })

    it('separates prices with different entitlementKeys in first-seen order', () => {
      const prices = [
        makeMapping({ entitlementKey: 'pro-plan', externalPriceId: 'a' }),
        makeMapping({ entitlementKey: 'starter', externalPriceId: 'b' }),
        makeMapping({ entitlementKey: 'pro-plan', externalPriceId: 'c' }),
      ]

      const keyGroups = groupByEntitlementKey(prices)

      expect(keyGroups.map((g) => g.entitlementKey)).toEqual(['pro-plan', 'starter'])
      // The later pro-plan price merges into the first group, preserving order.
      expect(keyGroups[0].prices.map((p) => p.externalPriceId)).toEqual(['a', 'c'])
    })

    it('returns an empty array for empty input (boundary)', () => {
      expect(groupByEntitlementKey([])).toEqual([])
    })
  })
})

describe('shared-key color derivation', () => {
  // Contract: the SAME key always maps to the SAME hue so the admin can scan
  // which prices share a key. We assert STABILITY, not a specific hue value
  // (the algorithm is implementation; determinism is the contract).
  it.each(['pro-plan', 'starter', 'a', 'team-2026', 'some-very-long-entitlement-key-name-12345'])(
    'derives a stable hue for key %s across repeated calls',
    (key) => {
      const first = deriveSharedKeyColor(key)
      const second = deriveSharedKeyColor(key)

      expect(first.hue).toBe(second.hue)
      expect(first.className).toBe(second.className)
      // hue is always within the valid HSL range.
      expect(first.hue).toBeGreaterThanOrEqual(0)
      expect(first.hue).toBeLessThan(360)
    }
  )

  it('returns equal results for the same key called from independent call sites', () => {
    // Two different code paths deriving the color for the same key must agree.
    const fromProductA = deriveSharedKeyColor('shared-key-xyz')
    const fromProductB = deriveSharedKeyColor('shared-key-xyz')

    expect(fromProductA).toEqual(fromProductB)
  })

  it('does not require two DIFFERENT keys to produce different hues (no anti-collision contract)', () => {
    // The contract is stability per key, not uniqueness across keys. We only
    // assert that calling with two different keys returns *some* valid value;
    // a hash collision between two keys is acceptable behavior, not a bug.
    const a = deriveSharedKeyColor('key-one')
    const b = deriveSharedKeyColor('key-two')

    expect(a.hue).toBeGreaterThanOrEqual(0)
    expect(a.hue).toBeLessThan(360)
    expect(b.hue).toBeGreaterThanOrEqual(0)
    expect(b.hue).toBeLessThan(360)
  })

  it('collapses an empty key to the neutral fallback (no misleading shared color)', () => {
    const empty = deriveSharedKeyColor('')

    expect(empty.hue).toBe(0)
    expect(empty.className).toBe('bg-muted-foreground')
  })
})
