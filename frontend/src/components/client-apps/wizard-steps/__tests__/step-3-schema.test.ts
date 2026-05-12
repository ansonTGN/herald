import { describe, it, expect } from 'vitest'
import {
  step3Schema,
  type Step3FormData,
  SESSION_TTL_PRESETS,
  ADVANCED_SECURITY_OPTIONS,
} from '../step-3-schema'

describe('step3Schema', () => {
  describe('valid session TTL', () => {
    it('accepts minimum valid session TTL (60 seconds)', () => {
      const result = step3Schema.safeParse({
        sessionTtlSeconds: 60,
      })
      expect(result.success).toBe(true)
    })

    it('accepts maximum valid session TTL (86400 seconds)', () => {
      const result = step3Schema.safeParse({
        sessionTtlSeconds: 86400,
      })
      expect(result.success).toBe(true)
    })

    it('accepts default session TTL (3600 seconds)', () => {
      const result = step3Schema.safeParse({
        sessionTtlSeconds: 3600,
      })
      expect(result.success).toBe(true)
    })

    it('applies default value when session TTL is not provided', () => {
      const result = step3Schema.safeParse({})
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.sessionTtlSeconds).toBe(3600)
      }
    })
  })

  describe('invalid session TTL', () => {
    it('rejects session TTL below minimum (59 seconds)', () => {
      const result = step3Schema.safeParse({
        sessionTtlSeconds: 59,
      })
      expect(result.success).toBe(false)
    })

    it('rejects session TTL above maximum (86401 seconds)', () => {
      const result = step3Schema.safeParse({
        sessionTtlSeconds: 86401,
      })
      expect(result.success).toBe(false)
    })

    it('provides helpful error message for invalid session TTL', () => {
      const result = step3Schema.safeParse({
        sessionTtlSeconds: 59,
      })
      expect(result.success).toBe(false)
    })
  })

  describe('session renewal TTL', () => {
    it('accepts valid session renewal TTL', () => {
      const result = step3Schema.safeParse({
        sessionTtlSeconds: 3600,
        sessionRenewalTtlSeconds: 7200,
      })
      expect(result.success).toBe(true)
    })

    it('accepts maximum valid session renewal TTL (604800 seconds)', () => {
      const result = step3Schema.safeParse({
        sessionTtlSeconds: 3600,
        sessionRenewalTtlSeconds: 604800,
      })
      expect(result.success).toBe(true)
    })

    it('allows session renewal TTL to be optional', () => {
      const result = step3Schema.safeParse({
        sessionTtlSeconds: 3600,
      })
      expect(result.success).toBe(true)
    })

    it('rejects session renewal TTL above maximum', () => {
      const result = step3Schema.safeParse({
        sessionTtlSeconds: 3600,
        sessionRenewalTtlSeconds: 604801,
      })
      expect(result.success).toBe(false)
    })

    it('rejects session renewal TTL when less than or equal to session TTL', () => {
      const result = step3Schema.safeParse({
        sessionTtlSeconds: 3600,
        sessionRenewalTtlSeconds: 3600,
      })
      expect(result.success).toBe(false)

      const result2 = step3Schema.safeParse({
        sessionTtlSeconds: 3600,
        sessionRenewalTtlSeconds: 1800,
      })
      expect(result2.success).toBe(false)
    })

    it('provides helpful error message when renewal TTL is invalid', () => {
      const result = step3Schema.safeParse({
        sessionTtlSeconds: 3600,
        sessionRenewalTtlSeconds: 1800,
      })
      expect(result.success).toBe(false)
    })
  })

  describe('SESSION_TTL_PRESETS', () => {
    it('contains expected preset options', () => {
      expect(SESSION_TTL_PRESETS).toHaveLength(7)
      expect(SESSION_TTL_PRESETS[0]).toEqual({
        value: 1800,
        label: '30 minutes',
        seconds: 1800,
      })
      expect(SESSION_TTL_PRESETS[6]).toEqual({
        value: 86400,
        label: '24 hours',
        seconds: 86400,
      })
    })

    it('all presets are within valid range', () => {
      SESSION_TTL_PRESETS.forEach((preset) => {
        expect(preset.seconds).toBeGreaterThanOrEqual(60)
        expect(preset.seconds).toBeLessThanOrEqual(86400)
      })
    })
  })

  describe('ADVANCED_SECURITY_OPTIONS', () => {
    it('contains 8 advanced security options', () => {
      expect(ADVANCED_SECURITY_OPTIONS).toHaveLength(8)
    })

    it('has required properties for each option', () => {
      ADVANCED_SECURITY_OPTIONS.forEach((option) => {
        expect(option).toHaveProperty('id')
        expect(option).toHaveProperty('label')
        expect(option).toHaveProperty('description')
        expect(option).toHaveProperty('type')
        expect(option).toHaveProperty('default')
        expect(option.type).toBe('boolean')
        expect(typeof option.default).toBe('boolean')
      })
    })

    it('includes expected security options', () => {
      const optionIds = ADVANCED_SECURITY_OPTIONS.map((opt) => opt.id)
      expect(optionIds).toContain('requireProofKeyForCodeExchange')
      expect(optionIds).toContain('implicitFlowEnabled')
      expect(optionIds).toContain('serviceAccountsEnabled')
      expect(optionIds).toContain('directAccessGrantsEnabled')
      expect(optionIds).toContain('standardFlowEnabled')
      expect(optionIds).toContain('frontchannelLogoutEnabled')
      expect(optionIds).toContain('backchannelLogoutEnabled')
      expect(optionIds).toContain('consentRequired')
    })
  })
})
