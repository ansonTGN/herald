import { describe, it, expect } from 'vitest'
import {
  step2Schema,
  type Step2FormData,
  transformToUriItems,
  transformFromUriItems,
} from '../step-2-schema'

describe('step2Schema', () => {
  describe('valid data', () => {
    it('should validate correct redirect URIs', () => {
      const validData = {
        redirectUris: ['https://example.com/callback', 'https://app.example.com/auth'],
      }

      const result = step2Schema.safeParse(validData)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.redirectUris).toEqual(validData.redirectUris)
      }
    })

    it('should validate with http URIs', () => {
      const validData = {
        redirectUris: ['http://localhost:3000/callback', 'http://127.0.0.1:8080/auth'],
      }

      const result = step2Schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('should validate with optional post logout URIs', () => {
      const validData = {
        redirectUris: ['https://example.com/callback'],
        postLogoutUris: ['https://example.com/logout'],
      }

      const result = step2Schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('should validate with optional web origins', () => {
      const validData = {
        redirectUris: ['https://example.com/callback'],
        webOrigins: ['https://example.com', 'https://app.example.com'],
      }

      const result = step2Schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('should validate with all optional fields', () => {
      const validData = {
        redirectUris: ['https://example.com/callback'],
        postLogoutUris: ['https://example.com/logout'],
        webOrigins: ['https://example.com'],
      }

      const result = step2Schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('should apply default empty arrays for optional fields', () => {
      const result = step2Schema.safeParse({
        redirectUris: ['https://example.com/callback'],
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.postLogoutUris).toEqual([])
        expect(result.data.webOrigins).toEqual([])
      }
    })
  })

  describe('redirectUris field validation', () => {
    it('should reject empty redirect URIs array', () => {
      const invalidData = {
        redirectUris: [],
      }

      const result = step2Schema.safeParse(invalidData)
      expect(result.success).toBe(false)
    })

    it('should reject redirect URIs exceeding 100 items', () => {
      const invalidData = {
        redirectUris: Array.from({ length: 101 }, (_, i) => `https://example${i}.com/callback`),
      }

      const result = step2Schema.safeParse(invalidData)
      expect(result.success).toBe(false)
    })

    it('should reject invalid URL format', () => {
      const invalidData = {
        redirectUris: ['not-a-valid-url', 'https://example.com/callback'],
      }

      const result = step2Schema.safeParse(invalidData)
      expect(result.success).toBe(false)
    })

    it('should reject URLs without http or https protocol', () => {
      const invalidData = {
        redirectUris: ['ftp://example.com/callback'],
      }

      const result = step2Schema.safeParse(invalidData)
      expect(result.success).toBe(false)
    })

    it('should reject empty string URIs', () => {
      const invalidData = {
        redirectUris: ['https://example.com/callback', ''],
      }

      const result = step2Schema.safeParse(invalidData)
      expect(result.success).toBe(false)
    })
  })

  describe('postLogoutUris field validation', () => {
    it('should reject post logout URIs exceeding 50 items', () => {
      const invalidData = {
        redirectUris: ['https://example.com/callback'],
        postLogoutUris: Array.from({ length: 51 }, (_, i) => `https://example${i}.com/logout`),
      }

      const result = step2Schema.safeParse(invalidData)
      expect(result.success).toBe(false)
    })

    it('should reject invalid URL format in post logout URIs', () => {
      const invalidData = {
        redirectUris: ['https://example.com/callback'],
        postLogoutUris: ['not-a-valid-url'],
      }

      const result = step2Schema.safeParse(invalidData)
      expect(result.success).toBe(false)
    })
  })

  describe('webOrigins field validation', () => {
    it('should reject web origins exceeding 50 items', () => {
      const invalidData = {
        redirectUris: ['https://example.com/callback'],
        webOrigins: Array.from({ length: 51 }, (_, i) => `https://example${i}.com`),
      }

      const result = step2Schema.safeParse(invalidData)
      expect(result.success).toBe(false)
    })

    it('should reject invalid URL format in web origins', () => {
      const invalidData = {
        redirectUris: ['https://example.com/callback'],
        webOrigins: ['not-a-valid-url'],
      }

      const result = step2Schema.safeParse(invalidData)
      expect(result.success).toBe(false)
    })
  })

  describe('transform functions', () => {
    it('should transform URIs to UriItem array', () => {
      const uris = ['https://example.com/callback', 'https://app.example.com/auth']
      const result = transformToUriItems(uris)

      expect(result).toHaveLength(2)
      expect(result[0]).toMatchObject({
        value: 'https://example.com/callback',
        isValid: true,
      })
      expect(result[1]).toMatchObject({
        value: 'https://app.example.com/auth',
        isValid: true,
      })
      expect(result[0].id).toMatch(/^init-\d+-\d+$/)
      expect(result[1].id).toMatch(/^init-\d+-\d+$/)
    })

    it('should transform UriItem array back to URIs', () => {
      const items = [
        { id: '1', value: 'https://example.com/callback', isValid: true },
        { id: '2', value: 'https://app.example.com/auth', isValid: true },
        { id: '3', value: 'https://invalid.com', isValid: false },
      ]
      const result = transformFromUriItems(items)

      expect(result).toEqual(['https://example.com/callback', 'https://app.example.com/auth'])
    })

    it('should filter out invalid URIs when transforming from UriItems', () => {
      const items = [
        { id: '1', value: 'https://example.com/callback', isValid: true },
        { id: '2', value: 'https://invalid.com', isValid: false },
      ]
      const result = transformFromUriItems(items)

      expect(result).toEqual(['https://example.com/callback'])
    })

    it('should handle empty array when transforming from UriItems', () => {
      const result = transformFromUriItems([])
      expect(result).toEqual([])
    })
  })

  describe('TypeScript type inference', () => {
    it('should correctly infer TypeScript types', () => {
      const data: Step2FormData = {
        redirectUris: ['https://example.com/callback'],
        postLogoutUris: ['https://example.com/logout'],
        webOrigins: ['https://example.com'],
      }

      expect(Array.isArray(data.redirectUris)).toBe(true)
      expect(Array.isArray(data.postLogoutUris)).toBe(true)
      expect(Array.isArray(data.webOrigins)).toBe(true)
    })
  })

  describe('edge cases', () => {
    it('should accept maximum valid redirect URIs (100 items)', () => {
      const validData = {
        redirectUris: Array.from({ length: 100 }, (_, i) => `https://example${i}.com/callback`),
      }

      const result = step2Schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('should accept maximum valid post logout URIs (50 items)', () => {
      const validData = {
        redirectUris: ['https://example.com/callback'],
        postLogoutUris: Array.from({ length: 50 }, (_, i) => `https://example${i}.com/logout`),
      }

      const result = step2Schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('should accept maximum valid web origins (50 items)', () => {
      const validData = {
        redirectUris: ['https://example.com/callback'],
        webOrigins: Array.from({ length: 50 }, (_, i) => `https://example${i}.com`),
      }

      const result = step2Schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('should handle URIs with ports', () => {
      const validData = {
        redirectUris: ['https://example.com:8443/callback', 'http://localhost:3000/auth'],
      }

      const result = step2Schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('should handle URIs with query parameters and fragments', () => {
      const validData = {
        redirectUris: [
          'https://example.com/callback?param=value',
          'https://example.com/auth#fragment',
        ],
      }

      const result = step2Schema.safeParse(validData)
      expect(result.success).toBe(true)
    })
  })
})
