import { describe, it, expect } from 'vitest'
import { step1Schema, type Step1FormData } from '../step-1-schema'

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

    it('should validate with optional description omitted', () => {
      const validData = {
        name: 'Test App',
        appType: 'NATIVE' as const,
        clientType: 'PUBLIC' as const,
      }

      const result = step1Schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('should validate with empty description string', () => {
      const validData = {
        name: 'Test App',
        description: '',
        appType: 'SERVICE' as const,
        clientType: 'CONFIDENTIAL' as const,
      }

      const result = step1Schema.safeParse(validData)
      expect(result.success).toBe(true)
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

    it('should accept all valid app types', () => {
      const validTypes = ['WEB', 'NATIVE', 'SERVICE'] as const

      validTypes.forEach((appType) => {
        const data = {
          name: 'Test App',
          appType,
          clientType: 'CONFIDENTIAL' as const,
        }
        const result = step1Schema.safeParse(data)
        expect(result.success).toBe(true)
      })
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

    it('should accept all valid client types', () => {
      const validTypes = ['PUBLIC', 'CONFIDENTIAL'] as const

      validTypes.forEach((clientType) => {
        const data = {
          name: 'Test App',
          appType: 'WEB' as const,
          clientType,
        }
        const result = step1Schema.safeParse(data)
        expect(result.success).toBe(true)
      })
    })
  })

  describe('TypeScript type inference', () => {
    it('should correctly infer TypeScript types', () => {
      const data: Step1FormData = {
        name: 'Test App',
        description: 'Test description',
        appType: 'WEB',
        clientType: 'CONFIDENTIAL',
      }

      expect(typeof data.name).toBe('string')
      expect(data.appType).toBe('WEB')
      expect(data.clientType).toBe('CONFIDENTIAL')
    })
  })
})
