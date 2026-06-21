import type { RealmConfigResponse } from '@/lib/api-generated'
import type { StripeConfigForm } from '@/lib/schemas/stripe-config'
import { PAYMENT_PROVIDERS, STRIPE_CONFIG_KEYS } from '@/lib/billing-constants'
import { parseProviderConfig, buildProviderConfigRequest } from '@/lib/provider-config-utils'

const STRIPE_KEY_MAPPINGS = [
  { configKey: STRIPE_CONFIG_KEYS.PUBLISHABLE_KEY, fieldName: 'publishableKey' },
  {
    configKey: STRIPE_CONFIG_KEYS.API_KEY,
    fieldName: 'secretKey',
    isSecret: true,
    transform: () => '',
  },
  {
    configKey: STRIPE_CONFIG_KEYS.WEBHOOK_SECRET,
    fieldName: 'webhookSecret',
    isSecret: true,
    transform: () => '',
  },
  {
    configKey: STRIPE_CONFIG_KEYS.ASYNC_POINTS_STRATEGY,
    fieldName: 'asyncPointsStrategy',
    transform: (v?: string) => (v === 'eager' ? 'eager' : 'conservative'),
  },
] as const

export function parseStripeConfig(configs: RealmConfigResponse[]): StripeConfigForm {
  return parseProviderConfig<StripeConfigForm>(
    configs,
    PAYMENT_PROVIDERS.STRIPE,
    [...STRIPE_KEY_MAPPINGS],
    {
      publishableKey: '',
      secretKey: '',
      webhookSecret: '',
      asyncPointsStrategy: 'conservative',
    }
  )
}

export function buildStripeConfigRequest(config: StripeConfigForm) {
  return buildProviderConfigRequest(config, PAYMENT_PROVIDERS.STRIPE, [...STRIPE_KEY_MAPPINGS])
}
