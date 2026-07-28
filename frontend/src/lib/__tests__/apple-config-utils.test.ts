import { describe, test, expect } from 'vitest'
import { parseAppleConfig, buildAppleConfigRequest } from '../apple-config-utils'
import type { RealmConfigResponse } from '@/lib/api-generated'
import type { AppleIapConfigForm } from '@/lib/schemas/apple-config'
import { PAYMENT_PROVIDERS, APPLE_CONFIG_KEYS } from '@/lib/billing-constants'

describe('apple-config-utils', () => {
  describe('parseAppleConfig', () => {
    test('maps realm config rows to AppleIapConfigForm (environment defaults to production)', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: PAYMENT_PROVIDERS.APPLE,
          configKey: APPLE_CONFIG_KEYS.BUNDLE_ID,
          configValue: 'com.example.app',
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.APPLE,
          configKey: APPLE_CONFIG_KEYS.ISSUER_ID,
          configValue: 'issuer-xyz',
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.APPLE,
          configKey: APPLE_CONFIG_KEYS.KEY_ID,
          configValue: 'key-abc',
          enabled: true,
        },
        // PRIVATE_KEY_P8 intentionally omitted: secrets are not echoed by the
        // backend; the form should keep the field empty so the admin can
        // leave it blank to retain the existing value on save.
      ]

      const result = parseAppleConfig(configs)

      expect(result).toEqual({
        bundleId: 'com.example.app',
        issuerId: 'issuer-xyz',
        keyId: 'key-abc',
        privateKeyP8: '',
        // No `environment` row → default 'production'.
        environment: 'production',
      })
    })

    test('returns defaults (all empty) when no apple config rows exist', () => {
      const configs: RealmConfigResponse[] = []

      const result = parseAppleConfig(configs)

      expect(result).toEqual({
        bundleId: '',
        issuerId: '',
        keyId: '',
        privateKeyP8: '',
        environment: 'production',
      })
    })

    test('reads an explicit sandbox environment when present', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: PAYMENT_PROVIDERS.APPLE,
          configKey: APPLE_CONFIG_KEYS.ENVIRONMENT,
          configValue: 'sandbox',
          enabled: true,
        },
      ]

      const result = parseAppleConfig(configs)

      expect(result.environment).toBe('sandbox')
    })

    test('ignores non-apple config rows', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: 'publishable_key',
          configValue: 'pk_test_123',
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.APPLE,
          configKey: APPLE_CONFIG_KEYS.BUNDLE_ID,
          configValue: 'com.example.app',
          enabled: true,
        },
      ]

      const result = parseAppleConfig(configs)

      expect(result.bundleId).toBe('com.example.app')
    })
  })

  describe('buildAppleConfigRequest', () => {
    test('emits all non-secret fields with isSecret=false', () => {
      const config: AppleIapConfigForm = {
        bundleId: 'com.example.app',
        issuerId: 'issuer-xyz',
        keyId: 'key-abc',
        privateKeyP8: '-----BEGIN PRIVATE KEY-----\nMIGT\n-----END PRIVATE KEY-----',
        environment: 'production',
      }

      const result = buildAppleConfigRequest(config)

      const nonSecretKeys = [
        APPLE_CONFIG_KEYS.BUNDLE_ID,
        APPLE_CONFIG_KEYS.ISSUER_ID,
        APPLE_CONFIG_KEYS.KEY_ID,
        APPLE_CONFIG_KEYS.ENVIRONMENT,
      ]
      for (const key of nonSecretKeys) {
        const row = result.find((r) => r.configKey === key)
        expect(row).toBeDefined()
        expect(row?.isSecret).toBe(false)
      }
    })

    test('includes private_key_p8 when provided and marks isSecret=true', () => {
      const config: AppleIapConfigForm = {
        bundleId: 'com.example.app',
        issuerId: 'issuer-xyz',
        keyId: 'key-abc',
        privateKeyP8: '-----BEGIN PRIVATE KEY-----\nMIGT\n-----END PRIVATE KEY-----',
        environment: 'production',
      }

      const result = buildAppleConfigRequest(config)

      const secretRow = result.find((r) => r.configKey === APPLE_CONFIG_KEYS.PRIVATE_KEY_P8)
      expect(secretRow).toBeDefined()
      expect(secretRow?.isSecret).toBe(true)
      expect(secretRow?.configValue).toBe(
        '-----BEGIN PRIVATE KEY-----\nMIGT\n-----END PRIVATE KEY-----'
      )
    })

    test('omits empty private_key_p8 (leave-empty-to-keep)', () => {
      // Core contract (support-iap): an empty secret MUST be dropped from the
      // upsert payload so the backend preserves the previously-stored value.
      const config: AppleIapConfigForm = {
        bundleId: 'com.example.app',
        issuerId: 'issuer-xyz',
        keyId: 'key-abc',
        privateKeyP8: '',
        environment: 'production',
      }

      const result = buildAppleConfigRequest(config)

      expect(result.find((r) => r.configKey === APPLE_CONFIG_KEYS.PRIVATE_KEY_P8)).toBeUndefined()
    })

    test('always emits configType=apple and enabled=true on every row', () => {
      const config: AppleIapConfigForm = {
        bundleId: 'com.example.app',
        issuerId: 'issuer-xyz',
        keyId: 'key-abc',
        privateKeyP8: '',
        environment: 'production',
      }

      const result = buildAppleConfigRequest(config)

      expect(result.length).toBeGreaterThan(0)
      expect(result.every((r) => r.configType === PAYMENT_PROVIDERS.APPLE)).toBe(true)
      expect(result.every((r) => r.enabled === true)).toBe(true)
    })
  })
})
