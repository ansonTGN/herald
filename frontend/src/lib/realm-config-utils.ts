import type { RealmConfigResponse } from '@/lib/api-generated'
import type { TOTPConfigForm, RegistrationConfigForm } from '@/lib/schemas/realm-config'

/**
 * Parses TOTP configuration from realm config array
 * Converts backend snake_case fields to frontend camelCase
 */
export function parseTOTPConfig(configs: RealmConfigResponse[]): TOTPConfigForm {
  const totpSettings = configs.find((c) => c.configType === 'totp' && c.configKey === 'settings')

  if (!totpSettings) {
    return {
      enabled: false,
      forceEnabled: false,
    }
  }

  try {
    const parsed = JSON.parse(totpSettings.configValue)
    return {
      enabled: parsed.enabled ?? false,
      forceEnabled: parsed.force_enabled ?? false,
    }
  } catch (error) {
    console.error('Failed to parse TOTP config:', error)
    return {
      enabled: false,
      forceEnabled: false,
    }
  }
}

/**
 * Parses Registration configuration from realm config array
 */
export function parseRegistrationConfig(configs: RealmConfigResponse[]): RegistrationConfigForm {
  const allowedConfig = configs.find(
    (c) => c.configType === 'registration' && c.configKey === 'allowed'
  )
  const requireEmailConfig = configs.find(
    (c) => c.configType === 'registration' && c.configKey === 'require_email_verification'
  )

  return {
    allowed: allowedConfig?.configValue === 'true',
    requireEmailVerification: requireEmailConfig?.configValue === 'true',
  }
}

/**
 * Builds TOTP config request for upsert operation
 * Converts frontend camelCase to backend snake_case
 */
export function buildTOTPConfigRequest(config: TOTPConfigForm) {
  return {
    configType: 'totp' as const,
    configKey: 'settings',
    configValue: JSON.stringify({
      enabled: config.enabled,
      force_enabled: config.forceEnabled,
    }),
    isSecret: false,
    enabled: config.enabled,
  }
}

/**
 * Builds Registration config request for upsert operation
 */
export function buildRegistrationConfigRequest(config: RegistrationConfigForm) {
  return [
    {
      configType: 'registration' as const,
      configKey: 'allowed',
      configValue: config.allowed ? 'true' : 'false',
      isSecret: false,
      enabled: true,
    },
    {
      configType: 'registration' as const,
      configKey: 'require_email_verification',
      configValue: config.requireEmailVerification ? 'true' : 'false',
      isSecret: false,
      enabled: true,
    },
  ]
}
