/**
 * Multiple-currency demo helpers — catalog seeding plus the
 * entitlement-slug mirror.
 *
 * These exist for deterministic Given states and afterAll cleanup in the
 * multiple-currency demo files. The UI flows themselves are what the demos
 * exercise; these helpers only arrange/restore state around them.
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
 * product, seed credentials, sync, resolve + enable the mapping rows).
 */
export async function setupMultiCurrencyDemo(opts: {
  baseUrl: string
  realmId: string
  adminEmail: string
  stripeSecretKey: string
  stripePublishableKey: string
  stripeWebhookSecret: string
}): Promise<MultiCurrencyCatalog> {
  return withRealmAdminApiContext(
    { baseUrl: opts.baseUrl, realmId: opts.realmId, adminEmail: opts.adminEmail },
    (apiContext) =>
      ensureMultiCurrencyCatalog(apiContext, {
        baseUrl: opts.baseUrl,
        realmId: opts.realmId,
        stripeSecretKey: opts.stripeSecretKey,
        stripePublishableKey: opts.stripePublishableKey,
        stripeWebhookSecret: opts.stripeWebhookSecret,
      }),
  )
}

/**
 * Shared afterAll cleanup for the multiple-currency demo files: disable the
 * catalog's mapping rows (so the realm's purchase page stays unpolluted for
 * other demos). Errors propagate; the caller's afterAll logs them without
 * masking test failures.
 */
export async function teardownMultiCurrencyDemo(opts: {
  baseUrl: string
  realmId: string
  adminEmail: string
  catalog: MultiCurrencyCatalog
}): Promise<void> {
  await withRealmAdminApiContext(
    { baseUrl: opts.baseUrl, realmId: opts.realmId, adminEmail: opts.adminEmail },
    (apiContext) =>
      disableMultiCurrencyMappings(apiContext, {
        baseUrl: opts.baseUrl,
        realmId: opts.realmId,
        product: opts.catalog.product,
      }),
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
