import type { ConfigType, RealmConfigResponse } from '@/lib/api-generated'

interface ConfigKeyMapping {
  configKey: string
  fieldName: string
  isSecret?: boolean
  transform?: (value: string | undefined) => unknown
}

interface BuildItem {
  configType: ConfigType
  configKey: string
  configValue: string
  isSecret: boolean
  enabled: boolean
}

/**
 * Parse realm configs into a typed form object.
 * Filters configs by provider type, then maps config keys to form fields.
 */
export function parseProviderConfig<T extends Record<string, unknown>>(
  configs: RealmConfigResponse[],
  providerType: string,
  keyMappings: ConfigKeyMapping[],
  defaults: T
): T {
  const providerConfigs = configs.filter((c) => c.configType === providerType)

  if (providerConfigs.length === 0) {
    return defaults
  }

  const getValue = (key: string): string | undefined =>
    providerConfigs.find((c) => c.configKey === key)?.configValue

  const result = { ...defaults }
  for (const mapping of keyMappings) {
    const rawValue = getValue(mapping.configKey)
    result[mapping.fieldName as keyof T] = (
      mapping.transform ? mapping.transform(rawValue) : rawValue
    ) as T[keyof T]
  }

  return result
}

/**
 * Build realm config request items from a form config object.
 * Maps form fields back to config key/value pairs for the batch upsert API.
 */
export function buildProviderConfigRequest<T extends Record<string, unknown>>(
  config: T,
  providerType: ConfigType,
  keyMappings: ConfigKeyMapping[],
  enabled: boolean
): BuildItem[] {
  const items: BuildItem[] = keyMappings.map((mapping) => {
    const value = config[mapping.fieldName as keyof T]
    return {
      configType: providerType,
      configKey: mapping.configKey,
      configValue: String(value ?? ''),
      isSecret: mapping.isSecret ?? false,
      enabled,
    }
  })

  // Skip empty secret values so the backend preserves existing secrets
  return items.filter((item) => !(item.isSecret && !item.configValue))
}
