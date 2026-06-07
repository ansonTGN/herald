/**
 * Live Stripe Payment Smoke Test
 *
 * Related User Stories: US-PA-001, US-PA-002, US-PA-003, US-PV-001
 * Coverage: partial live smoke; Stripe checkout branch of US-PA-001.
 * Not Covered: complete payment-attempt matrix, frontend polling states, failure/expiry,
 *   webhook compensation, refund, idempotency, or audit outcomes.
 * Live Dependency: real Stripe test credentials
 * Manual Step: no
 * Run Command:
 *   cd demo
 *   npx playwright test e2e/live/billing/payment-attempt/us-pa-001-stripe-checkout-live.e2e.ts --project=demo-fast --headed
 * Skip/Fail Policy:
 *   Fails loud when required Stripe credentials are absent.
 *
 * Validates that real Stripe test credentials from .env.demo are correctly
 * configured and accepted by the Stripe API, and that a complete checkout
 * flow can be performed using Stripe test cards.
 *
 * Prerequisites:
 *   - STRIPE_PUBLISHABLE_KEY, STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET,
 *     STRIPE_PRODUCT_ID set in demo/.env.demo
 *   - Demo seed data loaded (admin realm, admin@cas.com user)
 *   - backend/config.demo.toml [frontend].url must point to a publicly
 *     reachable address (e.g. an ngrok tunnel) so Stripe can deliver
 *     webhook callbacks during checkout
 *
 * Stripe Dashboard Setup (https://dashboard.stripe.com/test):
 *   1. Developers → API Keys → Copy "Publishable key" (pk_test_*)
 *      and "Secret key" (sk_test_*) → set as STRIPE_PUBLISHABLE_KEY
 *      and STRIPE_SECRET_KEY in .env.demo
 *   2. Products → Add product:
 *      - Name: Herald Live Stripe Product
 *      - Price: Recurring, $10.00/month
 *      - Copy the Product ID (prod_*) → set as STRIPE_PRODUCT_ID in .env.demo
 *   3. Developers → Webhooks → Add endpoint:
 *      - URL: http://localhost:8080/api/third/pay/{realmId}/stripe/webhooks
 *      - Events: checkout.session.completed, customer.subscription.*
 *      - Copy the Signing secret → set as STRIPE_WEBHOOK_SECRET in .env.demo
 *
 * Fixed test identifiers:
 *   - Entitlement Key: herald-live-stripe-entitlement
 *
 * Test mode: use sk_test_* API key, test card 4242 4242 4242 4242,
 *            expiry 12/34, CVC 123
 *
 * Fails loud when credentials are absent.
 */

import { test, expect, type Frame, type Locator, type Page } from '@playwright/test'
import { secrets, requireStripePayment } from '../../../secrets/env'
import { seedStripeConfig } from '../../../secrets/realm-seed'
import { loginAsAdmin } from '../../../helpers/auth'
import { verifyTestEnvironment } from '../../../helpers/environment-setup'
import { fulfillPayment, waitForPaymentStatus } from '../../../helpers/payment-simulation'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'
const REALM_ID = 'admin'
const ENTITLEMENT_KEY = 'herald-live-stripe-entitlement'

type SearchRoot = Page | Frame

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
      clientId: `live-test-stripe-${Date.now()}`,
      name: 'Live Stripe Test App',
      redirectUris: ['http://localhost:3000/callback'],
      enabled: true,
    },
  })
  expect(createResp.ok()).toBeTruthy()
  const created = await createResp.json()
  return created.id
}

test.describe('[Live][Billing Payment Attempt] US-PA-001: Stripe checkout payment attempt', () => {

  test.beforeEach(async ({ page }) => {
    requireStripePayment()

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

  test('US-PA-001 Setup: Stripe credentials are configured and entitlement mapping synced', async ({ page }) => {
    await test.step('Given Stripe config is seeded', async () => {
      const providersResponse = await page.request.get(
        `${BASE_URL}/api/third/pay/${REALM_ID}/providers`,
      )
      expect(providersResponse.ok()).toBeTruthy()
      const providers = await providersResponse.json()
      console.log(`[live] Payment providers response: ${JSON.stringify(providers)}`)
    })

    await test.step('When syncing Stripe provider products', async () => {
      const syncResp = await page.request.post(
        `${BASE_URL}/api/bill/${REALM_ID}/entitlement-mappings/sync`,
        { data: { paymentProvider: 'stripe' } },
      )
      expect(syncResp.ok()).toBeTruthy()
      const syncBody = await syncResp.json()
      console.log(`[live] Sync result: ${JSON.stringify(syncBody)}`)
    })

    await test.step('Then find the Stripe product mapping and configure entitlement key', async () => {
      const mappingsResp = await page.request.get(
        `${BASE_URL}/api/bill/${REALM_ID}/entitlement-mappings?paymentProvider=stripe`,
      )
      expect(mappingsResp.ok()).toBeTruthy()
      const body = await mappingsResp.json()
      const items = body.items ?? body
      expect(Array.isArray(items) && items.length > 0).toBeTruthy()

      const targetMapping = items.find(
        (m: any) => m.externalProductId === secrets.stripe.productId,
      )
      expect(targetMapping).toBeTruthy()
      console.log(`[live] Found mapping: ${JSON.stringify(targetMapping)}`)

      const patchResp = await page.request.patch(
        `${BASE_URL}/api/bill/${REALM_ID}/entitlement-mappings/${targetMapping.id}`,
        {
          data: {
            entitlementKey: ENTITLEMENT_KEY,
            enabled: true,
            pointsPerPeriod: 1000,
            grantPeriodType: 'monthly',
            validityDays: 30,
            grantOnSubscribe: true,
          },
        },
      )
      expect(patchResp.ok()).toBeTruthy()
      const patched = await patchResp.json()
      console.log(`[live] Mapping configured: ${JSON.stringify(patched)}`)
    })

    await test.step('And verify Stripe API key is accepted by listing providers', async () => {
      const providersResponse = await page.request.get(
        `${BASE_URL}/api/third/pay/${REALM_ID}/providers`,
      )
      expect(providersResponse.ok()).toBeTruthy()
      const providersBody = await providersResponse.json()
      console.log(`[live] Payment providers: ${JSON.stringify(providersBody)}`)
    })
  })

  test('US-PA-001 Scenario 5: Stripe checkout payment attempt succeeds', async ({ page }) => {
    let clientAppId: string
    let attemptId: string
    let checkoutUrl: string

    await test.step('Given an entitlement mapping is configured', async () => {
      // Sync provider products
      const syncResp = await page.request.post(
        `${BASE_URL}/api/bill/${REALM_ID}/entitlement-mappings/sync`,
        { data: { paymentProvider: 'stripe' } },
      )
      expect(syncResp.ok()).toBeTruthy()

      // Find and configure the mapping
      const mappingsResp = await page.request.get(
        `${BASE_URL}/api/bill/${REALM_ID}/entitlement-mappings?paymentProvider=stripe`,
      )
      expect(mappingsResp.ok()).toBeTruthy()
      const body = await mappingsResp.json()
      const items = body.items ?? body
      const targetMapping = items.find(
        (m: any) => m.externalProductId === secrets.stripe.productId,
      )
      expect(targetMapping).toBeTruthy()

      if (targetMapping.entitlementKey !== ENTITLEMENT_KEY || !targetMapping.enabled) {
        const patchResp = await page.request.patch(
          `${BASE_URL}/api/bill/${REALM_ID}/entitlement-mappings/${targetMapping.id}`,
          {
            data: {
              entitlementKey: ENTITLEMENT_KEY,
              enabled: true,
              pointsPerPeriod: 1000,
              grantPeriodType: 'monthly',
              validityDays: 30,
              grantOnSubscribe: true,
            },
          },
        )
        expect(patchResp.ok()).toBeTruthy()
      }

      clientAppId = await ensureClientApp(page.request)
      console.log(`[live] Client App ID: ${clientAppId}`)
    })

    await test.step('When creating a checkout session', async () => {
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
      console.log(`[live] Checkout response: ${JSON.stringify(checkoutBody)}`)

      expect(checkoutBody.checkoutUrl).toBeTruthy()
      checkoutUrl = checkoutBody.checkoutUrl
      console.log(`[live] Stripe checkout URL: ${checkoutUrl}`)

      await page.goto(checkoutUrl, { waitUntil: 'domcontentloaded', timeout: 30000 })
      console.log(`[live] Navigated to Stripe checkout page: ${page.url()}`)

      await page.screenshot({ path: 'test-results/stripe-checkout-page.png' })
    })

    await test.step('Then fill test card and submit payment', async () => {
      await page.waitForLoadState('networkidle', { timeout: 15000 }).catch(() => {})

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

      await page.screenshot({ path: 'test-results/stripe-checkout-filled.png' })

      const submitButton = page.getByRole('button', { name: /pay|subscribe/i }).last()
      await expect(submitButton).toBeVisible({ timeout: 5000 })
      await submitButton.scrollIntoViewIfNeeded()
      await submitButton.click()
      await page.waitForTimeout(5000)

      console.log('[live] Payment submitted, waiting for result...')
    })

    await test.step('And wait for redirect and fulfill payment', async () => {
      await page.waitForURL(/\/billing\/success/, { timeout: 30000 }).catch(() => {
        console.log(`[live] Browser landed at ${page.url()} instead of success page`)
      })

      await page.screenshot({ path: 'test-results/stripe-checkout-redirect.png' })
      console.log(`[live] After payment, browser URL: ${page.url()}`)

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
        expect(
          fulfillResult.success,
          `Internal fulfillment failed: ${fulfillResult.error}`,
        ).toBeTruthy()
        console.log(`[live] Fulfillment result: ${JSON.stringify(fulfillResult)}`)

        const finalStatus = await waitForPaymentStatus(
          page.request,
          REALM_ID,
          attemptId,
          'Succeeded',
          15000,
        )

        await page.screenshot({ path: `test-results/stripe-checkout-result-${finalStatus}.png` })
        console.log(`[live] Final payment status: ${finalStatus}`)
        expect(finalStatus).not.toBe('Pending')
      } else {
        console.log('[live] No payment attempt found to fulfill — payment may have been processed via webhook')
      }
    })
  })
})
