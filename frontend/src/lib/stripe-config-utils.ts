import type { RealmConfigResponse } from '@/lib/api-generated'
import type { StripeConfigForm } from '@/lib/schemas/stripe-config'
import { PAYMENT_PROVIDERS } from '@/lib/billing-constants'

interface StripeConfigJSON {
  enabled: boolean
  publishableKey: string
  secretKey: string
  webhookSecret?: string
}

export function parseStripeConfig(configs: RealmConfigResponse[]): StripeConfigForm {
  const stripeSettings = configs.find(
    (c) => c.configType === PAYMENT_PROVIDERS.STRIPE && c.configKey === 'settings'
  )

  if (!stripeSettings) {
    return {
      enabled: false,
      publishableKey: '',
      secretKey: '',
      webhookSecret: '',
    }
  }

  try {
    const parsed = JSON.parse(stripeSettings.configValue) as StripeConfigJSON
    return {
      enabled: parsed.enabled ?? false,
      publishableKey: parsed.publishableKey ?? '',
      secretKey: parsed.secretKey ?? '',
      webhookSecret: parsed.webhookSecret ?? '',
    }
  } catch (error) {
    console.error('Failed to parse Stripe config:', error)
    return {
      enabled: false,
      publishableKey: '',
      secretKey: '',
      webhookSecret: '',
    }
  }
}

export function buildStripeConfigRequest(config: StripeConfigForm) {
  const configJSON: StripeConfigJSON = {
    enabled: config.enabled,
    publishableKey: config.publishableKey,
    secretKey: config.secretKey,
    webhookSecret: config.webhookSecret || undefined,
  }

  return {
    configType: PAYMENT_PROVIDERS.STRIPE,
    configKey: 'settings',
    configValue: JSON.stringify(configJSON),
    isSecret: false, // Individual fields are marked as secret in JSON
    enabled: config.enabled,
  }
}
