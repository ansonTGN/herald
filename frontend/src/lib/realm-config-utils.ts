import type { RealmConfigResponse } from '@/lib/api-generated'
import type {
  TOTPConfigForm,
  RegistrationConfigForm,
  EmailConfigForm,
} from '@/lib/schemas/realm-config'

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
    const parsed = JSON.parse(totpSettings.configValue ?? '{}')
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

/** Masked placeholder returned by backend for secret fields */
const MASKED_SECRET = '••••••••'

/**
 * Parses Email configuration from realm config array
 */
export function parseEmailConfig(configs: RealmConfigResponse[]): EmailConfigForm {
  const emailConfigs = configs.filter((c) => c.configType === 'email')

  const find = (key: string) => emailConfigs.find((c) => c.configKey === key)

  return {
    provider: (find('provider')?.configValue as 'resend' | 'smtp') || 'resend',
    fromAddress: find('from_address')?.configValue ?? '',
    resendApiKey: find('resend_api_key')?.configValue ?? undefined,
    smtpHost: find('smtp_host')?.configValue ?? undefined,
    smtpPort: find('smtp_port')?.configValue ?? '587',
    smtpUsername: find('smtp_username')?.configValue ?? undefined,
    smtpPassword: find('smtp_password')?.configValue ?? undefined,
    smtpEncryption: (find('smtp_encryption')?.configValue as 'starttls' | 'ssl') ?? 'starttls',
  }
}

/**
 * Builds Email config request for upsert operation.
 * Secret fields (resendApiKey, smtpPassword) are only included when the user
 * has changed them (i.e. the value differs from the masked placeholder).
 */
export function buildEmailConfigRequest(config: EmailConfigForm) {
  const entries: {
    configKey: string
    configValue: string
    isSecret: boolean
  }[] = [
    { configKey: 'provider', configValue: config.provider, isSecret: false },
    { configKey: 'from_address', configValue: config.fromAddress, isSecret: false },
    { configKey: 'smtp_host', configValue: config.smtpHost ?? '', isSecret: false },
    { configKey: 'smtp_port', configValue: config.smtpPort, isSecret: false },
    { configKey: 'smtp_username', configValue: config.smtpUsername ?? '', isSecret: false },
    {
      configKey: 'smtp_encryption',
      configValue: config.smtpEncryption,
      isSecret: false,
    },
  ]

  // Only include secret fields when the user has entered a new value
  if (config.resendApiKey && config.resendApiKey !== MASKED_SECRET) {
    entries.push({
      configKey: 'resend_api_key',
      configValue: config.resendApiKey,
      isSecret: true,
    })
  }

  if (config.smtpPassword && config.smtpPassword !== MASKED_SECRET) {
    entries.push({
      configKey: 'smtp_password',
      configValue: config.smtpPassword,
      isSecret: true,
    })
  }

  return entries.map((entry) => ({
    configType: 'email' as const,
    configKey: entry.configKey,
    configValue: entry.configValue,
    isSecret: entry.isSecret,
  }))
}
