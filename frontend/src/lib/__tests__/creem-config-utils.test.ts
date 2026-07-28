import { describe, test, expect } from 'vitest'
import { parseCreemConfig, buildCreemConfigRequest } from '../creem-config-utils'
import type { RealmConfigResponse } from '@/lib/api-generated'
import type { CreemConfigForm } from '@/lib/schemas/creem-config'
import { PAYMENT_PROVIDERS } from '@/lib/billing-constants'
import { CREEM_CONFIG_KEYS } from '../creem-config-utils'

describe('creem-config-utils', () => {
  describe('parseCreemConfig', () => {
    test('maps realm config rows to CreemConfigForm (timeout transform applies)', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: PAYMENT_PROVIDERS.CREEM,
          configKey: CREEM_CONFIG_KEYS.API_KEY,
          configValue: 'ck_live_abc',
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.CREEM,
          configKey: CREEM_CONFIG_KEYS.TIMEOUT,
          configValue: '45',
          enabled: true,
        },
        // WEBHOOK_SECRET intentionally omitted: secrets are not echoed by the
        // backend; the form should keep the field empty so the admin can
        // leave it blank to retain the existing value on save.
      ]

      const result = parseCreemConfig(configs)

      expect(result).toEqual({
        apiKey: 'ck_live_abc',
        timeout: 45,
        webhookSecret: '',
      })
    })

    test('returns defaults (all empty) when no creem config rows exist', () => {
      const configs: RealmConfigResponse[] = []

      const result = parseCreemConfig(configs)

      expect(result).toEqual({
        apiKey: '',
        timeout: 30,
        webhookSecret: '',
      })
    })

    test('preserves defaults for absent rows (regression guard for parseProviderConfig)', () => {
      // Regression guard (support-iap FE-T02): the shared parseProviderConfig
      // must NOT clobber `defaults` with `undefined` when a row is absent.
      // Creem relies on this for its secret default (webhookSecret: '') and
      // its numeric default (timeout: 30). If the util regresses to assigning
      // `undefined` for missing rows, this assertion fails.
      const configs: RealmConfigResponse[] = [
        {
          configType: PAYMENT_PROVIDERS.CREEM,
          configKey: CREEM_CONFIG_KEYS.API_KEY,
          configValue: 'ck_live_abc',
          enabled: true,
        },
        // TIMEOUT and WEBHOOK_SECRET absent → must inherit defaults, NOT undefined.
      ]

      const result = parseCreemConfig(configs)

      expect(result.timeout).toBe(30)
      expect(result.webhookSecret).toBe('')
    })

    test('ignores non-creem config rows', () => {
      const configs: RealmConfigResponse[] = [
        {
          configType: PAYMENT_PROVIDERS.STRIPE,
          configKey: 'publishable_key',
          configValue: 'pk_test_123',
          enabled: true,
        },
        {
          configType: PAYMENT_PROVIDERS.CREEM,
          configKey: CREEM_CONFIG_KEYS.API_KEY,
          configValue: 'ck_live_abc',
          enabled: true,
        },
      ]

      const result = parseCreemConfig(configs)

      expect(result.apiKey).toBe('ck_live_abc')
    })
  })

  describe('buildCreemConfigRequest', () => {
    test('emits timeout as a non-secret field', () => {
      const config: CreemConfigForm = {
        apiKey: 'ck_live_abc',
        timeout: 45,
        webhookSecret: 'whsec_def',
      }

      const result = buildCreemConfigRequest(config)

      const timeoutRow = result.find((r) => r.configKey === CREEM_CONFIG_KEYS.TIMEOUT)
      expect(timeoutRow).toBeDefined()
      expect(timeoutRow?.isSecret).toBe(false)
      expect(timeoutRow?.configValue).toBe('45')
    })

    test('includes webhook_secret when provided and marks isSecret=true', () => {
      const config: CreemConfigForm = {
        apiKey: 'ck_live_abc',
        timeout: 45,
        webhookSecret: 'whsec_def',
      }

      const result = buildCreemConfigRequest(config)

      const secretRow = result.find((r) => r.configKey === CREEM_CONFIG_KEYS.WEBHOOK_SECRET)
      expect(secretRow).toBeDefined()
      expect(secretRow?.isSecret).toBe(true)
      expect(secretRow?.configValue).toBe('whsec_def')
    })

    test('omits empty webhook_secret (leave-empty-to-keep)', () => {
      // Core contract (support-iap): an empty secret MUST be dropped from the
      // upsert payload so the backend preserves the previously-stored value.
      const config: CreemConfigForm = {
        apiKey: 'ck_live_abc',
        timeout: 45,
        webhookSecret: '',
      }

      const result = buildCreemConfigRequest(config)

      expect(result.find((r) => r.configKey === CREEM_CONFIG_KEYS.WEBHOOK_SECRET)).toBeUndefined()
    })

    test('always emits configType=creem and enabled=true on every row', () => {
      const config: CreemConfigForm = {
        apiKey: 'ck_live_abc',
        timeout: 45,
        webhookSecret: '',
      }

      const result = buildCreemConfigRequest(config)

      expect(result.length).toBeGreaterThan(0)
      expect(result.every((r) => r.configType === PAYMENT_PROVIDERS.CREEM)).toBe(true)
      expect(result.every((r) => r.enabled === true)).toBe(true)
    })
  })
})
