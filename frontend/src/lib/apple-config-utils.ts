import type { RealmConfigResponse } from '@/lib/api-generated'
import type { AppleIapConfigForm } from '@/lib/schemas/apple-config'
import { PAYMENT_PROVIDERS, APPLE_CONFIG_KEYS } from '@/lib/billing-constants'
import { parseProviderConfig, buildProviderConfigRequest } from '@/lib/provider-config-utils'

const APPLE_KEY_MAPPINGS = [
  { configKey: APPLE_CONFIG_KEYS.BUNDLE_ID, fieldName: 'bundleId' },
  { configKey: APPLE_CONFIG_KEYS.ISSUER_ID, fieldName: 'issuerId' },
  { configKey: APPLE_CONFIG_KEYS.KEY_ID, fieldName: 'keyId' },
  { configKey: APPLE_CONFIG_KEYS.PRIVATE_KEY_P8, fieldName: 'privateKeyP8', isSecret: true },
  { configKey: APPLE_CONFIG_KEYS.ENVIRONMENT, fieldName: 'environment' },
] as const

export function parseAppleConfig(configs: RealmConfigResponse[]): AppleIapConfigForm {
  return parseProviderConfig<AppleIapConfigForm>(
    configs,
    PAYMENT_PROVIDERS.APPLE,
    [...APPLE_KEY_MAPPINGS],
    {
      bundleId: '',
      issuerId: '',
      keyId: '',
      privateKeyP8: '',
      environment: 'production',
    }
  )
}

export function buildAppleConfigRequest(config: AppleIapConfigForm) {
  return buildProviderConfigRequest(config, PAYMENT_PROVIDERS.APPLE, [...APPLE_KEY_MAPPINGS])
}
