import type { RealmConfigResponse } from '@/lib/api-generated'
import type { CreemConfigForm } from '@/lib/schemas/creem-config'
import { PAYMENT_PROVIDERS } from '@/lib/billing-constants'

export const CREEM_CONFIG_KEYS = {
  ENABLED: 'creem_enabled',
  API_KEY: 'creem_api_key',
  TIMEOUT: 'creem_timeout',
  WEBHOOK_SECRET: 'creem_webhook_secret',
} as const

export function parseCreemConfig(configs: RealmConfigResponse[]): CreemConfigForm {
  const creemConfigs = configs.filter(
    (c) => c.configType === PAYMENT_PROVIDERS.CREEM
  )

  if (creemConfigs.length === 0) {
    return {
      enabled: false,
      apiKey: '',
      timeout: 30,
      webhookSecret: '',
    }
  }

  const getValue = (key: string): string | undefined =>
    creemConfigs.find((c) => c.configKey === key)?.configValue

  const enabledStr = getValue(CREEM_CONFIG_KEYS.ENABLED)
  const timeoutStr = getValue(CREEM_CONFIG_KEYS.TIMEOUT)

  return {
    enabled: enabledStr === 'true',
    apiKey: getValue(CREEM_CONFIG_KEYS.API_KEY) ?? '',
    timeout: timeoutStr ? Number(timeoutStr) : 30,
    webhookSecret: getValue(CREEM_CONFIG_KEYS.WEBHOOK_SECRET) ?? '',
  }
}

export function buildCreemConfigRequest(config: CreemConfigForm) {
  return [
    {
      configType: PAYMENT_PROVIDERS.CREEM,
      configKey: CREEM_CONFIG_KEYS.ENABLED,
      configValue: String(config.enabled),
      isSecret: false,
      enabled: config.enabled,
    },
    {
      configType: PAYMENT_PROVIDERS.CREEM,
      configKey: CREEM_CONFIG_KEYS.API_KEY,
      configValue: config.apiKey,
      isSecret: true,
      enabled: config.enabled,
    },
    {
      configType: PAYMENT_PROVIDERS.CREEM,
      configKey: CREEM_CONFIG_KEYS.TIMEOUT,
      configValue: String(config.timeout),
      isSecret: false,
      enabled: config.enabled,
    },
    {
      configType: PAYMENT_PROVIDERS.CREEM,
      configKey: CREEM_CONFIG_KEYS.WEBHOOK_SECRET,
      configValue: config.webhookSecret || '',
      isSecret: true,
      enabled: config.enabled,
    },
  ]
}
