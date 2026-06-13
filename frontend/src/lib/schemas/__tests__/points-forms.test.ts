import { describe, it, expect } from 'vitest'
import { transactionFiltersSchema, accountFiltersSchema } from '../points-forms'

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
