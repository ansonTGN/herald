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
        publishableKey: 'pk_test_123456789',
        secretKey: '',
        webhookSecret: '',
        asyncPointsStrategy: 'conservative',
      })
    })

    test('returns default config when Stripe config is missing', () => {
      const configs: RealmConfigResponse[] = []

      const result = parseStripeConfig(configs)

      expect(result).toEqual({
        publishableKey: '',
        secretKey: '',
        webhookSecret: '',
        asyncPointsStrategy: 'conservative',
      })
    })

    test('handles missing optional fields gracefully', () => {
      const configs: RealmConfigResponse[] = [
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
        publishableKey: 'pk_test_123',
        secretKey: '',
        webhookSecret: '',
        asyncPointsStrategy: 'conservative',
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
          configKey: STRIPE_CONFIG_KEYS.PUBLISHABLE_KEY,
          configValue: 'pk_test_123',
          enabled: true,
        },
      ]

      const result = parseStripeConfig(configs)

      expect(result).toEqual({
        publishableKey: 'pk_test_123',
        secretKey: '',
        webhookSecret: '',
        asyncPointsStrategy: 'conservative',
      })
    })

    test('handles special characters in config values', () => {
      const configs: RealmConfigResponse[] = [
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

    test('parses eager async points strategy', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.ASYNC_POINTS_STRATEGY,
          configValue: 'eager',
          enabled: true,
        },
      ]

      const result = parseStripeConfig(configs)

      expect(result.asyncPointsStrategy).toBe('eager')
    })

    test('handles config with empty array', () => {
      const configs: RealmConfigResponse[] = []

      const result = parseStripeConfig(configs)

      expect(result).toEqual({
        publishableKey: '',
        secretKey: '',
        webhookSecret: '',
        asyncPointsStrategy: 'conservative',
      })
    })
  })

  describe('buildStripeConfigRequest', () => {
    test('builds correct upsert request for full Stripe config', () => {
      const formData: StripeConfigForm = {
        publishableKey: 'pk_test_123',
        secretKey: 'sk_test_456',
        webhookSecret: 'whsec_789',
        asyncPointsStrategy: 'conservative',
      }

      const result = buildStripeConfigRequest(formData)

      expect(result).toEqual([
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
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: STRIPE_CONFIG_KEYS.ASYNC_POINTS_STRATEGY,
          configValue: 'conservative',
          isSecret: false,
          enabled: true,
        },
      ])
    })

    test('builds request with optional webhook secret omitted', () => {
      const formData: StripeConfigForm = {
        publishableKey: 'pk_test_123',
        secretKey: 'sk_test_456',
        webhookSecret: '',
        asyncPointsStrategy: 'conservative',
      }

      const result = buildStripeConfigRequest(formData)

      // Empty secrets are filtered out
      expect(result.find((r) => r.configKey === STRIPE_CONFIG_KEYS.WEBHOOK_SECRET)).toBeUndefined()
      expect(result.find((r) => r.configKey === STRIPE_CONFIG_KEYS.API_KEY)).toBeDefined()
    })

    test('always marks every emitted row as enabled', () => {
      const formData: StripeConfigForm = {
        publishableKey: 'pk_test_123',
        secretKey: 'sk_test_456',
        webhookSecret: 'whsec_789',
        asyncPointsStrategy: 'conservative',
      }

      const result = buildStripeConfigRequest(formData)

      expect(result.every((r) => r.enabled === true)).toBe(true)
      expect(result.every((r) => r.configType === PAYMENT_PROVIDERS.STRIPE)).toBe(true)
    })

    test('handles minimum valid values', () => {
      const formData: StripeConfigForm = {
        publishableKey: 'pk_test_a',
        secretKey: 'sk_test_a',
        webhookSecret: '',
        asyncPointsStrategy: 'conservative',
      }

      const result = buildStripeConfigRequest(formData)

      expect(result.every((r) => r.configType === PAYMENT_PROVIDERS.STRIPE)).toBe(true)
      const keys = result.map((r) => r.configKey)
      expect(keys).toContain(STRIPE_CONFIG_KEYS.PUBLISHABLE_KEY)
      expect(keys).toContain(STRIPE_CONFIG_KEYS.API_KEY)
    })

    test('handles long key values', () => {
      const longKey = 'a'.repeat(200)
      const formData: StripeConfigForm = {
        publishableKey: `pk_test_${longKey}`,
        secretKey: `sk_test_${longKey}`,
        webhookSecret: `whsec_${longKey}`,
        asyncPointsStrategy: 'conservative',
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
        publishableKey: 'pk_test_123',
        secretKey: 'sk_test_456',
        webhookSecret: 'whsec_789',
        asyncPointsStrategy: 'conservative',
      }

      const result = buildStripeConfigRequest(formData)

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
