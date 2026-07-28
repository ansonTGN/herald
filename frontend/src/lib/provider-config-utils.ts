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

  const result = { ...defaults }
  for (const mapping of keyMappings) {
    const row = providerConfigs.find((c) => c.configKey === mapping.configKey)
    // Preserve the default when the row is absent (e.g. secrets not echoed by
    // the backend, or an unset environment). Only overwrite when the row exists.
    if (row === undefined) {
      continue
    }
    const rawValue = row.configValue ?? undefined
    result[mapping.fieldName as keyof T] = (
      mapping.transform ? mapping.transform(rawValue) : rawValue
    ) as T[keyof T]
  }

  return result
}

/**
 * Build realm config request items from a form config object.
 * Maps form fields back to config key/value pairs for the batch upsert API.
 *
 * Each emitted config row is marked enabled=true; the provider is considered
 * active when its required credentials are configured and disabled by deleting
 * the config rows (see payment-providers-page).
 */
export function buildProviderConfigRequest<T extends Record<string, unknown>>(
  config: T,
  providerType: ConfigType,
  keyMappings: ConfigKeyMapping[]
): BuildItem[] {
  const items: BuildItem[] = keyMappings.map((mapping) => {
    const value = config[mapping.fieldName as keyof T]
    return {
      configType: providerType,
      configKey: mapping.configKey,
      configValue: String(value ?? ''),
      isSecret: mapping.isSecret ?? false,
      enabled: true,
    }
  })

  // Skip empty secret values so the backend preserves existing secrets
  return items.filter((item) => !(item.isSecret && !item.configValue))
}
