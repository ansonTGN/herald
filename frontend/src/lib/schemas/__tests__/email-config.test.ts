import { describe, it, expect } from 'vitest'
import { emailConfigSchema } from '../realm-config'

describe('emailConfigSchema', () => {
  describe('default values encode business decisions', () => {
    it('should default smtpPort to 587 when omitted', () => {
      const result = emailConfigSchema.safeParse({
        provider: 'resend',
        fromAddress: '',
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.smtpPort).toBe('587')
      }
    })

    it('should default smtpEncryption to starttls when omitted', () => {
      const result = emailConfigSchema.safeParse({
        provider: 'resend',
        fromAddress: '',
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.smtpEncryption).toBe('starttls')
      }
    })
  })

  describe('fromAddress allows empty string for unconfigured realm', () => {
    it('should accept empty string (unconfigured realm)', () => {
      const result = emailConfigSchema.safeParse({
        provider: 'resend',
        fromAddress: '',
      })

      expect(result.success).toBe(true)
    })

    it('should accept valid email address', () => {
      const result = emailConfigSchema.safeParse({
        provider: 'resend',
        fromAddress: 'noreply@example.com',
      })

      expect(result.success).toBe(true)
    })

    it('should reject invalid email that is not empty string', () => {
      const result = emailConfigSchema.safeParse({
        provider: 'resend',
        fromAddress: 'not-an-email',
      })

      expect(result.success).toBe(false)
    })
  })

  describe('provider enum enforcement', () => {
    it.each(['resend', 'smtp'] as const)('should accept provider=%s', (provider) => {
      const result = emailConfigSchema.safeParse({
        provider,
        fromAddress: '',
      })

      expect(result.success).toBe(true)
    })

    it('should reject invalid provider value', () => {
      const result = emailConfigSchema.safeParse({
        provider: 'sendgrid',
        fromAddress: '',
      })

      expect(result.success).toBe(false)
    })
  })

  describe('smtpEncryption enum enforcement', () => {
    it.each(['starttls', 'ssl'] as const)('should accept smtpEncryption=%s', (encryption) => {
      const result = emailConfigSchema.safeParse({
        provider: 'smtp',
        fromAddress: '',
        smtpEncryption: encryption,
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.smtpEncryption).toBe(encryption)
      }
    })

    it('should reject invalid encryption value', () => {
      const result = emailConfigSchema.safeParse({
        provider: 'smtp',
        fromAddress: '',
        smtpEncryption: 'tls',
      })

      expect(result.success).toBe(false)
    })
  })

  describe('optional fields accept undefined and omit cleanly', () => {
    it('should accept Resend config without optional fields', () => {
      const result = emailConfigSchema.safeParse({
        provider: 'resend',
        fromAddress: 'noreply@example.com',
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.resendApiKey).toBeUndefined()
        expect(result.data.smtpHost).toBeUndefined()
        expect(result.data.smtpUsername).toBeUndefined()
        expect(result.data.smtpPassword).toBeUndefined()
      }
    })
  })

  describe('full valid Resend config', () => {
    it('should accept complete Resend config with API key', () => {
      const result = emailConfigSchema.safeParse({
        provider: 'resend',
        fromAddress: 'noreply@example.com',
        resendApiKey: 're_xxxxx',
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.provider).toBe('resend')
        expect(result.data.fromAddress).toBe('noreply@example.com')
        expect(result.data.resendApiKey).toBe('re_xxxxx')
      }
    })
  })

  describe('full valid SMTP config', () => {
    it('should accept complete SMTP config with all fields', () => {
      const result = emailConfigSchema.safeParse({
        provider: 'smtp',
        fromAddress: 'noreply@example.com',
        smtpHost: 'smtp.qq.com',
        smtpPort: '587',
        smtpEncryption: 'starttls',
        smtpUsername: 'user@qq.com',
        smtpPassword: 'authcode123',
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.provider).toBe('smtp')
        expect(result.data.fromAddress).toBe('noreply@example.com')
        expect(result.data.smtpHost).toBe('smtp.qq.com')
        expect(result.data.smtpPort).toBe('587')
        expect(result.data.smtpEncryption).toBe('starttls')
        expect(result.data.smtpUsername).toBe('user@qq.com')
        expect(result.data.smtpPassword).toBe('authcode123')
      }
    })
  })

  describe('minimum valid config applies defaults', () => {
    it('should apply smtpPort and smtpEncryption defaults when only required fields given', () => {
      const result = emailConfigSchema.safeParse({
        provider: 'resend',
        fromAddress: 'noreply@example.com',
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.smtpPort).toBe('587')
        expect(result.data.smtpEncryption).toBe('starttls')
      }
    })
  })
})
