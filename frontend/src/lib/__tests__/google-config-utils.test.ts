import { describe, test, expect } from 'vitest'
import { parseGoogleConfig, buildGoogleConfigRequest } from '../google-config-utils'
import type { RealmConfigResponse } from '@/lib/api-generated'
import type { GooglePlayConfigForm } from '@/lib/schemas/google-config'
import { PAYMENT_PROVIDERS, GOOGLE_CONFIG_KEYS } from '@/lib/billing-constants'

describe('google-config-utils', () => {
  describe('parseGoogleConfig', () => {
    test('maps realm config rows to GooglePlayConfigForm', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: PAYMENT_PROVIDERS.GOOGLE,
          configKey: GOOGLE_CONFIG_KEYS.PACKAGE_NAME,
          configValue: 'com.example.app',
          enabled: true,
        },
        // SERVICE_ACCOUNT_JSON intentionally omitted: secrets are not echoed by
        // the backend; the form should keep the field empty so the admin can
        // leave it blank to retain the existing value on save.
      ]

      const result = parseGoogleConfig(configs)

      expect(result).toEqual({
        packageName: 'com.example.app',
        serviceAccountJson: '',
      })
    })

    test('returns defaults (all empty) when no google config rows exist', () => {
      const configs: RealmConfigResponse[] = []

      const result = parseGoogleConfig(configs)

      expect(result).toEqual({
        packageName: '',
        serviceAccountJson: '',
      })
    })

    test('ignores non-google config rows', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: PAYMENT_PROVIDERS.APPLE,
          configKey: 'bundle_id',
          configValue: 'com.example.app',
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.GOOGLE,
          configKey: GOOGLE_CONFIG_KEYS.PACKAGE_NAME,
          configValue: 'com.example.android',
          enabled: true,
        },
      ]

      const result = parseGoogleConfig(configs)

      expect(result.packageName).toBe('com.example.android')
    })
  })

  describe('buildGoogleConfigRequest', () => {
    test('emits package_name as non-secret', () => {
      const config: GooglePlayConfigForm = {
        packageName: 'com.example.app',
        serviceAccountJson: '{"type":"service_account"}',
      }

      const result = buildGoogleConfigRequest(config)

      const packageNameRow = result.find((r) => r.configKey === GOOGLE_CONFIG_KEYS.PACKAGE_NAME)
      expect(packageNameRow).toBeDefined()
      expect(packageNameRow?.isSecret).toBe(false)
      expect(packageNameRow?.configValue).toBe('com.example.app')
    })

    test('includes service_account_json when provided and marks isSecret=true', () => {
      const config: GooglePlayConfigForm = {
        packageName: 'com.example.app',
        serviceAccountJson: '{"type":"service_account","project_id":"ex"}',
      }

      const result = buildGoogleConfigRequest(config)

      const secretRow = result.find((r) => r.configKey === GOOGLE_CONFIG_KEYS.SERVICE_ACCOUNT_JSON)
      expect(secretRow).toBeDefined()
      expect(secretRow?.isSecret).toBe(true)
      expect(secretRow?.configValue).toBe('{"type":"service_account","project_id":"ex"}')
    })

    test('omits empty service_account_json (leave-empty-to-keep)', () => {
      // Core contract (support-iap): an empty secret MUST be dropped from the
      // upsert payload so the backend preserves the previously-stored value.
      const config: GooglePlayConfigForm = {
        packageName: 'com.example.app',
        serviceAccountJson: '',
      }

      const result = buildGoogleConfigRequest(config)

      expect(
        result.find((r) => r.configKey === GOOGLE_CONFIG_KEYS.SERVICE_ACCOUNT_JSON)
      ).toBeUndefined()
    })

    test('always emits configType=google and enabled=true on every row', () => {
      const config: GooglePlayConfigForm = {
        packageName: 'com.example.app',
        serviceAccountJson: '',
      }

      const result = buildGoogleConfigRequest(config)

      expect(result.length).toBeGreaterThan(0)
      expect(result.every((r) => r.configType === PAYMENT_PROVIDERS.GOOGLE)).toBe(true)
      expect(result.every((r) => r.enabled === true)).toBe(true)
    })
  })
})
