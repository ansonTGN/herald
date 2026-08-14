import { describe, it, expect } from 'vitest'
import type { RealmConfigResponse } from '@/lib/api-generated'
import {
  WECHAT_CONFIG_KEYS,
  parseWechatConfig,
  buildWechatConfigRequest,
} from '../wechat-config-utils'

function makeConfigRow(
  configKey: string,
  configValue: string | null,
  overrides?: Partial<RealmConfigResponse>
): RealmConfigResponse {
  return {
    id: `row-${configKey}`,
    realmId: 'realm-1',
    configType: 'wechat',
    configKey,
    configValue,
    isSecret: false,
    enabled: true,
    createdAt: '2026-08-14T00:00:00Z',
    updatedAt: '2026-08-14T00:00:00Z',
    ...overrides,
  }
}

describe('parseWechatConfig', () => {
  it('returns all-blank defaults when the realm has no wechat rows', () => {
    expect(parseWechatConfig([])).toEqual({
      appId: '',
      mchId: '',
      privateKey: '',
      serialNo: '',
      v3Key: '',
      notifyUrl: '',
      platformPublicKey: '',
    })
  })

  it('maps config rows to form fields and keeps secrets blank when not echoed', () => {
    // The backend never echoes secret values, so their rows are absent and
    // the form must stay blank (edit keeps the stored secret by leaving it
    // blank), while non-secret rows load normally.
    const parsed = parseWechatConfig([
      makeConfigRow('app_id', 'wx1234'),
      makeConfigRow('mch_id', '1900000109'),
      makeConfigRow('serial_no', 'SERIAL1'),
      makeConfigRow('notify_url', 'https://example.com/notify'),
      makeConfigRow('platform_public_key', '-----BEGIN PUBLIC KEY-----'),
    ])

    expect(parsed).toEqual({
      appId: 'wx1234',
      mchId: '1900000109',
      privateKey: '',
      serialNo: 'SERIAL1',
      v3Key: '',
      notifyUrl: 'https://example.com/notify',
      platformPublicKey: '-----BEGIN PUBLIC KEY-----',
    })
  })

  it('ignores rows from other config types', () => {
    const parsed = parseWechatConfig([
      makeConfigRow('app_id', 'wx1234'),
      makeConfigRow('api_key', 'sk_live_stripe', { configType: 'stripe' }),
    ])
    expect(parsed.appId).toBe('wx1234')
  })
})

describe('buildWechatConfigRequest', () => {
  const FULL_FORM = {
    appId: 'wx1234',
    mchId: '1900000109',
    privateKey: '-----BEGIN PRIVATE KEY-----',
    serialNo: 'SERIAL1',
    v3Key: 'v3key32chars0000000000000000000',
    notifyUrl: 'https://example.com/notify',
    platformPublicKey: '-----BEGIN PUBLIC KEY-----',
  }

  it('emits all seven keys as configType wechat with secrets only on private_key/v3_key', () => {
    const items = buildWechatConfigRequest(FULL_FORM)
    expect(items.map((i) => i.configKey).sort()).toEqual(
      Object.values(WECHAT_CONFIG_KEYS).slice().sort()
    )
    for (const item of items) {
      expect(item.configType).toBe('wechat')
      expect(item.enabled).toBe(true)
    }
    const secretKeys = items
      .filter((i) => i.isSecret)
      .map((i) => i.configKey)
      .sort()
    expect(secretKeys).toEqual(['private_key', 'v3_key'])
  })

  it('drops blank secrets so editing keeps the stored merchant key and APIv3 key', () => {
    const items = buildWechatConfigRequest({ ...FULL_FORM, privateKey: '', v3Key: '' })
    expect(items.map((i) => i.configKey)).not.toContain('private_key')
    expect(items.map((i) => i.configKey)).not.toContain('v3_key')
    // Non-secret fields are still submitted.
    expect(items.map((i) => i.configKey)).toContain('app_id')
  })

  it('omits a blank platform public key instead of submitting an empty config value', () => {
    // The backend rejects empty config values and the platform public key is
    // an optional manual override — absence must mean "auto-maintained".
    const items = buildWechatConfigRequest({ ...FULL_FORM, platformPublicKey: '' })
    expect(items.map((i) => i.configKey)).not.toContain('platform_public_key')
  })

  it('keeps a provided platform public key override', () => {
    const items = buildWechatConfigRequest(FULL_FORM)
    const row = items.find((i) => i.configKey === 'platform_public_key')
    expect(row?.configValue).toBe('-----BEGIN PUBLIC KEY-----')
    expect(row?.isSecret).toBe(false)
  })
})
