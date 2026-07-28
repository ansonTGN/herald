import { describe, it, expect } from 'vitest'
import {
  createEntitlementMappingSchema,
  getCreateEntitlementMappingDefaults,
} from '../create-entitlement-mapping'

/**
 * Base factory for a fully-valid create-mapping form. Individual tests override
 * the fields they want to invalidate so the schema boundary under test is the
 * only moving part.
 */
function makeValidForm(overrides: Record<string, unknown> = {}) {
  return {
    paymentProvider: 'apple',
    externalProductId: 'com.example.app.premium',
    externalPriceId: null,
    entitlementKey: 'premium',
    bucketId: 'bucket-1',
    billingType: 'recurring',
    billingPeriod: 'monthly',
    pointsPerPeriod: null,
    grantOnSubscribe: false,
    validityDays: null,
    grantedRoleIds: [],
    enabled: true,
    ...overrides,
  }
}

describe('createEntitlementMappingSchema', () => {
  it('accepts a valid recurring mapping with billingPeriod', () => {
    const result = createEntitlementMappingSchema.safeParse(makeValidForm())

    expect(result.success).toBe(true)
  })

  it('rejects recurring without billingPeriod (cross-field refinement)', () => {
    // Core cross-field constraint (support-iap §4.4.2 / §4.2.2):
    // billingType === 'recurring' ⇒ billingPeriod is mandatory. A null/missing
    // billingPeriod on a recurring row must be flagged at the billingPeriod
    // path — Demo never reaches this branch because the form hides submit until
    // the field is filled, so this Vitest is the only coverage.
    const result = createEntitlementMappingSchema.safeParse(makeValidForm({ billingPeriod: null }))

    expect(result.success).toBe(false)
    if (!result.success) {
      const paths = result.error.issues.map((issue) => String(issue.path[0]))
      expect(paths).toContain('billingPeriod')
    }
  })

  it('rejects recurring with a missing billingPeriod field entirely', () => {
    const { billingPeriod: _omit, ...withoutPeriod } = makeValidForm()

    const result = createEntitlementMappingSchema.safeParse(withoutPeriod)

    expect(result.success).toBe(false)
    if (!result.success) {
      const paths = result.error.issues.map((issue) => String(issue.path[0]))
      expect(paths).toContain('billingPeriod')
    }
  })

  it('accepts one_time without billingPeriod', () => {
    const result = createEntitlementMappingSchema.safeParse(
      makeValidForm({
        billingType: 'one_time',
        billingPeriod: null,
        // one_time rows have no recurring points; validityDays is the relevant field.
        validityDays: 30,
      })
    )

    expect(result.success).toBe(true)
  })

  it.each(['', null])('rejects an empty/missing billingType (%s)', (billingType) => {
    const result = createEntitlementMappingSchema.safeParse(makeValidForm({ billingType }))

    expect(result.success).toBe(false)
    if (!result.success) {
      const paths = result.error.issues.map((issue) => String(issue.path[0]))
      expect(paths).toContain('billingType')
    }
  })

  it('rejects an invalid billingPeriod enum value', () => {
    const result = createEntitlementMappingSchema.safeParse(
      makeValidForm({ billingPeriod: 'weekly' as unknown })
    )

    expect(result.success).toBe(false)
    if (!result.success) {
      const paths = result.error.issues.map((issue) => String(issue.path[0]))
      expect(paths).toContain('billingPeriod')
    }
  })

  it.each(['paymentProvider', 'externalProductId', 'entitlementKey', 'bucketId'])(
    'rejects an empty required field: %s',
    (field) => {
      const result = createEntitlementMappingSchema.safeParse(makeValidForm({ [field]: '' }))

      expect(result.success).toBe(false)
      if (!result.success) {
        const paths = result.error.issues.map((issue) => String(issue.path[0]))
        expect(paths).toContain(field)
      }
    }
  )

  it('allows externalPriceId to be null/optional (IAP & Creem leave it empty)', () => {
    const result = createEntitlementMappingSchema.safeParse(
      makeValidForm({ externalPriceId: null })
    )

    expect(result.success).toBe(true)
  })

  it('getCreateEntitlementMappingDefaults returns a baseline the schema accepts once required fields are filled', () => {
    // Guards against the defaults themselves drifting into an invalid shape
    // (e.g. billingType defaulting to 'recurring' while billingPeriod is null).
    const defaults = getCreateEntitlementMappingDefaults()

    // Fill only the human-entered required strings; billingType stays '' so no
    // recurring refinement triggers.
    const result = createEntitlementMappingSchema.safeParse({
      ...defaults,
      paymentProvider: 'google',
      externalProductId: 'com.example.app.gold',
      entitlementKey: 'gold',
      bucketId: 'bucket-1',
      billingType: 'one_time',
    })

    expect(result.success).toBe(true)
  })
})
