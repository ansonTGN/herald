import { describe, it, expect } from 'vitest'
import {
  entitlementMappingUpdateSchema,
  getEntitlementMappingUpdateDefaults,
} from '../billing-forms'

/** Factory for valid entitlement mapping update data */
function validMapping(overrides: Record<string, unknown> = {}) {
  return {
    entitlementKey: 'pro-plan',
    enabled: false,
    grantOnSubscribe: false,
    ...overrides,
  }
}

describe('entitlementMappingUpdateSchema', () => {
  describe('entitlement_key accepts valid keys', () => {
    it.each([
      ['simple lowercase', 'basic'],
      ['lowercase with hyphens', 'pro-plan'],
      ['single character', 'a'],
      ['numeric', '123'],
      ['mixed alphanumeric with hyphens', 'plan-2024-v2'],
      ['leading hyphen', '-plan'],
      ['trailing hyphen', 'plan-'],
      ['all hyphens', '---'],
      ['max length 64 chars', 'a'.repeat(64)],
    ])('accepts %s: %s', (_label, key) => {
      const result = entitlementMappingUpdateSchema.safeParse(validMapping({ entitlementKey: key }))
      expect(result.success).toBe(true)
    })
  })

  describe('entitlement_key rejects invalid keys', () => {
    it.each([
      ['uppercase letters', 'Pro-Plan'],
      ['underscore', 'pro_plan'],
      ['dot', 'pro.plan'],
      ['space', 'pro plan'],
      ['empty string', ''],
      ['over 64 chars', 'a'.repeat(65)],
      ['special characters', 'pro!plan'],
    ])('rejects %s: %s', (_label, key) => {
      const result = entitlementMappingUpdateSchema.safeParse(validMapping({ entitlementKey: key }))
      expect(result.success).toBe(false)
    })
  })

  describe('points policy field validation', () => {
    it('accepts positive integer for pointsPerPeriod', () => {
      const result = entitlementMappingUpdateSchema.safeParse(
        validMapping({ pointsPerPeriod: 100 })
      )
      expect(result.success).toBe(true)
    })

    it('accepts zero for pointsPerPeriod', () => {
      const result = entitlementMappingUpdateSchema.safeParse(validMapping({ pointsPerPeriod: 0 }))
      expect(result.success).toBe(true)
    })

    it('rejects negative pointsPerPeriod', () => {
      const result = entitlementMappingUpdateSchema.safeParse(validMapping({ pointsPerPeriod: -1 }))
      expect(result.success).toBe(false)
    })

    it('accepts null for pointsPerPeriod', () => {
      const result = entitlementMappingUpdateSchema.safeParse(
        validMapping({ pointsPerPeriod: null })
      )
      expect(result.success).toBe(true)
    })

    it('accepts positive integer for validityDays', () => {
      const result = entitlementMappingUpdateSchema.safeParse(validMapping({ validityDays: 30 }))
      expect(result.success).toBe(true)
    })

    it('rejects zero for validityDays', () => {
      const result = entitlementMappingUpdateSchema.safeParse(validMapping({ validityDays: 0 }))
      expect(result.success).toBe(false)
    })

    it('rejects negative validityDays', () => {
      const result = entitlementMappingUpdateSchema.safeParse(validMapping({ validityDays: -5 }))
      expect(result.success).toBe(false)
    })

    it('accepts positive integer for maxPeriods', () => {
      const result = entitlementMappingUpdateSchema.safeParse(validMapping({ maxPeriods: 12 }))
      expect(result.success).toBe(true)
    })

    it('rejects zero for maxPeriods', () => {
      const result = entitlementMappingUpdateSchema.safeParse(validMapping({ maxPeriods: 0 }))
      expect(result.success).toBe(false)
    })

    it('rejects negative maxPeriods', () => {
      const result = entitlementMappingUpdateSchema.safeParse(validMapping({ maxPeriods: -3 }))
      expect(result.success).toBe(false)
    })
  })

  describe('grantPeriodType enum validation', () => {
    it.each([
      ['once', 'once'],
      ['daily', 'daily'],
      ['weekly', 'weekly'],
      ['monthly', 'monthly'],
    ])('accepts valid enum value: %s', (_label, value) => {
      const result = entitlementMappingUpdateSchema.safeParse(
        validMapping({ grantPeriodType: value })
      )
      expect(result.success).toBe(true)
    })

    it('rejects invalid grantPeriodType', () => {
      const result = entitlementMappingUpdateSchema.safeParse(
        validMapping({ grantPeriodType: 'yearly' })
      )
      expect(result.success).toBe(false)
    })

    it('accepts null for grantPeriodType', () => {
      const result = entitlementMappingUpdateSchema.safeParse(
        validMapping({ grantPeriodType: null })
      )
      expect(result.success).toBe(true)
    })
  })

  describe('grantOnSubscribe accepts boolean', () => {
    it('accepts true', () => {
      const result = entitlementMappingUpdateSchema.safeParse(
        validMapping({ grantOnSubscribe: true })
      )
      expect(result.success).toBe(true)
    })

    it('accepts false', () => {
      const result = entitlementMappingUpdateSchema.safeParse(
        validMapping({ grantOnSubscribe: false })
      )
      expect(result.success).toBe(true)
    })
  })

  describe('getEntitlementMappingUpdateDefaults returns expected defaults', () => {
    it('returns expected default shape', () => {
      const defaults = getEntitlementMappingUpdateDefaults()

      expect(defaults.entitlementKey).toBe('')
      expect(defaults.enabled).toBe(false)
      expect(defaults.pointsPerPeriod).toBeNull()
      expect(defaults.grantPeriodType).toBeNull()
      expect(defaults.validityDays).toBeNull()
      expect(defaults.grantOnSubscribe).toBe(false)
      expect(defaults.maxPeriods).toBeNull()
    })

    it('merges partial overrides with defaults', () => {
      const overrides = {
        entitlementKey: 'test-plan',
        pointsPerPeriod: 50,
      }
      const result = getEntitlementMappingUpdateDefaults(overrides)

      expect(result.entitlementKey).toBe('test-plan')
      expect(result.pointsPerPeriod).toBe(50)
      expect(result.enabled).toBe(false)
      expect(result.grantOnSubscribe).toBe(false)
      expect(result.maxPeriods).toBeNull()
    })
  })

  describe('enabled field accepts boolean values', () => {
    it('accepts true', () => {
      const result = entitlementMappingUpdateSchema.safeParse(validMapping({ enabled: true }))
      expect(result.success).toBe(true)
    })

    it('accepts false', () => {
      const result = entitlementMappingUpdateSchema.safeParse(validMapping({ enabled: false }))
      expect(result.success).toBe(true)
    })

    it('rejects string value for enabled', () => {
      const result = entitlementMappingUpdateSchema.safeParse(validMapping({ enabled: 'true' }))
      expect(result.success).toBe(false)
    })

    it('rejects number value for enabled', () => {
      const result = entitlementMappingUpdateSchema.safeParse(validMapping({ enabled: 1 }))
      expect(result.success).toBe(false)
    })
  })

  describe('enabled and points policy fields have no cross-field constraint', () => {
    it('accepts enabled=false with points policy fields set', () => {
      // Regression guard: both fields are independently valid.
      // No cross-field refine() constraint exists in the schema.
      const result = entitlementMappingUpdateSchema.safeParse({
        entitlementKey: 'test-plan',
        enabled: false,
        pointsPerPeriod: 100,
        grantPeriodType: 'monthly',
        validityDays: 30,
        maxPeriods: 12,
        grantOnSubscribe: true,
      })
      expect(result.success).toBe(true)
    })

    it('accepts enabled=true with points policy fields set', () => {
      const result = entitlementMappingUpdateSchema.safeParse({
        entitlementKey: 'test-plan',
        enabled: true,
        pointsPerPeriod: 100,
        grantPeriodType: 'monthly',
        validityDays: 30,
        maxPeriods: 12,
        grantOnSubscribe: true,
      })
      expect(result.success).toBe(true)
    })

    it('accepts enabled=true with all points policy fields null', () => {
      const result = entitlementMappingUpdateSchema.safeParse({
        entitlementKey: 'test-plan',
        enabled: true,
        pointsPerPeriod: null,
        grantPeriodType: null,
        validityDays: null,
        maxPeriods: null,
        grantOnSubscribe: false,
      })
      expect(result.success).toBe(true)
    })
  })
})
