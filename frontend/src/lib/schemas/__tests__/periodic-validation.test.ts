import { describe, it, expect } from 'vitest'
import { realmConfigSchema } from '../points-forms'

describe('Periodic Validation Tests (P0)', () => {
  describe('Test 2.1: Grant Period Type Enum Validation', () => {
    it('GIVEN periodType is "once" WHEN validating complete object THEN should pass', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'once',
        freePeriodicValidityDays: 0,
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.freePeriodicGrantPeriodType).toBe('once')
      }
    })

    it('GIVEN periodType is "daily" WHEN validating THEN should pass', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'daily',
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.freePeriodicGrantPeriodType).toBe('daily')
      }
    })

    it('GIVEN periodType is "weekly" WHEN validating THEN should pass', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'weekly',
        freePeriodicValidityDays: 7,
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.freePeriodicGrantPeriodType).toBe('weekly')
      }
    })

    it('GIVEN periodType is "monthly" WHEN validating THEN should pass', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'monthly',
        freePeriodicValidityDays: 30,
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.freePeriodicGrantPeriodType).toBe('monthly')
      }
    })

    it('GIVEN periodType is "yearly" WHEN validating THEN should fail', () => {
      const result = realmConfigSchema.safeParse({
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
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'hourly' as any,
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(false)
    })

    it('GIVEN periodType is empty string WHEN validating THEN should fail', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: '' as any,
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(false)
    })

    it('GIVEN periodType is null WHEN validating THEN should fail', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: null as any,
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(false)
    })

    it('GIVEN periodType is undefined WHEN validating THEN should fail', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: undefined as any,
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(false)
    })

    it('GIVEN periodType is "ONCE" (uppercase) WHEN validating THEN should fail', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'ONCE' as any,
        freePeriodicValidityDays: 0,
      })

      expect(result.success).toBe(false)
    })

    it('GIVEN periodType is "DAILY" (uppercase) WHEN validating THEN should fail', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'DAILY' as any,
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(false)
    })
  })

  describe('Test 2.2: Period Type and Validity Days Validation Logic', () => {
    it('GIVEN periodType is "once" and validityDays is 0 WHEN validating THEN should pass', () => {
      const result = realmConfigSchema.safeParse({
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

    it('GIVEN periodType is "once" and validityDays is 30 WHEN validating THEN should pass', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'once',
        freePeriodicValidityDays: 30,
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.freePeriodicValidityDays).toBe(30)
      }
    })

    it('GIVEN periodType is "daily" and validityDays is 0 WHEN validating THEN should fail', () => {
      const result = realmConfigSchema.safeParse({
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

    it('GIVEN periodType is "daily" and validityDays is 1 WHEN validating THEN should pass', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'daily',
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.freePeriodicValidityDays).toBe(1)
      }
    })

    it('GIVEN periodType is "weekly" and validityDays is 7 WHEN validating THEN should pass', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'weekly',
        freePeriodicValidityDays: 7,
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.freePeriodicValidityDays).toBe(7)
      }
    })

    it('GIVEN periodType is "weekly" and validityDays is 0 WHEN validating THEN should fail', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'weekly',
        freePeriodicValidityDays: 0,
      })

      expect(result.success).toBe(false)
    })

    it('GIVEN periodType is "monthly" and validityDays is 30 WHEN validating THEN should pass', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'monthly',
        freePeriodicValidityDays: 30,
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.freePeriodicValidityDays).toBe(30)
      }
    })

    it('GIVEN periodType is "monthly" and validityDays is 0 WHEN validating THEN should fail', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'monthly',
        freePeriodicValidityDays: 0,
      })

      expect(result.success).toBe(false)
    })
  })

  describe('Additional Periodic Field Validation', () => {
    it('GIVEN valid periodic points amount WHEN validating THEN should pass', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'daily',
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.freePeriodicPointsAmount).toBe(50)
      }
    })

    it('GIVEN zero periodic points amount WHEN validating THEN should pass', () => {
      const result = realmConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 0,
        freePeriodicGrantPeriodType: 'daily',
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.freePeriodicPointsAmount).toBe(0)
      }
    })

    it('GIVEN negative periodic points amount WHEN validating THEN should fail', () => {
      const result = realmConfigSchema.safeParse({
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
      const result = realmConfigSchema.safeParse({
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
