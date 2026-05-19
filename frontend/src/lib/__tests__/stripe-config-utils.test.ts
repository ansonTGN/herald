import { describe, test, expect } from 'vitest'
import { parseStripeConfig, buildStripeConfigRequest } from '../stripe-config-utils'
import type { RealmConfigResponse } from '@/lib/api-generated'
import type { StripeConfigForm } from '@/lib/schemas/stripe-config'
import { PAYMENT_PROVIDERS, STRIPE_CONFIG_KEYS } from '@/lib/billing-constants'

describe('stripe-config-utils', () => {
  describe('parseStripeConfig', () => {
    test('parses valid Stripe config from realm config array', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.ENABLED,
          configValue: 'true',
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.PUBLISHABLE_KEY,
          configValue: 'pk_test_123456789',
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.API_KEY,
          configValue: 'sk_test_987654321',
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.WEBHOOK_SECRET,
          configValue: 'whsec_abcdef',
          enabled: true,
        },
      ]

      const result = parseStripeConfig(configs)

      expect(result).toEqual({
        enabled: true,
        publishableKey: 'pk_test_123456789',
        secretKey: '',
        webhookSecret: '',
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

    test('handles missing optional fields gracefully', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.ENABLED,
          configValue: 'true',
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.PUBLISHABLE_KEY,
          configValue: 'pk_test_123',
          enabled: true,
        },
        // API_KEY and WEBHOOK_SECRET are missing
      ]

      const result = parseStripeConfig(configs)

      expect(result).toEqual({
        enabled: true,
        publishableKey: 'pk_test_123',
        secretKey: '',
        webhookSecret: '',
      })
    })

    test('handles malformed JSON gracefully', () => {
      // The new implementation reads configValue directly, not as JSON
      // so a malformed value just becomes the string value
      const configs: RealmConfigResponse[] = [
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.ENABLED,
          configValue: 'not-true',
          enabled: true,
        },
      ]

      const result = parseStripeConfig(configs)

      expect(result.enabled).toBe(false)
    })

    test('handles null values in JSON', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.ENABLED,
          configValue: '',
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.PUBLISHABLE_KEY,
          configValue: '',
          enabled: true,
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

    test('ignores non-stripe configs', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: 'creem',
          configKey: 'settings',
          configValue: JSON.stringify({ enabled: true }),
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.ENABLED,
          configValue: 'true',
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.PUBLISHABLE_KEY,
          configValue: 'pk_test_123',
          enabled: true,
        },
      ]

      const result = parseStripeConfig(configs)

      expect(result).toEqual({
        enabled: true,
        publishableKey: 'pk_test_123',
        secretKey: '',
        webhookSecret: '',
      })
    })

    test('handles special characters in config values', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.ENABLED,
          configValue: 'true',
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.PUBLISHABLE_KEY,
          configValue: 'pk_test_<>{}"\\n\\t',
          enabled: true,
        },
      ]

      const result = parseStripeConfig(configs)

      expect(result.publishableKey).toBe('pk_test_<>{}"\\n\\t')
    })

    test('handles config with empty array', () => {
      const configs: RealmConfigResponse[] = []

      const result = parseStripeConfig(configs)

      expect(result).toEqual({
        enabled: false,
        publishableKey: '',
        secretKey: '',
        webhookSecret: '',
      })
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

      expect(result).toEqual([
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.ENABLED,
          configValue: 'true',
          isSecret: false,
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.PUBLISHABLE_KEY,
          configValue: 'pk_test_123',
          isSecret: false,
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.API_KEY,
          configValue: 'sk_test_456',
          isSecret: true,
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.WEBHOOK_SECRET,
          configValue: 'whsec_789',
          isSecret: true,
          enabled: true,
        },
      ])
    })

    test('builds request with optional webhook secret omitted', () => {
      const formData: StripeConfigForm = {
        enabled: true,
        publishableKey: 'pk_test_123',
        secretKey: 'sk_test_456',
        webhookSecret: '',
      }

      const result = buildStripeConfigRequest(formData)

      // Empty secrets are filtered out
      expect(result.find((r) => r.configKey === STRIPE_CONFIG_KEYS.WEBHOOK_SECRET)).toBeUndefined()
      expect(result.find((r) => r.configKey === STRIPE_CONFIG_KEYS.API_KEY)).toBeDefined()
    })

    test('builds request with disabled config', () => {
      const formData: StripeConfigForm = {
        enabled: false,
        publishableKey: 'pk_test_123',
        secretKey: 'sk_test_456',
        webhookSecret: 'whsec_789',
      }

      const result = buildStripeConfigRequest(formData)

      expect(result.every((r) => r.enabled === false)).toBe(true)
      expect(result.every((r) => r.configType === PAYMENT_PROVIDERS.STRIPE)).toBe(true)
    })

    test('handles minimum valid values', () => {
      const formData: StripeConfigForm = {
        enabled: true,
        publishableKey: 'pk_test_a',
        secretKey: 'sk_test_a',
        webhookSecret: '',
      }

      const result = buildStripeConfigRequest(formData)

      expect(result.every((r) => r.configType === PAYMENT_PROVIDERS.STRIPE)).toBe(true)
      const keys = result.map((r) => r.configKey)
      expect(keys).toContain(STRIPE_CONFIG_KEYS.ENABLED)
      expect(keys).toContain(STRIPE_CONFIG_KEYS.PUBLISHABLE_KEY)
      expect(keys).toContain(STRIPE_CONFIG_KEYS.API_KEY)
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

      expect(
        result.find((r) => r.configKey === STRIPE_CONFIG_KEYS.PUBLISHABLE_KEY)?.configValue
      ).toBe(`pk_test_${longKey}`)
      expect(result.find((r) => r.configKey === STRIPE_CONFIG_KEYS.API_KEY)?.configValue).toBe(
        `sk_test_${longKey}`
      )
      expect(
        result.find((r) => r.configKey === STRIPE_CONFIG_KEYS.WEBHOOK_SECRET)?.configValue
      ).toBe(`whsec_${longKey}`)
    })

    test('marks individual fields with correct isSecret', () => {
      const formData: StripeConfigForm = {
        enabled: true,
        publishableKey: 'pk_test_123',
        secretKey: 'sk_test_456',
        webhookSecret: 'whsec_789',
      }

      const result = buildStripeConfigRequest(formData)

      expect(result.find((r) => r.configKey === STRIPE_CONFIG_KEYS.ENABLED)?.isSecret).toBe(false)
      expect(result.find((r) => r.configKey === STRIPE_CONFIG_KEYS.PUBLISHABLE_KEY)?.isSecret).toBe(
        false
      )
      expect(result.find((r) => r.configKey === STRIPE_CONFIG_KEYS.API_KEY)?.isSecret).toBe(true)
      expect(result.find((r) => r.configKey === STRIPE_CONFIG_KEYS.WEBHOOK_SECRET)?.isSecret).toBe(
        true
      )
    })
  })
})
