import type { RealmConfigResponse } from '@/lib/api-generated'
import type { CreemConfigForm } from '@/lib/schemas/creem-config'
import { PAYMENT_PROVIDERS } from '@/lib/billing-constants'
import { parseProviderConfig, buildProviderConfigRequest } from '@/lib/provider-config-utils'

export const CREEM_CONFIG_KEYS = {
  ENABLED: 'enabled',
  API_KEY: 'api_key',
  TIMEOUT: 'timeout',
  WEBHOOK_SECRET: 'webhook_secret',
} as const

const CREEM_KEY_MAPPINGS = [
  { configKey: CREEM_CONFIG_KEYS.ENABLED, fieldName: 'enabled', transform: (v?: string) => v === 'true' },
  { configKey: CREEM_CONFIG_KEYS.API_KEY, fieldName: 'apiKey', isSecret: true },
  { configKey: CREEM_CONFIG_KEYS.TIMEOUT, fieldName: 'timeout', transform: (v?: string) => (v ? Number(v) : 30) },
  { configKey: CREEM_CONFIG_KEYS.WEBHOOK_SECRET, fieldName: 'webhookSecret', isSecret: true },
] as const

export function parseCreemConfig(configs: RealmConfigResponse[]): CreemConfigForm {
  return parseProviderConfig<CreemConfigForm>(
    configs,
    PAYMENT_PROVIDERS.CREEM,
    [...CREEM_KEY_MAPPINGS],
    {
      enabled: false,
      apiKey: '',
      timeout: 30,
      webhookSecret: '',
    }
  )
}

export function buildCreemConfigRequest(config: CreemConfigForm) {
  return buildProviderConfigRequest(
    config,
    PAYMENT_PROVIDERS.CREEM,
    [...CREEM_KEY_MAPPINGS],
    config.enabled
  )
}
