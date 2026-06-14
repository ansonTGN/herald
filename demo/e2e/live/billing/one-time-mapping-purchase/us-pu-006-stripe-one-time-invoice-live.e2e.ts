/**
 * Live Stripe One-Time Payment Invoice Verification Test
 *
 * Related User Stories: US-PU-006, US-IF-004
 * Coverage: partial; one-time Stripe checkout creates external invoice with correct fields,
 *   external invoice field coverage + provider filter (US-IF-004).
 * Not Covered: points fulfillment, webhook compensation, refund, idempotency, failure/expiry, or audit outcomes.
 * Live Dependency: real Stripe test credentials + one-time product
 * Manual Step: no
 * Run Command:
 *   cd demo
 *   npx playwright test e2e/live/billing/one-time-mapping-purchase/us-pu-006-stripe-one-time-invoice-live.e2e.ts --project=demo-fast --headed
 * Skip/Fail Policy:
 *   Fails loud when required Stripe one-time credentials are absent.
 *
 * Prerequisites:
 *   - STRIPE_PUBLISHABLE_KEY, STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET,
 *     STRIPE_ONETIME_PRODUCT_ID set in demo/.env.demo
 *   - Demo seed data loaded (admin realm, admin@cas.com user)
 *   - backend/config.demo.toml [frontend].url must point to a publicly
 *     reachable address (e.g. an ngrok tunnel) so Stripe can deliver
 *     webhook callbacks during checkout
 *
 * Fixed test identifiers:
 *   - Entitlement Key: herald-live-stripe-onetime-entitlement
 */

import { test, expect, type Frame, type Locator, type Page } from '@playwright/test'
import { secrets, requireStripeOneTimePayment } from '../../../secrets/env'
import { seedStripeConfig } from '../../../secrets/realm-seed'
import { loginAsAdmin } from '../../../helpers/auth'
import { verifyTestEnvironment } from '../../../helpers/environment-setup'
import { fulfillPayment, waitForPaymentStatus } from '../../../helpers/payment-simulation'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'
const REALM_ID = 'admin'
const ENTITLEMENT_KEY = 'herald-live-stripe-onetime-entitlement'

type SearchRoot = Page | Frame

// ---------------------------------------------------------------------------
// File-private helpers
// ---------------------------------------------------------------------------

async function findVisibleCheckoutControl(
  page: Page,
  label: string,
  selectors: Array<(root: SearchRoot) => Locator>,
): Promise<Locator> {
  const roots: SearchRoot[] = [page, ...page.frames()]

  for (const root of roots) {
    for (const selector of selectors) {
      const locator = selector(root).first()
      if (await locator.isVisible({ timeout: 1000 }).catch(() => false)) {
        return locator
      }
    }
  }

  const frames = page.frames()
    .map((frame) => `- name="${frame.name()}" url="${frame.url()}"`)
    .join('\n')
  const title = await page.title().catch(() => '<unavailable>')

  throw new Error(
    `Stripe checkout ${label} control not found.\n` +
      `Current URL: ${page.url()}\n` +
      `Page title: ${title}\n` +
      `Frames:\n${frames || '- <none>'}`,
  )
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
      clientId: `live-stripe-onetime-${Date.now()}`,
      name: 'Live Stripe One-Time Test App',
      redirectUris: ['http://localhost:3000/callback'],
      enabled: true,
    },
  })
  expect(createResp.ok()).toBeTruthy()
  const created = await createResp.json()
  return created.id
}

/**
 * Poll the invoice API until a Stripe external invoice appears.
 * Returns the first invoice with provider='stripe' and an external_invoice_id starting with 'in_'.
 * Throws on timeout.
 */
async function waitForStripeInvoice(
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
      `${BASE_URL}/api/bill/${REALM_ID}/invoices?provider=stripe`,
    )
    if (resp.ok()) {
      const body = await resp.json()
      const items = body.data ?? body.items ?? body
      if (Array.isArray(items)) {
        const stripeInvoice = items.find(
          (inv: any) =>
            inv.provider === 'stripe' &&
            inv.external_invoice_id?.startsWith('in_'),
        )
        if (stripeInvoice) {
          return stripeInvoice
        }
      }
    }
    await new Promise((resolve) => setTimeout(resolve, delay))
    delay = Math.min(delay * 1.5, maxDelay)
  }

  throw new Error(`Timed out waiting for Stripe external invoice after ${timeout}ms`)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test.describe('[Live][Billing One-Time Mapping] US-PU-006: Stripe one-time invoice verification', () => {

  test.beforeEach(async ({ page }) => {
    requireStripeOneTimePayment()

    await verifyTestEnvironment(page, {
      requiredRealms: [REALM_ID],
      requiredUsers: ['admin@cas.com'],
    })

    await loginAsAdmin(page, { realmId: REALM_ID })

    await seedStripeConfig(page.request, REALM_ID, {
      publishableKey: secrets.stripe.publishableKey!,
      secretKey: secrets.stripe.secretKey!,
      webhookSecret: secrets.stripe.webhookSecret!,
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
      for (const key of ['publishable_key', 'api_key', 'webhook_secret']) {
        const resp = await page.request.delete(
          `${BASE_URL}/api/configs/${REALM_ID}/stripe/${key}`,
        )
        console.log(`[cleanup] Stripe ${key} delete: ${resp.status()}`)
      }
    } catch (error) {
      console.error('[cleanup] Error during Stripe config cleanup:', error)
    }
  })

  test('US-PU-006 Scenario 7: Stripe one-time checkout creates external invoice with correct fields', async ({ page }) => {
    let clientAppId: string
    let attemptId: string

    await test.step('Given a one-time entitlement mapping is configured', async () => {
      // Sync provider products to pull real one-time product from Stripe
      const syncResp = await page.request.post(
        `${BASE_URL}/api/bill/${REALM_ID}/entitlement-mappings/sync`,
        { data: { paymentProvider: 'stripe' } },
      )
      expect(syncResp.ok()).toBeTruthy()

      // Find the one-time product mapping by matching the configured onetimeProductId
      const mappingsResp = await page.request.get(
        `${BASE_URL}/api/bill/${REALM_ID}/entitlement-mappings?paymentProvider=stripe`,
      )
      expect(mappingsResp.ok()).toBeTruthy()
      const body = await mappingsResp.json()
      const items = body.items ?? body
      const targetMapping = items.find(
        (m: any) => m.externalProductId === secrets.stripe.onetimeProductId,
      )
      expect(targetMapping, `One-time Stripe product mapping not found after sync. Available products: ${JSON.stringify(items.map((m: any) => m.externalProductId))}`).toBeTruthy()

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
            paymentProvider: 'stripe',
          },
        },
      )
      expect(checkoutResp.ok(), `Checkout failed: ${await checkoutResp.text().catch(() => '')}`).toBeTruthy()

      const checkoutBody = await checkoutResp.json()
      expect(checkoutBody.checkoutUrl).toBeTruthy()
      const checkoutUrl = checkoutBody.checkoutUrl
      console.log(`[live-s7] Stripe one-time checkout URL: ${checkoutUrl}`)

      await page.goto(checkoutUrl, { waitUntil: 'domcontentloaded', timeout: 60000 })
      await page.waitForLoadState('networkidle', { timeout: 15000 }).catch(() => {})

      // Fill test card
      const cardInput = await findVisibleCheckoutControl(page, 'card number', [
        (root) => root.locator('input[name="cardNumber"]'),
        (root) => root.locator('input[autocomplete*="cc-number"]'),
        (root) => root.getByLabel(/card number/i),
        (root) => root.getByPlaceholder(/4242|card|number/i),
      ])
      await cardInput.fill('4242424242424242')

      const expiryInput = await findVisibleCheckoutControl(page, 'expiry', [
        (root) => root.locator('input[name="cardExpiry"]'),
        (root) => root.locator('input[autocomplete*="cc-exp"]'),
        (root) => root.getByLabel(/expiry|expiration/i),
        (root) => root.getByPlaceholder(/MM|YY|expiry/i),
      ])
      await expiryInput.fill('1234')

      const cvcInput = await findVisibleCheckoutControl(page, 'CVC', [
        (root) => root.locator('input[name="cardCvc"]'),
        (root) => root.locator('input[autocomplete*="cc-csc"]'),
        (root) => root.getByLabel(/cvc|cvv|security/i),
        (root) => root.getByPlaceholder(/CVC|CVV|security/i),
      ])
      await cvcInput.fill('123')

      const nameInput = await findVisibleCheckoutControl(page, 'cardholder name', [
        (root) => root.getByLabel(/cardholder name/i),
        (root) => root.getByPlaceholder(/full name on card/i),
      ])
      await nameInput.fill('Test User')
      await nameInput.blur()
      await page.waitForTimeout(1000)

      await page.screenshot({ path: 'test-results/stripe-onetime-checkout-filled.png' })

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

      await page.screenshot({ path: 'test-results/stripe-onetime-redirect.png' })

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

    await test.step('Then a Stripe external invoice exists with correct fields', async () => {
      const invoice = await waitForStripeInvoice(page, 30000)
      console.log(`[live-s7] Stripe one-time external invoice: ${JSON.stringify(invoice)}`)

      expect(invoice.provider).toBe('stripe')
      expect(invoice.external_invoice_id, 'Expected external_invoice_id to start with in_').toMatch(/^in_/)
      expect(invoice.status, `Expected status 'paid', got '${invoice.status}'`).toBe('paid')
      expect(invoice.total, 'Expected total > 0').toBeGreaterThan(0)
      expect(invoice.external_hosted_url, 'Expected external_hosted_url to be present').toBeTruthy()
      expect(invoice.external_pdf_url, 'Expected external_pdf_url to be present').toBeTruthy()
      // Expanded invoice field coverage (US-IF-004): provider linkage, source, currency, refund totals.
      expect(invoice.payment_provider, 'Expected payment_provider to be stripe').toBe('stripe')
      expect(invoice.source, 'Expected source to be external_sync').toBe('external_sync')
      expect(invoice.currency, 'Expected currency to be a 3-letter code').toMatch(/^[a-z]{3}$/)
      expect(invoice.invoice_number, 'Expected invoice_number to be present').toBeTruthy()
      expect(invoice.amount_refunded, 'Expected amount_refunded to be 0 on a fresh paid invoice').toBe(0)
    })

    await test.step('And invoice detail endpoint returns full response', async () => {
      const invoice = await waitForStripeInvoice(page, 10000)
      const detailResp = await page.request.get(
        `${BASE_URL}/api/bill/${REALM_ID}/invoices/${invoice.id}`,
      )
      expect(detailResp.ok(), `Detail fetch failed: ${await detailResp.text().catch(() => '')}`).toBeTruthy()

      const detail = await detailResp.json()
      console.log(`[live-s7] Invoice detail: ${JSON.stringify(detail)}`)

      expect(detail.id).toBe(invoice.id)
      expect(detail.provider).toBe('stripe')
      expect(detail.external_invoice_id).toMatch(/^in_/)
      expect(detail.external_hosted_url).toBeTruthy()
      expect(detail.external_pdf_url).toBeTruthy()
      // Expanded detail coverage (US-IF-004/008): provider linkage, currency, refund totals.
      expect(detail.payment_provider).toBe('stripe')
      expect(detail.source).toBe('external_sync')
      expect(detail.currency).toMatch(/^[a-z]{3}$/)
      expect(detail.invoice_number).toBeTruthy()
      expect(detail.amount_refunded, 'Expected amount_refunded to be 0 initially').toBe(0)
      expect(detail.amount_remaining, 'Expected amount_remaining to equal total initially').toBe(detail.total)
      expect(detail.external_order_id, 'Expected external_order_id (payment_intent) to be present').toBeTruthy()
    })

    await test.step('And provider filter returns only stripe invoices (US-IF-004)', async () => {
      const invoice = await waitForStripeInvoice(page, 10000)

      const stripeOnly = await page.request.get(
        `${BASE_URL}/api/bill/${REALM_ID}/invoices?provider=stripe`,
      )
      expect(stripeOnly.ok(), `provider=stripe filter failed: ${await stripeOnly.text().catch(() => '')}`).toBeTruthy()
      const stripeBody = await stripeOnly.json()
      const stripeItems = stripeBody.data ?? stripeBody.items ?? stripeBody
      expect(Array.isArray(stripeItems), 'Expected an array of stripe invoices').toBeTruthy()
      for (const inv of stripeItems as any[]) {
        expect(inv.provider, `provider=stripe filter must not leak '${inv.provider}' invoices`).toBe('stripe')
      }
      expect(
        (stripeItems as any[]).some((inv) => inv.id === invoice.id),
        'The one-time stripe invoice should appear in provider=stripe filter',
      ).toBeTruthy()

      // Negative isolation: provider=manual must NOT include the stripe invoice.
      const manualOnly = await page.request.get(
        `${BASE_URL}/api/bill/${REALM_ID}/invoices?provider=manual`,
      )
      if (manualOnly.ok()) {
        const manualBody = await manualOnly.json()
        const manualItems = manualBody.data ?? manualBody.items ?? manualBody
        if (Array.isArray(manualItems)) {
          expect(
            (manualItems as any[]).find((inv) => inv.id === invoice.id),
            'stripe invoice must not leak into provider=manual filter',
          ).toBeUndefined()
        }
      }
    })
  })
})
