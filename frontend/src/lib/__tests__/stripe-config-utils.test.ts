import { describe, test, expect } from 'vitest'
import { parseStripeConfig, buildStripeConfigRequest } from '../stripe-config-utils'
import type { RealmConfigResponse } from '@/lib/api-generated'
import type { StripeConfigForm } from '@/lib/schemas/stripe-config'

describe('stripe-config-utils', () => {
  describe('parseStripeConfig', () => {
    test('parses valid Stripe config from realm config array', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: 'stripe',
          configKey: 'settings',
          configValue: JSON.stringify({
            enabled: true,
            publishableKey: 'pk_test_123456789',
            secretKey: 'sk_test_987654321',
            webhookSecret: 'whsec_abcdef',
          }),
          enabled: true,
        },
      ]

      const result = parseStripeConfig(configs)

      expect(result).toEqual({
        enabled: true,
        publishableKey: 'pk_test_123456789',
        secretKey: 'sk_test_987654321',
        webhookSecret: 'whsec_abcdef',
      })
    })

    test('returns default config when Stripe config is missing', () => {
      const configs: RealmConfigResponse[] = []

      const result = parseStripeConfig(configs)

      expect(result).toEqual({
        enabled: false,
        publishableKey: '',
        secretKey: '',
        webhookSecret: '',
      })
    })

    test('returns default config when config array is empty', () => {
      const configs: RealmConfigResponse[] = []

      const result = parseStripeConfig(configs)

      expect(result).toEqual({
        enabled: false,
        publishableKey: '',
        secretKey: '',
        webhookSecret: '',
      })
    })

    test('handles missing optional fields gracefully', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: 'stripe',
          configKey: 'settings',
          configValue: JSON.stringify({
            enabled: true,
            publishableKey: 'pk_test_123',
            secretKey: 'sk_test_456',
            // webhookSecret is missing
          }),
          enabled: true,
        },
      ]

      const result = parseStripeConfig(configs)

      expect(result).toEqual({
        enabled: true,
        publishableKey: 'pk_test_123',
        secretKey: 'sk_test_456',
        webhookSecret: '',
      })
    })

    test('handles malformed JSON gracefully', () => {
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

      const configs: RealmConfigResponse[] = [
        {
          configType: 'stripe',
          configKey: 'settings',
          configValue: 'not-valid-json{',
          enabled: true,
        },
      ]

      const result = parseStripeConfig(configs)

      // Should return default config instead of throwing
      expect(result).toEqual({
        enabled: false,
        publishableKey: '',
        secretKey: '',
        webhookSecret: '',
      })

      expect(consoleSpy).toHaveBeenCalledWith('Failed to parse Stripe config:', expect.any(Error))

      consoleSpy.mockRestore()
    })

    test('handles null values in JSON', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: 'stripe',
          configKey: 'settings',
          configValue: JSON.stringify({
            enabled: null,
            publishableKey: null,
            secretKey: null,
            webhookSecret: null,
          }),
          enabled: true,
        },
      ]

      const result = parseStripeConfig(configs)

      expect(result).toEqual({
        enabled: false, // null should default to false
        publishableKey: '', // null should default to empty string
        secretKey: '', // null should default to empty string
        webhookSecret: '',
      })
    })

    test('ignores non-stripe configs', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: 'creem',
          configKey: 'settings',
          configValue: JSON.stringify({ enabled: true }),
          enabled: true,
        },
        {
          configType: 'stripe',
          configKey: 'settings',
          configValue: JSON.stringify({
            enabled: true,
            publishableKey: 'pk_test_123',
            secretKey: 'sk_test_456',
          }),
          enabled: true,
        },
      ]

      const result = parseStripeConfig(configs)

      expect(result).toEqual({
        enabled: true,
        publishableKey: 'pk_test_123',
        secretKey: 'sk_test_456',
        webhookSecret: '',
      })
    })

    test('handles special characters in config values', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: 'stripe',
          configKey: 'settings',
          configValue: JSON.stringify({
            enabled: true,
            publishableKey: 'pk_test_<>{}"\\n\\t',
            secretKey: 'sk_test_special',
            webhookSecret: '',
          }),
          enabled: true,
        },
      ]

      const result = parseStripeConfig(configs)

      expect(result.publishableKey).toBe('pk_test_<>{}"\\n\\t')
    })

    test('handles empty string values', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: 'stripe',
          configKey: 'settings',
          configValue: JSON.stringify({
            enabled: false,
            publishableKey: '',
            secretKey: '',
            webhookSecret: '',
          }),
          enabled: false,
        },
      ]

      const result = parseStripeConfig(configs)

      expect(result).toEqual({
        enabled: false,
        publishableKey: '',
        secretKey: '',
        webhookSecret: '',
      })
    })

    test('handles config with null in array', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: 'stripe',
          configKey: 'settings',
          configValue: JSON.stringify({
            enabled: true,
            publishableKey: 'pk_test_123',
            secretKey: 'sk_test_456',
          }),
          enabled: true,
        },
        null as any,
      ]

      // Should not throw, should find the valid config
      expect(() => parseStripeConfig(configs)).not.toThrow()
    })
  })

  describe('buildStripeConfigRequest', () => {
    test('builds correct upsert request for full Stripe config', () => {
      const formData: StripeConfigForm = {
        enabled: true,
        publishableKey: 'pk_test_123',
        secretKey: 'sk_test_456',
        webhookSecret: 'whsec_789',
      }

      const result = buildStripeConfigRequest(formData)

      expect(result).toEqual({
        configType: 'stripe',
        configKey: 'settings',
        configValue: JSON.stringify({
          enabled: true,
          publishableKey: 'pk_test_123',
          secretKey: 'sk_test_456',
          webhookSecret: 'whsec_789',
        }),
        isSecret: false,
        enabled: true,
      })
    })

    test('builds request with optional webhook secret omitted', () => {
      const formData: StripeConfigForm = {
        enabled: true,
        publishableKey: 'pk_test_123',
        secretKey: 'sk_test_456',
        webhookSecret: '',
      }

      const result = buildStripeConfigRequest(formData)

      const parsedValue = JSON.parse(result.configValue)
      expect(parsedValue.webhookSecret).toBeUndefined()
      expect(result).toEqual({
        configType: 'stripe',
        configKey: 'settings',
        configValue: JSON.stringify({
          enabled: true,
          publishableKey: 'pk_test_123',
          secretKey: 'sk_test_456',
          webhookSecret: undefined,
        }),
        isSecret: false,
        enabled: true,
      })
    })

    test('builds request with disabled config', () => {
      const formData: StripeConfigForm = {
        enabled: false,
        publishableKey: 'pk_test_123',
        secretKey: 'sk_test_456',
        webhookSecret: 'whsec_789',
      }

      const result = buildStripeConfigRequest(formData)

      expect(result.enabled).toBe(false)
      expect(result.configType).toBe('stripe')
    })

    test('handles minimum valid values', () => {
      const formData: StripeConfigForm = {
        enabled: true,
        publishableKey: 'pk_test_a',
        secretKey: 'sk_test_a',
        webhookSecret: '',
      }

      const result = buildStripeConfigRequest(formData)

      expect(result.configType).toBe('stripe')
      expect(result.configKey).toBe('settings')
      expect(JSON.parse(result.configValue)).toEqual({
        enabled: true,
        publishableKey: 'pk_test_a',
        secretKey: 'sk_test_a',
        webhookSecret: undefined,
      })
    })

    test('handles long key values', () => {
      const longKey = 'a'.repeat(200)
      const formData: StripeConfigForm = {
        enabled: true,
        publishableKey: `pk_test_${longKey}`,
        secretKey: `sk_test_${longKey}`,
        webhookSecret: `whsec_${longKey}`,
      }

      const result = buildStripeConfigRequest(formData)

      const parsedValue = JSON.parse(result.configValue)
      expect(parsedValue.publishableKey).toBe(`pk_test_${longKey}`)
      expect(parsedValue.secretKey).toBe(`sk_test_${longKey}`)
      expect(parsedValue.webhookSecret).toBe(`whsec_${longKey}`)
    })

    test('produces valid JSON string', () => {
      const formData: StripeConfigForm = {
        enabled: true,
        publishableKey: 'pk_test_123',
        secretKey: 'sk_test_456',
        webhookSecret: 'whsec_789',
      }

      const result = buildStripeConfigRequest(formData)

      // Should be able to parse the JSON back
      expect(() => JSON.parse(result.configValue)).not.toThrow()

      const parsed = JSON.parse(result.configValue)
      expect(parsed).toEqual({
        enabled: true,
        publishableKey: 'pk_test_123',
        secretKey: 'sk_test_456',
        webhookSecret: 'whsec_789',
      })
    })

    test('marks isSecret as false (individual fields are marked in JSON)', () => {
      const formData: StripeConfigForm = {
        enabled: true,
        publishableKey: 'pk_test_123',
        secretKey: 'sk_test_456',
        webhookSecret: 'whsec_789',
      }

      const result = buildStripeConfigRequest(formData)

      // At the request level, isSecret is false
      // Individual sensitive fields are marked in the JSON structure
      expect(result.isSecret).toBe(false)
    })
  })
})
