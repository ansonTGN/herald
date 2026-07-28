import type { RealmConfigResponse } from '@/lib/api-generated'
import type { GooglePlayConfigForm } from '@/lib/schemas/google-config'
import { PAYMENT_PROVIDERS, GOOGLE_CONFIG_KEYS } from '@/lib/billing-constants'
import { parseProviderConfig, buildProviderConfigRequest } from '@/lib/provider-config-utils'

const GOOGLE_KEY_MAPPINGS = [
  { configKey: GOOGLE_CONFIG_KEYS.PACKAGE_NAME, fieldName: 'packageName' },
  {
    configKey: GOOGLE_CONFIG_KEYS.SERVICE_ACCOUNT_JSON,
    fieldName: 'serviceAccountJson',
    isSecret: true,
  },
] as const

export function parseGoogleConfig(configs: RealmConfigResponse[]): GooglePlayConfigForm {
  return parseProviderConfig<GooglePlayConfigForm>(
    configs,
    PAYMENT_PROVIDERS.GOOGLE,
    [...GOOGLE_KEY_MAPPINGS],
    {
      packageName: '',
      serviceAccountJson: '',
    }
  )
}

export function buildGoogleConfigRequest(config: GooglePlayConfigForm) {
  return buildProviderConfigRequest(config, PAYMENT_PROVIDERS.GOOGLE, [...GOOGLE_KEY_MAPPINGS])
}
