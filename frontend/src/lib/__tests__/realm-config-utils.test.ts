import { describe, test, expect } from 'vitest'
import { parseEmailConfig, buildEmailConfigRequest } from '../realm-config-utils'
import type { RealmConfigResponse } from '@/lib/api-generated'

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
