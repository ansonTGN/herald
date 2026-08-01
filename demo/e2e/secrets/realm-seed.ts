/**
 * Realm seed utilities for injecting third-party credentials via API.
 *
 * These functions require an authenticated APIRequestContext. Under the browser
 * Bearer model, callers must provide a context with an Authorization header.
 *
 * Usage:
 *   import { seedOAuthConfig, seedCreemConfig, seedStripeConfig } from '../secrets/realm-seed'
 *
 *   test('demo', async ({ page }) => {
 *     await loginAsAdmin(page, { realmId: 'admin' })
 *     await seedOAuthConfig(page, 'admin', { providerType: 'github', clientId: '...', clientSecret: '...' })
 *   })
 */

import { type APIRequestContext } from '@playwright/test'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'

// ---------------------------------------------------------------------------
// OAuth
// POST /api/oauth/{realmId}/configs  (camelCase body)
// ---------------------------------------------------------------------------

export interface OAuthSeedInput {
  providerType: string
  clientId: string
  clientSecret: string
  scopes?: string[]
  enabled?: boolean
}

export async function seedOAuthConfig(
  request: APIRequestContext,
  realmId: string,
  input: OAuthSeedInput,
): Promise<void> {
  const response = await request.post(
    `${BASE_URL}/api/oauth/${realmId}/configs`,
    {
      data: {
        providerType: input.providerType,
        clientId: input.clientId,
        clientSecret: input.clientSecret,
        scopes: input.scopes,
        enabled: input.enabled ?? true,
      },
    },
  )

  if (response.status() === 409) {
    // Already exists -- update via PUT instead
    console.log(
      `[realm-seed] OAuth config for "${input.providerType}" already exists in realm "${realmId}", updating...`,
    )
    const updateResponse = await request.put(
      `${BASE_URL}/api/oauth/${realmId}/configs/${input.providerType}`,
      {
        data: {
          clientId: input.clientId,
          clientSecret: input.clientSecret,
          scopes: input.scopes,
          enabled: input.enabled ?? true,
        },
      },
    )
    if (!updateResponse.ok()) {
      const body = await updateResponse.text()
      throw new Error(
        `Failed to update OAuth config for "${input.providerType}": ${updateResponse.status()} ${body}`,
      )
    }
    console.log(`[realm-seed] OAuth config for "${input.providerType}" updated in realm "${realmId}"`)
    return
  }

  if (!response.ok()) {
    const body = await response.text()
    throw new Error(
      `Failed to create OAuth config for "${input.providerType}": ${response.status()} ${body}`,
    )
  }

  console.log(`[realm-seed] OAuth config for "${input.providerType}" seeded in realm "${realmId}"`)
}

// ---------------------------------------------------------------------------
// Realm Config batch
// POST /api/configs/{realmId}/batch  (camelCase body, array of configs)
// ---------------------------------------------------------------------------

interface BatchConfigItem {
  configType: string
  configKey: string
  configValue: string
  isSecret?: boolean
  enabled?: boolean
}

async function seedBatchConfigs(
  request: APIRequestContext,
  realmId: string,
  configs: BatchConfigItem[],
): Promise<void> {
  const response = await request.post(
    `${BASE_URL}/api/configs/${realmId}/batch`,
    {
      data: {
        configs: configs.map((c) => ({
          configType: c.configType,
          configKey: c.configKey,
          configValue: c.configValue,
          isSecret: c.isSecret ?? true,
          enabled: c.enabled ?? true,
        })),
      },
    },
  )

  if (!response.ok()) {
    const body = await response.text()
    throw new Error(
      `Failed to seed batch configs in realm "${realmId}": ${response.status()} ${body}`,
    )
  }

  console.log(
    `[realm-seed] ${configs.length} config(s) seeded in realm "${realmId}" (${configs[0]?.configType})`,
  )
}

// ---------------------------------------------------------------------------
// Creem
// ---------------------------------------------------------------------------

export interface CreemSeedInput {
  apiKey: string
  webhookSecret: string
}

export async function seedCreemConfig(
  request: APIRequestContext,
  realmId: string,
  input: CreemSeedInput,
): Promise<void> {
  await seedBatchConfigs(request, realmId, [
    { configType: 'creem', configKey: 'api_key', configValue: input.apiKey, isSecret: true },
    { configType: 'creem', configKey: 'webhook_secret', configValue: input.webhookSecret, isSecret: true },
  ])
}

// ---------------------------------------------------------------------------
// Stripe
// ---------------------------------------------------------------------------

export interface StripeSeedInput {
  publishableKey: string
  secretKey: string
  webhookSecret: string
}

export async function seedStripeConfig(
  request: APIRequestContext,
  realmId: string,
  input: StripeSeedInput,
): Promise<void> {
  await seedBatchConfigs(request, realmId, [
    { configType: 'stripe', configKey: 'publishable_key', configValue: input.publishableKey, isSecret: false },
    { configType: 'stripe', configKey: 'api_key', configValue: input.secretKey, isSecret: true },
    { configType: 'stripe', configKey: 'webhook_secret', configValue: input.webhookSecret, isSecret: true },
  ])
}

// ---------------------------------------------------------------------------
// Email (SMTP)
// POST /api/configs/{realmId}/batch  (config_type='email', snake_case keys)
// Key set mirrors herald_core::third::email::read_email_config expectations.
// ---------------------------------------------------------------------------

export interface EmailSmtpSeedInput {
  smtpHost: string
  smtpPort: string // note: string, parsed to u16 by backend
  smtpUsername: string
  smtpPassword: string
  smtpEncryption: string // "ssl" | "starttls"
  fromAddress: string
}

export async function seedEmailSmtpConfig(
  request: APIRequestContext,
  realmId: string,
  input: EmailSmtpSeedInput,
): Promise<void> {
  await seedBatchConfigs(request, realmId, [
    { configType: 'email', configKey: 'provider', configValue: 'smtp', isSecret: false },
    { configType: 'email', configKey: 'from_address', configValue: input.fromAddress, isSecret: false },
    { configType: 'email', configKey: 'smtp_host', configValue: input.smtpHost, isSecret: false },
    { configType: 'email', configKey: 'smtp_port', configValue: input.smtpPort, isSecret: false },
    { configType: 'email', configKey: 'smtp_username', configValue: input.smtpUsername, isSecret: false },
    { configType: 'email', configKey: 'smtp_password', configValue: input.smtpPassword, isSecret: true },
    { configType: 'email', configKey: 'smtp_encryption', configValue: input.smtpEncryption, isSecret: false },
  ])
}
