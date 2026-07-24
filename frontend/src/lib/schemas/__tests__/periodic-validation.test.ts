import { describe, it, expect } from 'vitest'
import { pointsDefaultConfigSchema } from '../points-forms'

describe('Periodic Validation Tests (P0)', () => {
  describe('Test 2.1: Grant Period Type Enum Validation', () => {
    it('GIVEN periodType is "yearly" WHEN validating THEN should fail', () => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'yearly' as any,
        freePeriodicValidityDays: 365,
      })

      // Zod enum validation should fail for invalid values
      expect(result.success).toBe(false)
      if (!result.success) {
        // Check that there's at least an error
        expect(result.error.issues.length).toBeGreaterThan(0)
      }
    })

    it('GIVEN periodType is "hourly" WHEN validating THEN should fail', () => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'hourly' as any,
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(false)
    })

    it('GIVEN periodType is empty string WHEN validating THEN should fail', () => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: '' as any,
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(false)
    })

    it('GIVEN periodType is null WHEN validating THEN should fail', () => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: null as any,
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(false)
    })

    it('GIVEN periodType is undefined WHEN validating THEN should fail', () => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: undefined as any,
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(false)
    })
  })

  describe('Test 2.2: Period Type and Validity Days Validation Logic', () => {
    it('GIVEN periodType is "once" and validityDays is 0 WHEN validating THEN should pass', () => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'once',
        freePeriodicValidityDays: 0,
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.freePeriodicValidityDays).toBe(0)
      }
    })

    it('GIVEN periodType is "daily" and validityDays is 0 WHEN validating THEN should fail', () => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'daily',
        freePeriodicValidityDays: 0,
      })

      expect(result.success).toBe(false)
      if (!result.success) {
        expect(
          result.error.issues.some(
            (issue) => issue.message.includes('non-once periods') || issue.message.includes('>= 1')
          )
        ).toBe(true)
      }
    })

    it('GIVEN periodType is "weekly" and validityDays is 0 WHEN validating THEN should fail', () => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'weekly',
        freePeriodicValidityDays: 0,
      })

      expect(result.success).toBe(false)
    })

    it('GIVEN periodType is "monthly" and validityDays is 0 WHEN validating THEN should fail', () => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'monthly',
        freePeriodicValidityDays: 0,
      })

      expect(result.success).toBe(false)
    })
  })

  describe('Additional Periodic Field Validation', () => {
    it('GIVEN negative periodic points amount WHEN validating THEN should fail', () => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: -50,
        freePeriodicGrantPeriodType: 'daily',
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(false)
      if (!result.success) {
        expect(
          result.error.issues.some((issue) => issue.message.toLowerCase().includes('negative'))
        ).toBe(true)
      }
    })

    it('GIVEN decimal periodic points amount WHEN validating THEN should fail', () => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50.5,
        freePeriodicGrantPeriodType: 'daily',
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(false)
      if (!result.success) {
        expect(
          result.error.issues.some((issue) => issue.message.toLowerCase().includes('integer'))
        ).toBe(true)
      }
    })
  })
})

describe('freePeriodicQuotaWindows array contract', () => {
  // WHY: the schema is the MultiWindowQuotaEditor's save gate (onChange
  // validator). These cases pin the accept/reject boundary so a future
  // loosening (e.g. dropping .max(8) or the per-window min) fails the
  // editor's own protection.
  it.each([
    ['accepts a valid windows array', [{ windowSeconds: 3600, limit: 0 }], true],
    ['accepts an empty array (clear semantics)', [], true],
    ['accepts exactly 8 windows (the cap)', Array(8).fill({ windowSeconds: 60, limit: 1 }), true],
    ['rejects windowSeconds 0', [{ windowSeconds: 0, limit: 1 }], false],
    ['rejects a negative limit', [{ windowSeconds: 60, limit: -1 }], false],
  ])(
    '%s',
    (_label: string, windows: Array<{ windowSeconds: number; limit: number }>, ok: boolean) => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'daily',
        freePeriodicValidityDays: 1,
        freePeriodicQuotaWindows: windows,
      })
      expect(result.success).toBe(ok)
    }
  )

  it('rejects more than 8 windows', () => {
    const result = pointsDefaultConfigSchema.safeParse({
      registrationBonusPoints: 1000,
      freePeriodicPointsAmount: 50,
      freePeriodicGrantPeriodType: 'daily',
      freePeriodicValidityDays: 1,
      freePeriodicQuotaWindows: Array(9).fill({ windowSeconds: 60, limit: 1 }),
    })
    expect(result.success).toBe(false)
  })
})
