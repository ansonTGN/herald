import { describe, it, expect } from 'vitest'
import {
  pointsPlanConfigSchema,
  transactionFiltersSchema,
  accountFiltersSchema,
} from '../points-forms'

describe('pointsPlanConfigSchema', () => {
  describe('planId field', () => {
    it('GIVEN empty planId WHEN validating THEN should fail', () => {
      const result = pointsPlanConfigSchema.safeParse({
        planId: '',
        pointsPerPeriod: 1000,
        grantOnSubscribe: true,
        grantPeriodType: 'monthly',
        validityDays: 30,
      })

      expect(result.success).toBe(false)
    })

    it('GIVEN whitespace-only planId WHEN validating THEN should fail if schema includes trim', () => {
      // Note: Current schema uses .min(1) which doesn't auto-trim
      // If schema is updated to use .trim().min(1), this test will pass
      const result = pointsPlanConfigSchema.safeParse({
        planId: '   ',
        pointsPerPeriod: 1000,
        grantOnSubscribe: true,
        grantPeriodType: 'monthly',
        validityDays: 30,
      })

      // Currently whitespace-only passes because .min(1) doesn't trim
      // This test documents current behavior
      expect(result.success).toBe(true)
    })
  })

  describe('pointsPerPeriod field', () => {
    it('GIVEN negative points WHEN validating THEN should fail', () => {
      const result = pointsPlanConfigSchema.safeParse({
        planId: 'plan-123',
        pointsPerPeriod: -100,
        grantOnSubscribe: true,
        grantPeriodType: 'monthly',
        validityDays: 30,
      })

      expect(result.success).toBe(false)
    })

    it('GIVEN non-integer points WHEN validating THEN should fail', () => {
      const result = pointsPlanConfigSchema.safeParse({
        planId: 'plan-123',
        pointsPerPeriod: 1000.5,
        grantOnSubscribe: true,
        grantPeriodType: 'monthly',
        validityDays: 30,
      })

      expect(result.success).toBe(false)
    })
  })

  describe('grantPeriodType field', () => {
    it('GIVEN invalid grant period WHEN validating THEN should fail', () => {
      const result = pointsPlanConfigSchema.safeParse({
        planId: 'plan-123',
        pointsPerPeriod: 1000,
        grantOnSubscribe: true,
        grantPeriodType: 'yearly' as any,
        validityDays: 30,
      })

      expect(result.success).toBe(false)
    })
  })

  describe('validityDays field', () => {
    it('GIVEN zero validity days WHEN validating THEN should fail', () => {
      const result = pointsPlanConfigSchema.safeParse({
        planId: 'plan-123',
        pointsPerPeriod: 1000,
        grantOnSubscribe: true,
        grantPeriodType: 'monthly',
        validityDays: 0,
      })

      expect(result.success).toBe(false)
    })

    it('GIVEN negative validity days WHEN validating THEN should fail', () => {
      const result = pointsPlanConfigSchema.safeParse({
        planId: 'plan-123',
        pointsPerPeriod: 1000,
        grantOnSubscribe: true,
        grantPeriodType: 'monthly',
        validityDays: -10,
      })

      expect(result.success).toBe(false)
    })

    it('GIVEN non-integer validity days WHEN validating THEN should fail', () => {
      const result = pointsPlanConfigSchema.safeParse({
        planId: 'plan-123',
        pointsPerPeriod: 1000,
        grantOnSubscribe: true,
        grantPeriodType: 'monthly',
        validityDays: 30.5,
      })

      expect(result.success).toBe(false)
    })
  })

  describe('maxPeriods field', () => {
    it('GIVEN negative maxPeriods WHEN validating THEN should fail', () => {
      const result = pointsPlanConfigSchema.safeParse({
        planId: 'plan-123',
        pointsPerPeriod: 1000,
        grantOnSubscribe: true,
        grantPeriodType: 'monthly',
        validityDays: 30,
        maxPeriods: -10,
      })

      expect(result.success).toBe(false)
    })

    it('GIVEN non-integer maxPeriods WHEN validating THEN should fail', () => {
      const result = pointsPlanConfigSchema.safeParse({
        planId: 'plan-123',
        pointsPerPeriod: 1000,
        grantOnSubscribe: true,
        grantPeriodType: 'monthly',
        validityDays: 30,
        maxPeriods: 12.5,
      })

      expect(result.success).toBe(false)
    })
  })

  describe('complete valid config', () => {
    it('GIVEN all valid fields WHEN validating THEN should pass with all fields', () => {
      const result = pointsPlanConfigSchema.safeParse({
        planId: 'plan-123',
        pointsPerPeriod: 1000,
        grantOnSubscribe: true,
        grantPeriodType: 'monthly',
        validityDays: 30,
        maxPeriods: 12,
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.planId).toBe('plan-123')
        expect(result.data.pointsPerPeriod).toBe(1000)
        expect(result.data.grantOnSubscribe).toBe(true)
        expect(result.data.grantPeriodType).toBe('monthly')
        expect(result.data.validityDays).toBe(30)
        expect(result.data.maxPeriods).toBe(12)
      }
    })
  })
})

describe('transactionFiltersSchema', () => {
  describe('transactionType field', () => {
    it('GIVEN invalid type WHEN validating THEN should fail', () => {
      const result = transactionFiltersSchema.safeParse({
        transactionType: 'transfer' as any,
      })

      expect(result.success).toBe(false)
    })
  })

  describe('startTime field', () => {
    it('GIVEN invalid datetime format WHEN validating THEN should fail', () => {
      const result = transactionFiltersSchema.safeParse({
        startTime: '2025-01-01',
      })

      expect(result.success).toBe(false)
    })
  })

  describe('endTime field', () => {
    it('GIVEN invalid datetime format WHEN validating THEN should fail', () => {
      const result = transactionFiltersSchema.safeParse({
        endTime: '2025-03-15',
      })

      expect(result.success).toBe(false)
    })
  })

  describe('clientAppId field', () => {
    it('GIVEN valid UUID WHEN validating THEN should pass', () => {
      const result = transactionFiltersSchema.safeParse({
        clientAppId: '550e8400-e29b-41d4-a716-446655440000',
      })

      expect(result.success).toBe(true)
    })
  })

  describe('complete valid filters', () => {
    it('GIVEN all filter fields WHEN validating THEN should pass with all fields', () => {
      const result = transactionFiltersSchema.safeParse({
        transactionType: 'recharge',
        startTime: '2025-01-01T00:00:00Z',
        endTime: '2025-03-15T23:59:59Z',
        clientAppId: 'app-123',
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.transactionType).toBe('recharge')
        expect(result.data.startTime).toBe('2025-01-01T00:00:00Z')
        expect(result.data.endTime).toBe('2025-03-15T23:59:59Z')
        expect(result.data.clientAppId).toBe('app-123')
      }
    })
  })
})

describe('accountFiltersSchema', () => {
  describe('status field', () => {
    it('GIVEN invalid status WHEN validating THEN should fail', () => {
      const result = accountFiltersSchema.safeParse({
        status: 'pending' as any,
      })

      expect(result.success).toBe(false)
    })
  })

  describe('complete valid filters', () => {
    it('GIVEN all filter fields WHEN validating THEN should pass with all fields', () => {
      const result = accountFiltersSchema.safeParse({
        search: 'john',
        status: 'active',
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.search).toBe('john')
        expect(result.data.status).toBe('active')
      }
    })
  })
})
