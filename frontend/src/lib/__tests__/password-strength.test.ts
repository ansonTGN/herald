import { describe, it, expect } from 'vitest'
import { calculatePasswordStrength, type PasswordConfig } from '../password-strength'

describe('calculatePasswordStrength', () => {
  const defaultConfig: PasswordConfig = {
    minLength: 8,
    requireUppercase: true,
    requireLowercase: true,
    requireNumber: true,
    requireSpecialChar: true,
  }

  describe('password length validation', () => {
    it('returns Weak for empty password', () => {
      const strength = calculatePasswordStrength('', defaultConfig)

      expect(strength.label).toBe('Weak')
      expect(strength.score).toBe(0)
      expect(strength.suggestions).toContain('Password must be at least 8 characters')
    })

    it('returns Weak for password shorter than min length', () => {
      const strength = calculatePasswordStrength('pass', defaultConfig)

      // Score is 1 because lowercase requirement is met even though length is not
      expect(strength.label).toBe('Weak')
      expect(strength.score).toBe(1)
      expect(strength.suggestions).toContain('Password must be at least 8 characters')
    })

    it('returns Fair for password at min length', () => {
      const strength = calculatePasswordStrength('password', defaultConfig)

      expect(strength.label).toBe('Fair')
      expect(strength.score).toBe(2)
      expect(strength.suggestions).not.toContain('Password must be at least 8 characters')
    })

    it('returns Strong for strong password', () => {
      const strength = calculatePasswordStrength('Password123!', defaultConfig)

      expect(strength.label).toBe('Strong')
      expect(strength.score).toBe(5)
      expect(strength.suggestions).toHaveLength(0)
    })
  })

  describe('uppercase letter validation', () => {
    it('suggests uppercase letters when required and missing', () => {
      const strength = calculatePasswordStrength('password123!', defaultConfig)

      expect(strength.label).toBe('Strong')
      expect(strength.suggestions).toContain('Password must contain uppercase letters')
    })

    it('does not require uppercase when config is false', () => {
      const relaxedConfig: PasswordConfig = {
        ...defaultConfig,
        requireUppercase: false,
      }

      const strength = calculatePasswordStrength('password123!', relaxedConfig)

      expect(strength.suggestions).not.toContain('Password must contain uppercase letters')
      expect(strength.label).toBe('Strong')
    })
  })

  describe('lowercase letter validation', () => {
    it('suggests lowercase letters when required and missing', () => {
      const strength = calculatePasswordStrength('PASSWORD123!', defaultConfig)

      expect(strength.label).toBe('Strong')
      expect(strength.suggestions).toContain('Password must contain lowercase letters')
    })

    it('does not require lowercase when config is false', () => {
      const relaxedConfig: PasswordConfig = {
        ...defaultConfig,
        requireLowercase: false,
      }

      const strength = calculatePasswordStrength('PASSWORD123!', relaxedConfig)

      expect(strength.suggestions).not.toContain('Password must contain lowercase letters')
      expect(strength.label).toBe('Strong')
    })
  })

  describe('number validation', () => {
    it('suggests numbers when required and missing', () => {
      const strength = calculatePasswordStrength('Password!', defaultConfig)

      expect(strength.label).toBe('Strong')
      expect(strength.suggestions).toContain('Password must contain numbers')
    })

    it('does not require numbers when config is false', () => {
      const relaxedConfig: PasswordConfig = {
        ...defaultConfig,
        requireNumber: false,
      }

      const strength = calculatePasswordStrength('Password!', relaxedConfig)

      expect(strength.suggestions).not.toContain('Password must contain numbers')
      expect(strength.label).toBe('Strong')
    })
  })

  describe('special character validation', () => {
    it('suggests special characters when required and missing', () => {
      const strength = calculatePasswordStrength('Password123', defaultConfig)

      expect(strength.label).toBe('Strong')
      expect(strength.suggestions).toContain('Password must contain special characters')
    })

    it('does not require special characters when config is false', () => {
      const relaxedConfig: PasswordConfig = {
        ...defaultConfig,
        requireSpecialChar: false,
      }

      const strength = calculatePasswordStrength('Password123', relaxedConfig)

      expect(strength.suggestions).not.toContain('Password must contain special characters')
      expect(strength.label).toBe('Strong')
    })
  })

  describe('complex passwords', () => {
    it('handles passwords with all character types', () => {
      const strength = calculatePasswordStrength('ComplexP@ssw0rd123!', defaultConfig)

      expect(strength.label).toBe('Strong')
      expect(strength.score).toBe(5)
      expect(strength.suggestions).toHaveLength(0)
    })

    it('handles passwords with mixed cases and numbers', () => {
      const strength = calculatePasswordStrength('PassWord123', defaultConfig)

      expect(strength.label).toBe('Strong')
      expect(strength.suggestions).toContain('Password must contain special characters')
    })
  })
})
