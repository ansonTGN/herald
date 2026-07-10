import { describe, test, expect, it } from 'vitest'
import {
  parseEmailConfig,
  buildEmailConfigRequest,
  emptyCustomDomainConfig,
  normalizeCustomDomainConfig,
  toUpdateCustomDomainConfigRequest,
} from '../realm-config-utils'
import type { RealmConfigResponse, UpdateCustomDomainConfigRequest } from '@/lib/api-generated'
import type { CustomDomainConfigForm } from '@/lib/schemas/realm-config'

const makeConfig = (
  configType: string,
  configKey: string,
  configValue: string
): RealmConfigResponse =>
  ({
    configType,
    configKey,
    configValue,
    enabled: true,
    isSecret: false,
    id: 'test-id',
    realmId: 'test-realm',
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
  }) as RealmConfigResponse

describe('parseEmailConfig', () => {
  test('returns defaults when email configs are empty', () => {
    const result = parseEmailConfig([])

    expect(result).toEqual({
      provider: 'resend',
      fromAddress: '',
      resendApiKey: undefined,
      smtpHost: undefined,
      smtpPort: '587',
      smtpUsername: undefined,
      smtpPassword: undefined,
      smtpEncryption: 'starttls',
    })
  })

  test('parses full resend config', () => {
    const configs: RealmConfigResponse[] = [
      makeConfig('email', 'provider', 'resend'),
      makeConfig('email', 'from_address', 'noreply@example.com'),
      makeConfig('email', 'resend_api_key', 're_xxxxx'),
    ]

    const result = parseEmailConfig(configs)

    expect(result).toEqual({
      provider: 'resend',
      fromAddress: 'noreply@example.com',
      resendApiKey: 're_xxxxx',
      smtpHost: undefined,
      smtpPort: '587',
      smtpUsername: undefined,
      smtpPassword: undefined,
      smtpEncryption: 'starttls',
    })
  })

  test('parses full smtp config', () => {
    const configs: RealmConfigResponse[] = [
      makeConfig('email', 'provider', 'smtp'),
      makeConfig('email', 'from_address', 'admin@corp.com'),
      makeConfig('email', 'smtp_host', 'smtp.corp.com'),
      makeConfig('email', 'smtp_port', '465'),
      makeConfig('email', 'smtp_username', 'admin@corp.com'),
      makeConfig('email', 'smtp_password', 'secret-pass'),
      makeConfig('email', 'smtp_encryption', 'ssl'),
    ]

    const result = parseEmailConfig(configs)

    expect(result).toEqual({
      provider: 'smtp',
      fromAddress: 'admin@corp.com',
      resendApiKey: undefined,
      smtpHost: 'smtp.corp.com',
      smtpPort: '465',
      smtpUsername: 'admin@corp.com',
      smtpPassword: 'secret-pass',
      smtpEncryption: 'ssl',
    })
  })

  test('parses partial config with defaults', () => {
    const configs: RealmConfigResponse[] = [
      makeConfig('email', 'provider', 'smtp'),
      makeConfig('email', 'smtp_host', 'smtp.example.com'),
    ]

    const result = parseEmailConfig(configs)

    expect(result.provider).toBe('smtp')
    expect(result.fromAddress).toBe('')
    expect(result.smtpHost).toBe('smtp.example.com')
    expect(result.smtpPort).toBe('587')
    expect(result.smtpEncryption).toBe('starttls')
    expect(result.smtpPassword).toBeUndefined()
  })

  test('ignores non-email config types', () => {
    const configs: RealmConfigResponse[] = [
      makeConfig('totp', 'settings', '{"enabled":true}'),
      makeConfig('registration', 'enabled', 'true'),
      makeConfig('email', 'provider', 'resend'),
    ]

    const result = parseEmailConfig(configs)

    expect(result.provider).toBe('resend')
  })
})

describe('buildEmailConfigRequest', () => {
  test('builds request with all non-secret fields', () => {
    const config = {
      provider: 'smtp' as const,
      fromAddress: 'noreply@example.com',
      smtpHost: 'smtp.example.com',
      smtpPort: '465',
      smtpUsername: 'user@example.com',
      smtpEncryption: 'ssl' as const,
      resendApiKey: undefined,
      smtpPassword: 'my-new-password',
    }

    const result = buildEmailConfigRequest(config)

    const keys = result.map((r) => r.configKey)
    expect(keys).toContain('provider')
    expect(keys).toContain('from_address')
    expect(keys).toContain('smtp_host')
    expect(keys).toContain('smtp_port')
    expect(keys).toContain('smtp_username')
    expect(keys).toContain('smtp_encryption')
    expect(keys).toContain('smtp_password')
    expect(keys).not.toContain('resend_api_key')

    // All entries have configType 'email'
    expect(result.every((r) => r.configType === 'email')).toBe(true)
  })

  test('skips masked secret values', () => {
    const config = {
      provider: 'resend' as const,
      fromAddress: 'noreply@example.com',
      resendApiKey: '••••••••', // masked placeholder
      smtpHost: undefined,
      smtpPort: '587',
      smtpUsername: undefined,
      smtpPassword: '••••••••',
      smtpEncryption: 'starttls' as const,
    }

    const result = buildEmailConfigRequest(config)

    const keys = result.map((r) => r.configKey)
    expect(keys).not.toContain('resend_api_key')
    expect(keys).not.toContain('smtp_password')
  })

  test('includes new secret values when provided', () => {
    const config = {
      provider: 'resend' as const,
      fromAddress: 'noreply@example.com',
      resendApiKey: 're_new_key_123',
      smtpHost: undefined,
      smtpPort: '587',
      smtpUsername: undefined,
      smtpPassword: undefined,
      smtpEncryption: 'starttls' as const,
    }

    const result = buildEmailConfigRequest(config)

    const apiKeyEntry = result.find((r) => r.configKey === 'resend_api_key')
    expect(apiKeyEntry).toBeDefined()
    expect(apiKeyEntry!.configValue).toBe('re_new_key_123')
    expect(apiKeyEntry!.isSecret).toBe(true)
  })

  test('marks isSecret correctly for all fields', () => {
    const config = {
      provider: 'smtp' as const,
      fromAddress: 'noreply@example.com',
      resendApiKey: 're_key',
      smtpHost: 'smtp.example.com',
      smtpPort: '587',
      smtpUsername: 'user',
      smtpPassword: 'pass',
      smtpEncryption: 'starttls' as const,
    }

    const result = buildEmailConfigRequest(config)

    // Non-secret fields
    expect(result.find((r) => r.configKey === 'provider')!.isSecret).toBe(false)
    expect(result.find((r) => r.configKey === 'from_address')!.isSecret).toBe(false)
    expect(result.find((r) => r.configKey === 'smtp_host')!.isSecret).toBe(false)
    expect(result.find((r) => r.configKey === 'smtp_port')!.isSecret).toBe(false)
    expect(result.find((r) => r.configKey === 'smtp_username')!.isSecret).toBe(false)
    expect(result.find((r) => r.configKey === 'smtp_encryption')!.isSecret).toBe(false)

    // Secret fields
    expect(result.find((r) => r.configKey === 'resend_api_key')!.isSecret).toBe(true)
    expect(result.find((r) => r.configKey === 'smtp_password')!.isSecret).toBe(true)
  })

  test('does not include enabled field in request entries', () => {
    const config = {
      provider: 'resend' as const,
      fromAddress: 'noreply@example.com',
      smtpPort: '587',
      smtpEncryption: 'starttls' as const,
    }

    const result = buildEmailConfigRequest(config)

    // None of the entries should have an 'enabled' property
    for (const entry of result) {
      expect(entry).not.toHaveProperty('enabled')
    }
  })
})

// ==================== Custom-domain mapper pure functions ====================

describe('emptyCustomDomainConfig', () => {
  test('returns a config with a null hostname (no custom login domain configured)', () => {
    expect(emptyCustomDomainConfig()).toEqual({ hostname: null })
  })
})

describe('normalizeCustomDomainConfig', () => {
  test('passes a valid custom-domain config through unchanged', () => {
    expect(normalizeCustomDomainConfig({ hostname: 'login.acme.com' })).toEqual({
      hostname: 'login.acme.com',
    })
  })

  test('keeps an empty-string hostname as-is (trim happens in toUpdate, not normalize)', () => {
    // z.string() accepts '' — the schema intentionally does not coerce empties
    // to null, so a stored-but-empty hostname round-trips without being lost.
    expect(normalizeCustomDomainConfig({ hostname: '' })).toEqual({ hostname: '' })
  })

  // A malformed stored config must never crash the admin form: safeParse fails
  // and we fall back to the empty config so the editor renders a clean state.
  it.each([
    ['null', null],
    ['undefined', undefined],
    ['empty object', {}],
    ['non-object', 'login.acme.com'],
    ['hostname of wrong type', { hostname: 123 }],
    ['hostname as array', { hostname: ['login.acme.com'] }],
    ['extra junk object', { unrelated: 'x' }],
  ])('falls back to empty config when value is %s', (_label, value) => {
    expect(normalizeCustomDomainConfig(value)).toEqual({ hostname: null })
  })
})

describe('toUpdateCustomDomainConfigRequest', () => {
  const trimCases: Array<[string, string | null, string | null]> = [
    ['trims surrounding whitespace', '  login.acme.com  ', 'login.acme.com'],
    ['collapses a whitespace-only hostname to null', '   ', null],
    ['collapses an empty hostname to null', '', null],
    ['keeps an already-trimmed hostname unchanged', 'login.acme.com', 'login.acme.com'],
    ['keeps a null hostname as null', null, null],
  ]

  it.each(trimCases)('%s', (_label, hostname, expected) => {
    const form: CustomDomainConfigForm = { hostname }
    expect(toUpdateCustomDomainConfigRequest(form)).toEqual({ hostname: expected })
  })

  test('returns a value assignable to the generated UpdateCustomDomainConfigRequest shape', () => {
    // Shape guard: ensures the mapper keeps matching the wire contract even if
    // the schema gains fields later. `hostname` must be `string | null`.
    const result: UpdateCustomDomainConfigRequest = toUpdateCustomDomainConfigRequest({
      hostname: 'login.acme.com',
    })
    expect(result).toEqual({ hostname: 'login.acme.com' })
  })
})
