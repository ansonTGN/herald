import type { RealmConfigResponse } from '@/lib/api-generated'
import type { StripeConfigForm } from '@/lib/schemas/stripe-config'
import { PAYMENT_PROVIDERS, STRIPE_CONFIG_KEYS } from '@/lib/billing-constants'

export function parseStripeConfig(configs: RealmConfigResponse[]): StripeConfigForm {
  const stripeConfigs = configs.filter(
    (c) => c.configType === PAYMENT_PROVIDERS.STRIPE
  )

  if (stripeConfigs.length === 0) {
    return {
      enabled: false,
      publishableKey: '',
      secretKey: '',
      webhookSecret: '',
    }
  }

  const getValue = (key: string): string | undefined =>
    stripeConfigs.find((c) => c.configKey === key)?.configValue

  const enabledStr = getValue(STRIPE_CONFIG_KEYS.ENABLED)

  return {
    enabled: enabledStr === 'true',
    publishableKey: getValue(STRIPE_CONFIG_KEYS.PUBLISHABLE_KEY) ?? '',
    secretKey: '',
    webhookSecret: '',
  }
}

export function buildStripeConfigRequest(config: StripeConfigForm) {
  const items = [
    {
      configType: PAYMENT_PROVIDERS.STRIPE,
      configKey: STRIPE_CONFIG_KEYS.ENABLED,
      configValue: String(config.enabled),
      isSecret: false,
      enabled: config.enabled,
    },
    {
      configType: PAYMENT_PROVIDERS.STRIPE,
      configKey: STRIPE_CONFIG_KEYS.PUBLISHABLE_KEY,
      configValue: config.publishableKey,
      isSecret: false,
      enabled: config.enabled,
    },
    {
      configType: PAYMENT_PROVIDERS.STRIPE,
      configKey: STRIPE_CONFIG_KEYS.API_KEY,
      configValue: config.secretKey,
      isSecret: true,
      enabled: config.enabled,
    },
    {
      configType: PAYMENT_PROVIDERS.STRIPE,
      configKey: STRIPE_CONFIG_KEYS.WEBHOOK_SECRET,
      configValue: config.webhookSecret || '',
      isSecret: true,
      enabled: config.enabled,
    },
  ]

  // Skip empty secret values so the backend preserves existing secrets
  return items.filter((item) => !(item.isSecret && !item.configValue))
}
