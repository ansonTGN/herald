import type {
  RealmConfigResponse,
  UpdateWhiteLabelConfigRequest,
  UpdateCustomDomainConfigRequest,
} from '@/lib/api-generated'
import {
  whiteLabelConfigSchema,
  customDomainConfigSchema,
  type TOTPConfigForm,
  type RegistrationConfigForm,
  type TurnstileConfigForm,
  type EmailConfigForm,
  type WhiteLabelConfigForm,
  type WhiteLabelBackgroundForm,
  type CustomDomainConfigForm,
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
  const enabledConfig = configs.find(
    (c) => c.configType === 'registration' && c.configKey === 'enabled'
  )
  const requireEmailConfig = configs.find(
    (c) => c.configType === 'registration' && c.configKey === 'require_email_verification'
  )

  return {
    enabled: enabledConfig?.configValue === 'true',
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
      configKey: 'enabled',
      configValue: config.enabled ? 'true' : 'false',
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
 * Parses Turnstile configuration from realm config array
 */
export function parseTurnstileConfig(configs: RealmConfigResponse[]): TurnstileConfigForm {
  const turnstileConfigs = configs.filter((c) => c.configType === 'turnstile')

  const find = (key: string) => turnstileConfigs.find((c) => c.configKey === key)

  return {
    siteKey: find('site_key')?.configValue ?? '',
    secretKey: find('secret_key')?.configValue ?? '',
  }
}

/**
 * Builds Turnstile config request for upsert operation.
 * Secret field (secretKey) is only included when the user has changed it.
 */
export function buildTurnstileConfigRequest(config: TurnstileConfigForm) {
  const entries: {
    configKey: string
    configValue: string
    isSecret: boolean
  }[] = [{ configKey: 'site_key', configValue: config.siteKey, isSecret: false }]

  // Only include secret field when the user has entered a new value
  if (config.secretKey && config.secretKey !== MASKED_SECRET) {
    entries.push({
      configKey: 'secret_key',
      configValue: config.secretKey,
      isSecret: true,
    })
  }

  return entries.map((entry) => ({
    configType: 'turnstile' as const,
    configKey: entry.configKey,
    configValue: entry.configValue,
    isSecret: entry.isSecret,
  }))
}

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

// ==================== White-label 配置 ====================

/**
 * Empty white-label form values. All fields default to `null` so the form
 * starts with no branding applied (terminal pages fall back to Herald defaults).
 */
export function emptyWhiteLabelConfig(): WhiteLabelConfigForm {
  return {
    brandName: null,
    logoUrl: null,
    faviconUrl: null,
    accentColor: null,
    background: null,
    footerText: null,
    loginTitle: null,
    loginSubtitle: null,
    registerTitle: null,
    registerSubtitle: null,
  }
}

/**
 * Safely parses an unknown value (e.g. a backend `WhiteLabelConfig` object or
 * raw JSON) into a `WhiteLabelConfigForm`. Invalid or missing fields fall back
 * to `null`, so a malformed stored config never crashes the admin form.
 */
export function normalizeWhiteLabelConfig(value: unknown): WhiteLabelConfigForm {
  const candidate =
    typeof value === 'object' && value !== null
      ? { brandName: null, faviconUrl: null, ...value }
      : value
  const parsed = whiteLabelConfigSchema.safeParse(candidate)
  if (!parsed.success) {
    return emptyWhiteLabelConfig()
  }
  return parsed.data
}

/**
 * Normalizes a single string field: trims whitespace and converts empty strings
 * to `null` (the backend treats `null` and empty string equivalently via its
 * own `normalize_optional_string`, but we send `null` to keep the wire shape clean).
 */
function normalizeOptionalString(value: string | null): string | null {
  if (value === null) return null
  const trimmed = value.trim()
  return trimmed === '' ? null : trimmed
}

/**
 * Normalizes the background field: a background with an empty `value` collapses
 * to `null` so the backend does not store a background with no value.
 */
function normalizeBackground(
  background: WhiteLabelBackgroundForm | null
): WhiteLabelBackgroundForm | null {
  if (!background) return null
  const trimmedValue = background.value.trim()
  if (trimmedValue === '') return null
  return { type: background.type, value: trimmedValue }
}

/**
 * Converts form values into the backend PUT /draft (or POST /publish) request
 * body. Empty strings are normalized to `null`; whitespace is trimmed. The
 * returned shape matches the generated `UpdateWhiteLabelConfigRequest`.
 */
export function toUpdateWhiteLabelConfigRequest(
  config: WhiteLabelConfigForm
): UpdateWhiteLabelConfigRequest {
  return {
    brandName: normalizeOptionalString(config.brandName),
    logoUrl: normalizeOptionalString(config.logoUrl),
    faviconUrl: normalizeOptionalString(config.faviconUrl),
    accentColor: normalizeOptionalString(config.accentColor),
    background: normalizeBackground(config.background),
    footerText: normalizeOptionalString(config.footerText),
    loginTitle: normalizeOptionalString(config.loginTitle),
    loginSubtitle: normalizeOptionalString(config.loginSubtitle),
    registerTitle: normalizeOptionalString(config.registerTitle),
    registerSubtitle: normalizeOptionalString(config.registerSubtitle),
  }
}

// ==================== Custom-domain 配置 ====================

/**
 * Empty custom-domain form values. `hostname` defaults to `null` so the form
 * starts with no custom login domain configured.
 */
export function emptyCustomDomainConfig(): CustomDomainConfigForm {
  return {
    hostname: null,
  }
}

/**
 * Safely parses an unknown value (e.g. a backend `CustomDomainConfig` object or
 * raw JSON) into a `CustomDomainConfigForm`. Invalid or missing fields fall back
 * to `emptyCustomDomainConfig()`, so a malformed stored config never crashes the
 * admin form. Used by the settings tab to normalize `published`.
 */
export function normalizeCustomDomainConfig(value: unknown): CustomDomainConfigForm {
  const parsed = customDomainConfigSchema.safeParse(value)
  if (!parsed.success) {
    return emptyCustomDomainConfig()
  }
  return parsed.data
}

/**
 * Converts form values into the backend PUT request body. The hostname is
 * trimmed and empty strings are normalized to `null`. The returned shape matches
 * the generated `UpdateCustomDomainConfigRequest` (`{ hostname: string | null }`).
 */
export function toUpdateCustomDomainConfigRequest(
  config: CustomDomainConfigForm
): UpdateCustomDomainConfigRequest {
  return {
    hostname: normalizeOptionalString(config.hostname),
  }
}
