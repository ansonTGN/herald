import { describe, it, expect } from 'vitest'
import type { RealmConfigResponse } from '@/lib/api-generated'
import {
  parseBillingCurrencyConfig,
  buildBillingCurrencyConfigRequest,
} from '@/lib/realm-config-utils'

// Contracts for the realm default currency's realm_config row mapping:
// the value lives in the single `billing/default_currency` row as a plain
// string, and the built request normalizes to the uppercase ISO form the
// backend validates against (3 uppercase letters, reserved codes rejected).

function makeConfigRow(overrides: Partial<RealmConfigResponse>): RealmConfigResponse {
  return {
    id: 'cfg-1',
    realmId: 'realm-1',
    configType: 'billing',
    configKey: 'default_currency',
    configValue: 'USD',
    isSecret: false,
    enabled: true,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    ...overrides,
  } as RealmConfigResponse
}

describe('parseBillingCurrencyConfig', () => {
  it('reads the default currency from the billing/default_currency row', () => {
    expect(
      parseBillingCurrencyConfig([
        makeConfigRow({}),
        makeConfigRow({ id: 'cfg-2', configKey: 'something_else', configValue: 'EUR' }),
      ])
    ).toEqual({ defaultCurrency: 'USD' })
  })

  it('treats a missing row as unset (empty string)', () => {
    expect(
      parseBillingCurrencyConfig([makeConfigRow({ configType: 'totp', configKey: 'settings' })])
    ).toEqual({ defaultCurrency: '' })
  })
})

describe('buildBillingCurrencyConfigRequest', () => {
  it('builds a single billing row with the normalized uppercase code', () => {
    expect(buildBillingCurrencyConfigRequest({ defaultCurrency: ' eur ' })).toEqual([
      {
        configType: 'billing',
        configKey: 'default_currency',
        configValue: 'EUR',
        isSecret: false,
      },
    ])
  })
})
