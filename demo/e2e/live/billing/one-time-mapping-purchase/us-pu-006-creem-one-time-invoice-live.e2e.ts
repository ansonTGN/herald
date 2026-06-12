/**
 * Live Creem One-Time Payment Invoice Verification Test
 *
 * Related User Stories: US-PU-006
 * Coverage: partial; one-time Creem checkout creates external invoice with correct fields.
 * Not Covered: points fulfillment, webhook compensation, refund, idempotency, failure/expiry, or audit outcomes.
 * Live Dependency: real Creem test credentials + one-time product
 * Manual Step: maybe, depending on Creem checkout challenge behavior
 * Run Command:
 *   cd demo
 *   npx playwright test e2e/live/billing/one-time-mapping-purchase/us-pu-006-creem-one-time-invoice-live.e2e.ts --project=demo-fast --headed
 * Skip/Fail Policy:
 *   Fails loud when required Creem one-time credentials are absent.
 *
 * Prerequisites:
 *   - CREEM_API_KEY, CREEM_WEBHOOK_SECRET, CREEM_ONETIME_PRODUCT_ID set in demo/.env.demo
 *   - Demo seed data loaded (admin realm, admin@cas.com user)
 *   - backend/config.demo.toml [frontend].url must point to a publicly
 *     reachable address (e.g. an ngrok tunnel) so Creem can deliver
 *     webhook callbacks during checkout
 *
 * Fixed test identifiers:
 *   - Entitlement Key: herald-live-creem-onetime-entitlement
 */

import { test, expect, type Frame, type Locator, type Page } from '@playwright/test'
import { secrets, requireCreemOneTimePayment } from '../../../secrets/env'
import { seedCreemConfig } from '../../../secrets/realm-seed'
import { loginAsAdmin } from '../../../helpers/auth'
import { verifyTestEnvironment } from '../../../helpers/environment-setup'
import { fulfillPayment, waitForPaymentStatus } from '../../../helpers/payment-simulation'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'
const REALM_ID = 'admin'
const ENTITLEMENT_KEY = 'herald-live-creem-onetime-entitlement'

type SearchRoot = Page | Frame

// ---------------------------------------------------------------------------
// File-private helpers
// ---------------------------------------------------------------------------

/** Navigate to a URL with retry on timeout. */
async function navigateWithRetry(page: Page, url: string, maxRetries = 2): Promise<void> {
  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 60000 })
      return
    } catch (e) {
      if (attempt === maxRetries) throw e
      console.log(`[retry] Navigation attempt ${attempt + 1} failed, retrying...`)
    }
  }
}

async function findVisibleCheckoutControl(
  page: Page,
  label: string,
  selectors: Array<(root: SearchRoot) => Locator>,
): Promise<Locator> {
  const roots: SearchRoot[] = [page, ...page.frames()]
  const visibilityTimeout = 3000

  for (const root of roots) {
    for (const selector of selectors) {
      const locator = selector(root).first()
      if (await locator.isVisible({ timeout: visibilityTimeout }).catch(() => false)) {
        return locator
      }
    }
  }

  const frames = page.frames()
    .map((frame) => `- name="${frame.name()}" url="${frame.url()}"`)
    .join('\n')
  const title = await page.title().catch(() => '<unavailable>')

  throw new Error(
    `Creem checkout ${label} control not found.\n` +
      `Current URL: ${page.url()}\n` +
      `Page title: ${title}\n` +
      `Frames:\n${frames || '- <none>'}`,
  )
}

const CARD_NUMBER_SELECTORS: Array<(root: SearchRoot) => Locator> = [
  (root) => root.getByRole('textbox', { name: /card number/i }),
  (root) => root.locator('input[autocomplete*="cc-number"]'),
  (root) => root.getByLabel(/card number/i),
  (root) => root.locator('input[name*="card" i], input[name*="number" i]'),
  (root) => root.getByPlaceholder(/4242|card|number/i),
  (root) => root.locator('input[inputmode="numeric"]').first(),
  (root) => root.locator('form input[type="text"]').first(),
]

const EXPIRY_SELECTORS: Array<(root: SearchRoot) => Locator> = [
  (root) => root.getByRole('textbox', { name: /expiration/i }),
  (root) => root.locator('input[autocomplete*="cc-exp"]'),
  (root) => root.getByLabel(/expiry|expiration|expires/i),
  (root) => root.locator('input[name*="expir" i], input[name*="expiry" i], input[name*="exp" i]'),
  (root) => root.getByPlaceholder(/MM|YY|expiry|expiration/i),
  (root) => root.locator('input[inputmode="numeric"]').nth(1),
]

const CVC_SELECTORS: Array<(root: SearchRoot) => Locator> = [
  (root) => root.getByRole('textbox', { name: /security code/i }),
  (root) => root.locator('input[autocomplete*="cc-csc"]'),
  (root) => root.getByLabel(/cvc|cvv|security code/i),
  (root) => root.locator('input[name*="cvc" i], input[name*="cvv" i]'),
  (root) => root.getByPlaceholder(/CVC|CVV|security/i),
  (root) => root.locator('input[inputmode="numeric"]').nth(2),
]

async function fillWithVerification(locator: Locator, value: string, minLength = 1): Promise<void> {
  await locator.fill(value)
  const filledValue = await locator.inputValue().catch(() => '')
  if (!filledValue || filledValue.replace(/\s/g, '').length < minLength) {
    await locator.clear().catch(() => {})
    await locator.pressSequentially(value, { delay: 50 })
  }
}

/** Find or create a client app and return its UUID. */
async function ensureClientApp(request: import('@playwright/test').APIRequestContext): Promise<string> {
  const listResp = await request.get(`${BASE_URL}/api/client/${REALM_ID}`)
  if (listResp.ok()) {
    const body = await listResp.json()
    const apps = body.items ?? body
    if (Array.isArray(apps) && apps.length > 0) {
      return apps[0].id
    }
  }

  const createResp = await request.post(`${BASE_URL}/api/client/${REALM_ID}`, {
    data: {
      clientId: `live-creem-onetime-${Date.now()}`,
      name: 'Live Creem One-Time Test App',
      redirectUris: ['http://localhost:3000/callback'],
      enabled: true,
    },
  })
  expect(createResp.ok()).toBeTruthy()
  const created = await createResp.json()
  return created.id
}

/**
 * Poll the invoice API until a Creem external invoice appears.
 * Returns the first invoice with provider='creem' and a truthy external_invoice_id.
 * Throws on timeout.
 */
async function waitForCreemInvoice(
  page: Page,
  timeout = 30000,
): Promise<{
  id: string
  provider: string
  external_invoice_id: string
  external_hosted_url: string | null
  external_pdf_url: string | null
  status: string
  total: number
  [key: string]: unknown
}> {
  const startTime = Date.now()
  let delay = 1000
  const maxDelay = 3000

  while (Date.now() - startTime < timeout) {
    const resp = await page.request.get(
      `${BASE_URL}/api/bill/${REALM_ID}/invoices?provider=creem`,
    )
    if (resp.ok()) {
      const body = await resp.json()
      const items = body.data ?? body.items ?? body
      if (Array.isArray(items)) {
        const creemInvoice = items.find(
          (inv: any) =>
            inv.provider === 'creem' &&
            inv.external_invoice_id,
        )
        if (creemInvoice) {
          return creemInvoice
        }
      }
    }
    await new Promise((resolve) => setTimeout(resolve, delay))
    delay = Math.min(delay * 1.5, maxDelay)
  }

  throw new Error(`Timed out waiting for Creem external invoice after ${timeout}ms`)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test.describe('[Live][Billing One-Time Mapping] US-PU-006: Creem one-time invoice verification', () => {

  test.beforeEach(async ({ page }) => {
    requireCreemOneTimePayment()

    await verifyTestEnvironment(page, {
      requiredRealms: [REALM_ID],
      requiredUsers: ['admin@cas.com'],
    })

    await loginAsAdmin(page, { realmId: REALM_ID })

    await seedCreemConfig(page.request, REALM_ID, {
      apiKey: secrets.creem.apiKey!,
      webhookSecret: secrets.creem.webhookSecret!,
    })

    // Cleanup stale entitlement mappings from previous runs
    try {
      const mappingsResp = await page.request.get(
        `${BASE_URL}/api/bill/${REALM_ID}/entitlement-mappings`,
      )
      if (mappingsResp.ok()) {
        const body = await mappingsResp.json()
        const items = body.items ?? body
        if (Array.isArray(items)) {
          for (const m of items) {
            if (m.entitlementKey === ENTITLEMENT_KEY) {
              await page.request.patch(
                `${BASE_URL}/api/bill/${REALM_ID}/entitlement-mappings/${m.id}`,
                { data: { entitlementKey: `stale-${ENTITLEMENT_KEY}-${Date.now()}`, enabled: false } },
              )
              console.log(`[cleanup] Reset stale mapping ${m.id}`)
            }
          }
        }
      }
    } catch (error) {
      console.error('[cleanup] Error during stale data cleanup (non-fatal):', error)
    }
  })

  test.afterEach(async ({ page }) => {
    try {
      for (const key of ['api_key', 'webhook_secret']) {
        const resp = await page.request.delete(
          `${BASE_URL}/api/configs/${REALM_ID}/creem/${key}`,
        )
        console.log(`[cleanup] Creem ${key} delete: ${resp.status()}`)
      }
    } catch (error) {
      console.error('[cleanup] Error during Creem config cleanup:', error)
    }
  })

  test('US-PU-006 Scenario 7: Creem one-time checkout creates external invoice with correct fields', async ({ page }) => {
    let clientAppId: string
    let attemptId: string

    await test.step('Given a one-time entitlement mapping is configured', async () => {
      // Sync provider products to pull real one-time product from Creem
      const syncResp = await page.request.post(
        `${BASE_URL}/api/bill/${REALM_ID}/entitlement-mappings/sync`,
        { data: { paymentProvider: 'creem' } },
      )
      expect(syncResp.ok()).toBeTruthy()

      // Find the one-time product mapping by matching the configured onetimeProductId
      const mappingsResp = await page.request.get(
        `${BASE_URL}/api/bill/${REALM_ID}/entitlement-mappings?paymentProvider=creem`,
      )
      expect(mappingsResp.ok()).toBeTruthy()
      const body = await mappingsResp.json()
      const items = body.items ?? body
      const targetMapping = items.find(
        (m: any) => m.externalProductId === secrets.creem.onetimeProductId,
      )
      expect(targetMapping, `One-time Creem product mapping not found after sync. Available products: ${JSON.stringify(items.map((m: any) => m.externalProductId))}`).toBeTruthy()

      if (targetMapping.entitlementKey !== ENTITLEMENT_KEY || !targetMapping.enabled) {
        const patchResp = await page.request.patch(
          `${BASE_URL}/api/bill/${REALM_ID}/entitlement-mappings/${targetMapping.id}`,
          {
            data: {
              entitlementKey: ENTITLEMENT_KEY,
              enabled: true,
              pointsPerPeriod: 500,
              grantPeriodType: 'one_time',
              validityDays: 30,
              grantOnSubscribe: true,
            },
          },
        )
        expect(patchResp.ok()).toBeTruthy()
      }

      clientAppId = await ensureClientApp(page.request)
      console.log(`[live-s7] Client App ID: ${clientAppId}`)
    })

    await test.step('When creating a one-time checkout session and completing payment', async () => {
      const checkoutResp = await page.request.post(
        `${BASE_URL}/api/bill/${REALM_ID}/client/${clientAppId}/checkout`,
        {
          data: {
            entitlementKey: ENTITLEMENT_KEY,
            paymentProvider: 'creem',
          },
        },
      )
      expect(checkoutResp.ok(), `Checkout failed: ${await checkoutResp.text().catch(() => '')}`).toBeTruthy()

      const checkoutBody = await checkoutResp.json()
      expect(checkoutBody.checkoutUrl).toBeTruthy()
      const checkoutUrl = checkoutBody.checkoutUrl
      console.log(`[live-s7] Creem one-time checkout URL: ${checkoutUrl}`)

      await navigateWithRetry(page, checkoutUrl)
      await page.waitForSelector('body', { timeout: 15000 }).catch(() => {})
      await page.waitForLoadState('networkidle', { timeout: 15000 }).catch(() => {})

      // Fill test card
      const cardInput = await findVisibleCheckoutControl(page, 'card number', CARD_NUMBER_SELECTORS)
      await fillWithVerification(cardInput, '4242424242424242', 16)

      const expiryInput = await findVisibleCheckoutControl(page, 'expiry', EXPIRY_SELECTORS)
      await fillWithVerification(expiryInput, '1230', 4)

      const cvcInput = await findVisibleCheckoutControl(page, 'CVC', CVC_SELECTORS)
      await fillWithVerification(cvcInput, '123', 3)

      const fullNameInput = page.getByRole('textbox', { name: /full name/i })
      if (await fullNameInput.isVisible({ timeout: 1000 }).catch(() => false)) {
        await fullNameInput.fill('Herald Demo User')
      }

      await page.screenshot({ path: 'test-results/creem-onetime-checkout-filled.png' })

      // Submit payment
      const submitButton = page.getByRole('button', { name: /pay/i }).last()
      await expect(submitButton).toBeVisible({ timeout: 5000 })
      await submitButton.scrollIntoViewIfNeeded()
      await submitButton.click()
      await page.waitForTimeout(5000)

      console.log('[live-s7] One-time payment submitted')
    })

    await test.step('And fulfill payment and verify success', async () => {
      await page.waitForURL(/\/billing\/success/, { timeout: 30000 }).catch(() => {
        console.log(`[live-s7] Browser landed at ${page.url()} instead of success page`)
      })

      await page.screenshot({ path: 'test-results/creem-onetime-redirect.png' })

      // Find the payment attempt for fulfillment
      const attemptsResp = await page.request.get(
        `${BASE_URL}/api/bill/${REALM_ID}/purchase/payment-attempts`,
      )
      if (attemptsResp.ok()) {
        const attemptsBody = await attemptsResp.json()
        const attempts = attemptsBody.items ?? attemptsBody.attempts ?? attemptsBody
        if (Array.isArray(attempts) && attempts.length > 0) {
          attemptId = attempts[0].id ?? attempts[0].attemptId
        }
      }

      if (attemptId) {
        const fulfillResult = await fulfillPayment(page.request, REALM_ID, attemptId)
        expect(fulfillResult.success, `Fulfillment failed: ${fulfillResult.error}`).toBeTruthy()
        console.log(`[live-s7] Fulfillment result: ${JSON.stringify(fulfillResult)}`)

        const finalStatus = await waitForPaymentStatus(
          page.request,
          REALM_ID,
          attemptId,
          'Succeeded',
          15000,
        )
        console.log(`[live-s7] Payment status: ${finalStatus}`)
        expect(finalStatus).not.toBe('Pending')
      } else {
        console.log('[live-s7] No payment attempt found to fulfill -- payment may have been processed via webhook')
      }
    })

    await test.step('Then a Creem external invoice exists with correct fields', async () => {
      const invoice = await waitForCreemInvoice(page, 30000)
      console.log(`[live-s7] Creem one-time external invoice: ${JSON.stringify(invoice)}`)

      expect(invoice.provider).toBe('creem')
      expect(invoice.external_invoice_id, 'Expected external_invoice_id to be a non-empty string').toBeTruthy()
      expect(invoice.status, `Expected status 'paid', got '${invoice.status}'`).toBe('paid')
      expect(invoice.total, 'Expected total > 0').toBeGreaterThan(0)
      // Creem may or may not provide hosted/pdf URLs
      if (invoice.external_hosted_url) {
        expect(invoice.external_hosted_url).toBeTruthy()
      } else {
        console.log('[live-s7] external_hosted_url not present (Creem may not provide this)')
      }
      if (invoice.external_pdf_url) {
        expect(invoice.external_pdf_url).toBeTruthy()
      } else {
        console.log('[live-s7] external_pdf_url not present (Creem may not provide this)')
      }
    })

    await test.step('And invoice detail endpoint returns full response', async () => {
      const invoice = await waitForCreemInvoice(page, 10000)
      const detailResp = await page.request.get(
        `${BASE_URL}/api/bill/${REALM_ID}/invoices/${invoice.id}`,
      )
      expect(detailResp.ok(), `Detail fetch failed: ${await detailResp.text().catch(() => '')}`).toBeTruthy()

      const detail = await detailResp.json()
      console.log(`[live-s7] Invoice detail: ${JSON.stringify(detail)}`)

      expect(detail.id).toBe(invoice.id)
      expect(detail.provider).toBe('creem')
      expect(detail.external_invoice_id).toBeTruthy()
      if (detail.external_hosted_url) {
        expect(detail.external_hosted_url).toBeTruthy()
      }
      if (detail.external_pdf_url) {
        expect(detail.external_pdf_url).toBeTruthy()
      }
    })
  })
})
