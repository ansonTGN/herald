import type { RealmConfigResponse } from '@/lib/api-generated'
import type { WechatConfigForm } from '@/lib/schemas/wechat-config'
import { PAYMENT_PROVIDERS } from '@/lib/billing-constants'
import { parseProviderConfig, buildProviderConfigRequest } from '@/lib/provider-config-utils'

export const WECHAT_CONFIG_KEYS = {
  APP_ID: 'app_id',
  MCH_ID: 'mch_id',
  PRIVATE_KEY: 'private_key',
  SERIAL_NO: 'serial_no',
  V3_KEY: 'v3_key',
  NOTIFY_URL: 'notify_url',
  PLATFORM_PUBLIC_KEY: 'platform_public_key',
} as const

const WECHAT_KEY_MAPPINGS = [
  { configKey: WECHAT_CONFIG_KEYS.APP_ID, fieldName: 'appId' },
  { configKey: WECHAT_CONFIG_KEYS.MCH_ID, fieldName: 'mchId' },
  { configKey: WECHAT_CONFIG_KEYS.PRIVATE_KEY, fieldName: 'privateKey', isSecret: true },
  { configKey: WECHAT_CONFIG_KEYS.SERIAL_NO, fieldName: 'serialNo' },
  { configKey: WECHAT_CONFIG_KEYS.V3_KEY, fieldName: 'v3Key', isSecret: true },
  { configKey: WECHAT_CONFIG_KEYS.NOTIFY_URL, fieldName: 'notifyUrl' },
  { configKey: WECHAT_CONFIG_KEYS.PLATFORM_PUBLIC_KEY, fieldName: 'platformPublicKey' },
] as const

export function parseWechatConfig(configs: RealmConfigResponse[]): WechatConfigForm {
  return parseProviderConfig<WechatConfigForm>(
    configs,
    PAYMENT_PROVIDERS.WECHAT,
    [...WECHAT_KEY_MAPPINGS],
    {
      appId: '',
      mchId: '',
      privateKey: '',
      serialNo: '',
      v3Key: '',
      notifyUrl: '',
      platformPublicKey: '',
    }
  )
}

export function buildWechatConfigRequest(config: WechatConfigForm) {
  const items = buildProviderConfigRequest(config, PAYMENT_PROVIDERS.WECHAT, [
    ...WECHAT_KEY_MAPPINGS,
  ])
  // The backend rejects empty config values and the platform public key is an
  // optional manual override, so omit it when unset instead of storing an
  // empty row (empty secrets are already dropped by buildProviderConfigRequest).
  return items.filter(
    (item) => !(item.configKey === WECHAT_CONFIG_KEYS.PLATFORM_PUBLIC_KEY && !item.configValue)
  )
}
