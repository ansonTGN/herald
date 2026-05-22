import { describe, it, expect } from 'vitest'
import { createApiKeySchema, updateApiKeySchema } from '../api-key-forms'

describe('createApiKeySchema', () => {
  describe('valid inputs', () => {
    it('should accept valid input with name only', () => {
      const result = createApiKeySchema.safeParse({ name: 'My API Key' })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.name).toBe('My API Key')
        expect(result.data.expiresAt).toBeUndefined()
      }
    })

    it('should accept name at boundary length 1', () => {
      const result = createApiKeySchema.safeParse({ name: 'x' })

      expect(result.success).toBe(true)
    })

    it('should accept name at boundary length 100', () => {
      const result = createApiKeySchema.safeParse({ name: 'a'.repeat(100) })

      expect(result.success).toBe(true)
    })

    it('should accept optional expiresAt as a date string', () => {
      const result = createApiKeySchema.safeParse({
        name: 'Expiring Key',
        expiresAt: '2099-12-31T23:59:59Z',
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.expiresAt).toBe('2099-12-31T23:59:59Z')
      }
    })
  })

  describe('invalid inputs', () => {
    it('should reject empty name', () => {
      const result = createApiKeySchema.safeParse({ name: '' })

      expect(result.success).toBe(false)
    })

    it('should reject name exceeding 100 characters', () => {
      const result = createApiKeySchema.safeParse({ name: 'a'.repeat(101) })

      expect(result.success).toBe(false)
    })

    it('should accept whitespace-only name (no .trim() in schema)', () => {
      // Schema uses .min(1) without .trim(), so whitespace-only passes.
      // This test documents current behavior.
      const result = createApiKeySchema.safeParse({ name: '   ' })

      expect(result.success).toBe(true)
    })

    it('should reject non-string expiresAt', () => {
      const result = createApiKeySchema.safeParse({
        name: 'Valid Name',
        expiresAt: 12345,
      })

      expect(result.success).toBe(false)
    })

    it('should strip unknown fields from output', () => {
      const result = createApiKeySchema.safeParse({
        name: 'Valid Name',
        unknownField: 'should be stripped',
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect((result.data as Record<string, unknown>).unknownField).toBeUndefined()
      }
    })
  })
})

describe('updateApiKeySchema', () => {
  describe('valid inputs', () => {
    it('should accept update with name, enabled, and expiresAt', () => {
      const result = updateApiKeySchema.safeParse({
        name: 'Updated Key',
        enabled: true,
        expiresAt: '2099-12-31T23:59:59Z',
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.name).toBe('Updated Key')
        expect(result.data.enabled).toBe(true)
        expect(result.data.expiresAt).toBe('2099-12-31T23:59:59Z')
      }
    })

    it('should accept enabled as boolean false', () => {
      const result = updateApiKeySchema.safeParse({
        name: 'Key',
        enabled: false,
        expiresAt: null,
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.enabled).toBe(false)
      }
    })

    it('should accept expiresAt set to null (clear expiry)', () => {
      const result = updateApiKeySchema.safeParse({
        name: 'Key',
        enabled: true,
        expiresAt: null,
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.expiresAt).toBeNull()
      }
    })
  })

  describe('invalid inputs', () => {
    it('should reject name exceeding 100 characters', () => {
      const result = updateApiKeySchema.safeParse({
        name: 'a'.repeat(101),
        enabled: true,
        expiresAt: null,
      })

      expect(result.success).toBe(false)
    })

    it('should reject empty name', () => {
      const result = updateApiKeySchema.safeParse({
        name: '',
        enabled: true,
        expiresAt: null,
      })

      expect(result.success).toBe(false)
    })

    it('should reject non-boolean enabled value', () => {
      const result = updateApiKeySchema.safeParse({
        name: 'Key',
        enabled: 'yes',
        expiresAt: null,
      })

      expect(result.success).toBe(false)
    })

    it('should reject non-string, non-null expiresAt', () => {
      const result = updateApiKeySchema.safeParse({
        name: 'Key',
        enabled: true,
        expiresAt: 12345,
      })

      expect(result.success).toBe(false)
    })

    it('should reject missing expiresAt (field is required, not optional)', () => {
      // Schema uses .nullable(), not .optional(), so the field must be present
      const result = updateApiKeySchema.safeParse({
        name: 'Key',
        enabled: true,
      })

      expect(result.success).toBe(false)
    })

    it('should strip unknown fields from output', () => {
      const result = updateApiKeySchema.safeParse({
        name: 'Key',
        enabled: true,
        expiresAt: null,
        unknownField: 'should be stripped',
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect((result.data as Record<string, unknown>).unknownField).toBeUndefined()
      }
    })
  })
})
