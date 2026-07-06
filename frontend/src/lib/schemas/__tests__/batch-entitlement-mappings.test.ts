import { describe, it, expect } from 'vitest'
import {
  priceMappingUpdateSchema,
  batchEntitlementMappingsSchema,
  getBatchEntitlementMappingsDefaults,
} from '../billing-forms'
import { makeMapping } from '@/test/fixtures/entitlement-mappings'

// Unit-level Zod contracts for the price-granularity batch editor. Mirrors
// the generated `PriceMappingUpdate` / `BatchUpdateEntitlementMappingsRequest`.
// Asserts required fields, numeric/enum guards, and defaults shape. No MSW, no rendering.

/** Factory for a single valid update payload. */
function validUpdate(overrides: Record<string, unknown> = {}) {
  return {
    mappingId: 'map_pro_monthly',
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

  it('accepts a row with only mappingId (optionals omitted)', () => {
    const result = priceMappingUpdateSchema.safeParse({
      mappingId: 'map_1',
    })
    expect(result.success).toBe(true)
  })
})

describe('priceMappingUpdateSchema numeric / enum guards', () => {
  it.each([
    ['negative pointsPerPeriod', { pointsPerPeriod: -1 }],
    ['fractional pointsPerPeriod', { pointsPerPeriod: 1.5 }],
    ['zero validityDays', { validityDays: 0 }],
  ])('rejects %s', (_label, overrides) => {
    const result = priceMappingUpdateSchema.safeParse(validUpdate(overrides))
    expect(result.success).toBe(false)
  })

  it.each([
    ['zero pointsPerPeriod', { pointsPerPeriod: 0 }],
    ['validityDays', { validityDays: 30 }],
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

  it('accepts a batch seeded from a real fixture mapping', () => {
    const mapping = makeMapping()
    const result = batchEntitlementMappingsSchema.safeParse({
      paymentProvider: mapping.paymentProvider,
      externalProductId: mapping.externalProductId,
      updates: [
        {
          mappingId: mapping.id,
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
    const seededUpdates = [{ mappingId: 'map_1' }]

    const defaults = getBatchEntitlementMappingsDefaults({
      paymentProvider: 'stripe',
      externalProductId: 'prod_pro',
      updates: seededUpdates,
    })

    expect(defaults.updates).toBe(seededUpdates)
  })
})
