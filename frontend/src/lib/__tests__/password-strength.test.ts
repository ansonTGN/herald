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
    it('returns weak for empty password', () => {
      const strength = calculatePasswordStrength('', defaultConfig)

      expect(strength.level).toBe('weak')
      expect(strength.score).toBe(0)
      expect(strength.unmet).toContainEqual({ key: 'min_length', length: 8 })
    })

    it('returns weak for password shorter than min length', () => {
      const strength = calculatePasswordStrength('pass', defaultConfig)

      // Score is 1 because lowercase requirement is met even though length is not
      expect(strength.level).toBe('weak')
      expect(strength.score).toBe(1)
      expect(strength.unmet).toContainEqual({ key: 'min_length', length: 8 })
    })

    it('returns fair for password at min length', () => {
      const strength = calculatePasswordStrength('password', defaultConfig)

      expect(strength.level).toBe('fair')
      expect(strength.score).toBe(2)
      expect(strength.unmet.map((u) => u.key)).not.toContain('min_length')
    })

    it('returns strong for strong password', () => {
      const strength = calculatePasswordStrength('Password123!', defaultConfig)

      expect(strength.level).toBe('strong')
      expect(strength.score).toBe(5)
      expect(strength.unmet).toHaveLength(0)
    })
  })

  describe('uppercase letter validation', () => {
    it('reports require_uppercase when required and missing', () => {
      const strength = calculatePasswordStrength('password123!', defaultConfig)

      expect(strength.level).toBe('strong')
      expect(strength.unmet.map((u) => u.key)).toContain('require_uppercase')
    })

    it('does not require uppercase when config is false', () => {
      const relaxedConfig: PasswordConfig = {
        ...defaultConfig,
        requireUppercase: false,
      }

      const strength = calculatePasswordStrength('password123!', relaxedConfig)

      expect(strength.unmet.map((u) => u.key)).not.toContain('require_uppercase')
      expect(strength.level).toBe('strong')
    })
  })

  describe('lowercase letter validation', () => {
    it('reports require_lowercase when required and missing', () => {
      const strength = calculatePasswordStrength('PASSWORD123!', defaultConfig)

      expect(strength.level).toBe('strong')
      expect(strength.unmet.map((u) => u.key)).toContain('require_lowercase')
    })

    it('does not require lowercase when config is false', () => {
      const relaxedConfig: PasswordConfig = {
        ...defaultConfig,
        requireLowercase: false,
      }

      const strength = calculatePasswordStrength('PASSWORD123!', relaxedConfig)

      expect(strength.unmet.map((u) => u.key)).not.toContain('require_lowercase')
      expect(strength.level).toBe('strong')
    })
  })

  describe('number validation', () => {
    it('reports require_number when required and missing', () => {
      const strength = calculatePasswordStrength('Password!', defaultConfig)

      expect(strength.level).toBe('strong')
      expect(strength.unmet.map((u) => u.key)).toContain('require_number')
    })

    it('does not require numbers when config is false', () => {
      const relaxedConfig: PasswordConfig = {
        ...defaultConfig,
        requireNumber: false,
      }

      const strength = calculatePasswordStrength('Password!', relaxedConfig)

      expect(strength.unmet.map((u) => u.key)).not.toContain('require_number')
      expect(strength.level).toBe('strong')
    })
  })

  describe('special character validation', () => {
    it('reports require_special_char when required and missing', () => {
      const strength = calculatePasswordStrength('Password123', defaultConfig)

      expect(strength.level).toBe('strong')
      expect(strength.unmet.map((u) => u.key)).toContain('require_special_char')
    })

    it('does not require special characters when config is false', () => {
      const relaxedConfig: PasswordConfig = {
        ...defaultConfig,
        requireSpecialChar: false,
      }

      const strength = calculatePasswordStrength('Password123', relaxedConfig)

      expect(strength.unmet.map((u) => u.key)).not.toContain('require_special_char')
      expect(strength.level).toBe('strong')
    })
  })

  describe('complex passwords', () => {
    it('handles passwords with all character types', () => {
      const strength = calculatePasswordStrength('ComplexP@ssw0rd123!', defaultConfig)

      expect(strength.level).toBe('strong')
      expect(strength.score).toBe(5)
      expect(strength.unmet).toHaveLength(0)
    })

    it('handles passwords with mixed cases and numbers', () => {
      const strength = calculatePasswordStrength('PassWord123', defaultConfig)

      expect(strength.level).toBe('strong')
      expect(strength.unmet.map((u) => u.key)).toContain('require_special_char')
    })
  })
})
