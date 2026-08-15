/**
 * Multiple-currency demo helpers — realm default currency / user preferred
 * currency API shortcuts plus the entitlement-slug mirror.
 *
 * These exist for deterministic Given states and afterAll cleanup in the
 * multiple-currency demo files. The UI write paths themselves (settings
 * billing tab, profile preferred-currency card) are what the demos exercise;
 * these helpers only arrange/restore state around them.
 */

import type { APIRequestContext } from '@playwright/test'
import {
  disableMultiCurrencyMappings,
  ensureMultiCurrencyCatalog,
  type MultiCurrencyCatalog,
} from './resolve-mappings'

export type { MultiCurrencyCatalog }

/**
 * Shared beforeAll setup for the multiple-currency demo files: as the realm
 * admin, ensure the multi-currency catalog (create-or-reuse the real Stripe
 * product, seed credentials, sync, resolve + enable the mapping rows), then
 * seed the realm default currency — `null` deletes the config row for a
 * clean "not set" start.
 */
export async function setupMultiCurrencyDemo(opts: {
  baseUrl: string
  realmId: string
  adminEmail: string
  stripeSecretKey: string
  stripePublishableKey: string
  stripeWebhookSecret: string
  seedDefaultCurrency: string | null
}): Promise<MultiCurrencyCatalog> {
  return withRealmAdminApiContext(
    { baseUrl: opts.baseUrl, realmId: opts.realmId, adminEmail: opts.adminEmail },
    async (apiContext) => {
      const ensured = await ensureMultiCurrencyCatalog(apiContext, {
        baseUrl: opts.baseUrl,
        realmId: opts.realmId,
        stripeSecretKey: opts.stripeSecretKey,
        stripePublishableKey: opts.stripePublishableKey,
        stripeWebhookSecret: opts.stripeWebhookSecret,
      })
      await setRealmDefaultCurrency(
        apiContext,
        opts.baseUrl,
        opts.realmId,
        opts.seedDefaultCurrency,
      )
      return ensured
    },
  )
}

/**
 * Shared afterAll cleanup for the multiple-currency demo files: disable the
 * catalog's mapping rows (so the realm's purchase page stays unpolluted for
 * other demos) and delete the realm default-currency row. Errors propagate;
 * the caller's afterAll logs them without masking test failures.
 */
export async function teardownMultiCurrencyDemo(opts: {
  baseUrl: string
  realmId: string
  adminEmail: string
  catalog: MultiCurrencyCatalog
}): Promise<void> {
  await withRealmAdminApiContext(
    { baseUrl: opts.baseUrl, realmId: opts.realmId, adminEmail: opts.adminEmail },
    async (apiContext) => {
      await disableMultiCurrencyMappings(apiContext, {
        baseUrl: opts.baseUrl,
        realmId: opts.realmId,
        product: opts.catalog.product,
      })
      await setRealmDefaultCurrency(apiContext, opts.baseUrl, opts.realmId, null)
    },
  )
}

/**
 * Run an admin API call in a throwaway browser session: logs in as the realm
 * admin via a dedicated context, builds a Bearer API context from the
 * post-switch access token, runs the action, then disposes everything. Shared
 * by the multiple-currency demo files' beforeAll/afterAll setup and cleanup.
 */
export async function withRealmAdminApiContext<T>(
  opts: { baseUrl: string; realmId: string; adminEmail: string; password?: string },
  action: (ctx: APIRequestContext) => Promise<T>,
): Promise<T> {
  const { chromium } = await import('@playwright/test')
  const { LoginPage } = await import('../pages/login-page')
  const { createBearerApiContext } = await import('./auth')
  const browser = await chromium.launch()
  const context = await browser.newContext()
  try {
    const page = await context.newPage()
    const loginPage = new LoginPage(page)
    await loginPage.loginAsAdmin(opts.adminEmail, opts.password ?? 'password', opts.realmId)
    const apiContext = await createBearerApiContext(loginPage.getAccessToken())
    try {
      return await action(apiContext)
    } finally {
      await apiContext.dispose()
    }
  } finally {
    await context.close()
    await browser.close()
  }
}

/**
 * Run an API call in a Bearer-authenticated context built from the logged-in
 * account's access token (the billing/profile endpoints reject the browser
 * session cookie). Pass the `loginPage` fixture after a UI login.
 */
export async function withSessionApiContext<T>(
  loginPage: import('../pages/login-page').LoginPage,
  action: (ctx: APIRequestContext) => Promise<T>,
): Promise<T> {
  const { createBearerApiContext } = await import('./auth')
  const apiContext = await createBearerApiContext(loginPage.getAccessToken())
  try {
    return await action(apiContext)
  } finally {
    await apiContext.dispose()
  }
}

/** Mirror of the frontend's entitlement slug (kebab-case of the key). */
export function slugifyEntitlementKey(key: string): string {
  return key.toLowerCase().replace(/[^a-z0-9]+/g, '-')
}

/**
 * Set (code) or clear (null) a realm's default currency via the realm-config
 * API (`billing` / `default_currency`). `null` deletes the config row — 404
 * (already absent) is an acceptable cleanup end.
 */
export async function setRealmDefaultCurrency(
  apiContext: APIRequestContext,
  baseUrl: string,
  realmId: string,
  code: string | null,
): Promise<void> {
  if (code === null) {
    const resp = await apiContext.delete(
      `${baseUrl}/api/configs/${realmId}/billing/default_currency`,
    )
    if (resp.status() !== 204 && resp.status() !== 404) {
      throw new Error(`delete billing default_currency failed: ${resp.status()}`)
    }
    return
  }
  const resp = await apiContext.post(`${baseUrl}/api/configs/${realmId}/batch`, {
    data: {
      configs: [
        {
          configType: 'billing',
          configKey: 'default_currency',
          configValue: code,
          isSecret: false,
        },
      ],
    },
  })
  if (!resp.ok()) {
    throw new Error(`seed billing default_currency failed: ${resp.status()} ${await resp.text()}`)
  }
}

/**
 * Set (code) or clear (null) the CURRENT user's preferred-currency override
 * via the profile API. The profile endpoint requires a Bearer token (the
 * browser session cookie is not accepted), so pass an API context built from
 * the logged-in user's access token (`createBearerApiContext(loginPage.getAccessToken())`).
 */
export async function setUserPreferredCurrency(
  apiContext: APIRequestContext,
  baseUrl: string,
  code: string | null,
): Promise<void> {
  const resp = await apiContext.put(`${baseUrl}/api/user/profile`, {
    data: { preferredCurrency: code },
  })
  if (!resp.ok()) {
    throw new Error(`set preferredCurrency=${code} failed: ${resp.status()} ${await resp.text()}`)
  }
}
