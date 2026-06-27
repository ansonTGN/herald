import { describe, it, expect } from 'vitest'
import {
  priceMappingUpdateSchema,
  batchEntitlementMappingsSchema,
  getBatchEntitlementMappingsDefaults,
} from '../billing-forms'
import { makeMapping } from '@/test/fixtures/entitlement-mappings'

// Unit-level Zod contracts for the price-granularity batch editor. Mirrors
// the generated `PriceMappingUpdate` / `BatchUpdateEntitlementMappingsRequest`.
// Asserts boundary values, regex accept/reject, required fields, cross-field
// propagation, and defaults shape. No MSW, no rendering.

/** Factory for a single valid update payload (entitlement-keyed by pro-plan). */
function validUpdate(overrides: Record<string, unknown> = {}) {
  return {
    mappingId: 'map_pro_monthly',
    entitlementKey: 'pro-plan',
    ...overrides,
  }
}

/** Factory for a valid batch envelope wrapping one update. */
function validBatch(overrides: Record<string, unknown> = {}) {
  return {
    paymentProvider: 'stripe',
    externalProductId: 'prod_pro',
    updates: [validUpdate()],
    ...overrides,
  }
}

describe('priceMappingUpdateSchema entitlement_key regex', () => {
  describe('accepts valid keys', () => {
    it.each([
      ['lowercase with hyphen', 'pro-plan'],
      ['single character', 'a'],
      ['digits with hyphens', '1-2-3'],
      ['exactly 64 chars of [a-z0-9-]', 'a'.repeat(64)],
      ['mixed lowercase alnum', 'abc123def'],
    ])('accepts %s: %s', (_label, key) => {
      const result = priceMappingUpdateSchema.safeParse(validUpdate({ entitlementKey: key }))
      expect(result.success).toBe(true)
    })
  })

  describe('rejects invalid keys', () => {
    it.each([
      ['uppercase letter', 'Pro-Plan'],
      ['underscore', 'pro_plan'],
      ['space', 'pro plan'],
      ['empty string', ''],
      ['65 chars (over limit)', 'a'.repeat(65)],
      ['dot', 'pro.'],
    ])('rejects %s: %s', (_label, key) => {
      const result = priceMappingUpdateSchema.safeParse(validUpdate({ entitlementKey: key }))
      expect(result.success).toBe(false)
    })
  })
})

describe('priceMappingUpdateSchema required fields', () => {
  it('fails when mappingId is missing', () => {
    const { mappingId: _omit, ...withoutMappingId } = validUpdate()
    void _omit

    const result = priceMappingUpdateSchema.safeParse(withoutMappingId)

    expect(result.success).toBe(false)
  })

  it('fails when mappingId is an empty string', () => {
    const result = priceMappingUpdateSchema.safeParse(validUpdate({ mappingId: '' }))
    expect(result.success).toBe(false)
  })

  it('fails when entitlementKey is missing', () => {
    const { entitlementKey: _omit, ...withoutKey } = validUpdate()
    void _omit

    const result = priceMappingUpdateSchema.safeParse(withoutKey)

    expect(result.success).toBe(false)
  })

  it('accepts a row with only the two required fields (optionals omitted)', () => {
    const result = priceMappingUpdateSchema.safeParse({
      mappingId: 'map_1',
      entitlementKey: 'pro-plan',
    })
    expect(result.success).toBe(true)
  })
})

describe('priceMappingUpdateSchema numeric / enum guards', () => {
  it.each([
    ['negative pointsPerPeriod', { pointsPerPeriod: -1 }],
    ['fractional pointsPerPeriod', { pointsPerPeriod: 1.5 }],
    ['zero validityDays', { validityDays: 0 }],
    ['grantPeriodType outside enum', { grantPeriodType: 'hourly' }],
  ])('rejects %s', (_label, overrides) => {
    const result = priceMappingUpdateSchema.safeParse(validUpdate(overrides))
    expect(result.success).toBe(false)
  })

  it.each([
    ['zero pointsPerPeriod', { pointsPerPeriod: 0 }],
    ['grantPeriodType once', { grantPeriodType: 'once' }],
    ['grantPeriodType daily', { grantPeriodType: 'daily' }],
    ['grantPeriodType weekly', { grantPeriodType: 'weekly' }],
    ['grantPeriodType monthly', { grantPeriodType: 'monthly' }],
  ])('accepts %s', (_label, overrides) => {
    const result = priceMappingUpdateSchema.safeParse(validUpdate(overrides))
    expect(result.success).toBe(true)
  })
})

describe('batchEntitlementMappingsSchema', () => {
  it('wraps the updates array and requires paymentProvider / externalProductId', () => {
    const result = batchEntitlementMappingsSchema.safeParse(validBatch())
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.updates).toHaveLength(1)
      expect(result.data.paymentProvider).toBe('stripe')
      expect(result.data.externalProductId).toBe('prod_pro')
    }
  })

  it('fails when paymentProvider is missing', () => {
    const { paymentProvider: _omit, ...rest } = validBatch()
    void _omit
    expect(batchEntitlementMappingsSchema.safeParse(rest).success).toBe(false)
  })

  it('fails when externalProductId is missing', () => {
    const { externalProductId: _omit, ...rest } = validBatch()
    void _omit
    expect(batchEntitlementMappingsSchema.safeParse(rest).success).toBe(false)
  })

  it('fails when updates is empty (at least one row required)', () => {
    const result = batchEntitlementMappingsSchema.safeParse(validBatch({ updates: [] }))
    expect(result.success).toBe(false)
  })

  it('fails the WHOLE batch when any update has a bad entitlementKey (cross-field propagation)', () => {
    const result = batchEntitlementMappingsSchema.safeParse({
      paymentProvider: 'stripe',
      externalProductId: 'prod_pro',
      updates: [
        validUpdate({ entitlementKey: 'pro-plan' }),
        validUpdate({ entitlementKey: 'Pro_Plan' }), // regex violation
      ],
    })

    expect(result.success).toBe(false)
  })

  it('accepts a batch seeded from a real fixture mapping', () => {
    const mapping = makeMapping()
    const result = batchEntitlementMappingsSchema.safeParse({
      paymentProvider: mapping.paymentProvider,
      externalProductId: mapping.externalProductId,
      updates: [
        {
          mappingId: mapping.id,
          entitlementKey: mapping.entitlementKey,
          billingType: mapping.billingType,
          billingPeriod: mapping.billingPeriod,
          enabled: mapping.enabled,
        },
      ],
    })
    expect(result.success).toBe(true)
  })
})

describe('getBatchEntitlementMappingsDefaults', () => {
  it('returns the empty-defaults shape when called with no config', () => {
    const defaults = getBatchEntitlementMappingsDefaults()

    // Assert SHAPE only — optional fields are intentionally empty by default.
    expect(defaults.paymentProvider).toBe('')
    expect(defaults.externalProductId).toBe('')
    expect(defaults.updates).toEqual([])
  })

  it('seeds paymentProvider / externalProductId from config', () => {
    const defaults = getBatchEntitlementMappingsDefaults({
      paymentProvider: 'stripe',
      externalProductId: 'prod_pro',
    })

    expect(defaults.paymentProvider).toBe('stripe')
    expect(defaults.externalProductId).toBe('prod_pro')
    // Unspecified collections stay empty.
    expect(defaults.updates).toEqual([])
  })

  it('seeds the updates array from config without mutating the input', () => {
    const seededUpdates = [{ mappingId: 'map_1', entitlementKey: 'pro-plan' }]

    const defaults = getBatchEntitlementMappingsDefaults({
      paymentProvider: 'stripe',
      externalProductId: 'prod_pro',
      updates: seededUpdates,
    })

    expect(defaults.updates).toBe(seededUpdates)
  })
})
