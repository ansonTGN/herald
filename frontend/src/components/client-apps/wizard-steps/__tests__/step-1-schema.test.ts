import { describe, it, expect } from 'vitest'
import { step1Schema } from '../step-1-schema'

describe('step1Schema', () => {
  describe('valid data', () => {
    it('should validate correct basic information', () => {
      const validData = {
        name: 'Test App',
        description: 'A test application',
        appType: 'WEB' as const,
        clientType: 'CONFIDENTIAL' as const,
      }

      const result = step1Schema.safeParse(validData)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data).toEqual(validData)
      }
    })
  })

  describe('name field validation', () => {
    it('should reject empty name', () => {
      const invalidData = {
        name: '',
        appType: 'WEB' as const,
        clientType: 'CONFIDENTIAL' as const,
      }

      const result = step1Schema.safeParse(invalidData)
      expect(result.success).toBe(false)
    })

    it('should reject name exceeding 100 characters', () => {
      const invalidData = {
        name: 'a'.repeat(101),
        appType: 'WEB' as const,
        clientType: 'CONFIDENTIAL' as const,
      }

      const result = step1Schema.safeParse(invalidData)
      expect(result.success).toBe(false)
    })
  })

  describe('description field validation', () => {
    it('should reject description exceeding 500 characters', () => {
      const invalidData = {
        name: 'Test App',
        description: 'a'.repeat(501),
        appType: 'WEB' as const,
        clientType: 'CONFIDENTIAL' as const,
      }

      const result = step1Schema.safeParse(invalidData)
      expect(result.success).toBe(false)
    })
  })

  describe('appType field validation', () => {
    it('should reject invalid app type', () => {
      const invalidData = {
        name: 'Test App',
        appType: 'INVALID' as any,
        clientType: 'CONFIDENTIAL' as const,
      }

      const result = step1Schema.safeParse(invalidData)
      expect(result.success).toBe(false)
    })
  })

  describe('clientType field validation', () => {
    it('should reject invalid client type', () => {
      const invalidData = {
        name: 'Test App',
        appType: 'WEB' as const,
        clientType: 'INVALID' as any,
      }

      const result = step1Schema.safeParse(invalidData)
      expect(result.success).toBe(false)
    })
  })
})
