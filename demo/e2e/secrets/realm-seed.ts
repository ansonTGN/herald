/**
 * Realm seed utilities for injecting third-party credentials via API.
 *
 * These functions use `page.request` (APIRequestContext) so they inherit
 * the browser context's auth cookies. Call them AFTER the page is authenticated.
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
// WeChat Pay
// POST /api/third/pay/{realmId}/providers/wechat  (camelCase body)
// PUT  /api/third/pay/{realmId}/providers/wechat  (update if exists)
// ---------------------------------------------------------------------------

export interface WechatSeedInput {
  appId: string
  mchId: string
  v3Key: string
  serialNo: string
  privateKey: string
  notifyUrl: string
}

export async function seedWechatConfig(
  request: APIRequestContext,
  realmId: string,
  input: WechatSeedInput,
): Promise<void> {
  const body = {
    appId: input.appId,
    mchId: input.mchId,
    v3Key: input.v3Key,
    serialNo: input.serialNo,
    privateKey: input.privateKey,
    notifyUrl: input.notifyUrl,
  }

  const response = await request.post(
    `${BASE_URL}/api/third/pay/${realmId}/providers/wechat`,
    { data: body },
  )

  if (response.status() === 409) {
    const updateResponse = await request.put(
      `${BASE_URL}/api/third/pay/${realmId}/providers/wechat`,
      { data: body },
    )
    if (!updateResponse.ok()) {
      const text = await updateResponse.text()
      throw new Error(
        `Failed to update WeChat config in realm "${realmId}": ${updateResponse.status()} ${text}`,
      )
    }
    console.log(`[realm-seed] WeChat Pay config updated in realm "${realmId}"`)
    return
  }

  if (!response.ok()) {
    const text = await response.text()
    throw new Error(
      `Failed to create WeChat config in realm "${realmId}": ${response.status()} ${text}`,
    )
  }

  console.log(`[realm-seed] WeChat Pay config seeded in realm "${realmId}"`)
}
