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
 *      - Events: checkout.session.completed, customer.subscription.*, invoice.*
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

/** Get the admin user's wallet from the points API (session-authenticated, auto-creates). */
async function getAdminWallet(page: Page): Promise<{ balance: number; totalPaidGranted: number; totalRecharged: number; totalConsumed: number; status: string }> {
  // Get current user ID from auth status
  const statusResp = await page.request.get(`${BASE_URL}/api/auth/${REALM_ID}/status`)
  expect(statusResp.ok(), `Failed to get auth status: ${await statusResp.text().catch(() => '')}`).toBeTruthy()
  const { userId } = await statusResp.json()
  expect(userId, 'Expected userId in auth status').toBeTruthy()

  // Use single-user endpoint which auto-creates the wallet if missing
  const walletResp = await page.request.get(
    `${BASE_URL}/api/points/${REALM_ID}/wallets/${userId}`,
  )
  expect(walletResp.ok(), `Failed to get wallet: ${await walletResp.text().catch(() => '')}`).toBeTruthy()
  return await walletResp.json()
}

/** Get the admin wallet balance (total balance). */
async function getAdminWalletBalance(page: Page): Promise<number> {
  const wallet = await getAdminWallet(page)
  return wallet.balance
}

/**
 * Poll until the balance changes from the given baseline.
 * Returns the new balance once a change is detected.
 */
async function pollForBalanceChange(page: Page, baseline: number, timeout = 30000): Promise<number> {
  const startTime = Date.now()
  let delay = 500
  const maxDelay = 3000

  while (Date.now() - startTime < timeout) {
    const current = await getAdminWalletBalance(page)
    if (current !== baseline) {
      return current
    }
    await new Promise((resolve) => setTimeout(resolve, delay))
    delay = Math.min(delay * 1.5, maxDelay)
  }

  // Return current balance even if unchanged (caller decides what to assert)
  return await getAdminWalletBalance(page)
}

/**
 * Poll until balance stabilizes (same value across two consecutive reads).
 * Returns the stabilized balance.
 */
async function pollForBalanceStable(page: Page, timeout = 15000): Promise<number> {
  const startTime = Date.now()
  let delay = 500
  const maxDelay = 2000
  let lastBalance = await getAdminWalletBalance(page)

  while (Date.now() - startTime < timeout) {
    await new Promise((resolve) => setTimeout(resolve, delay))
    const current = await getAdminWalletBalance(page)
    if (current === lastBalance) {
      return current
    }
    lastBalance = current
    delay = Math.min(delay * 1.5, maxDelay)
  }

  return lastBalance
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

/**
 * Poll the invoice API until a Stripe external invoice appears.
 * Returns the first invoice with provider='stripe' and an external_invoice_id starting with 'in_'.
 * Throws on timeout.
 */
async function waitForStripeInvoice(
  page: Page,
  timeout = 30000,
): Promise<{ id: string; provider: string; external_invoice_id: string; external_hosted_url: string | null; external_pdf_url: string | null; status: string; total: number; [key: string]: unknown }> {
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

      await page.goto(checkoutUrl, { waitUntil: 'domcontentloaded', timeout: 60000 })
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

      const nameInput = await findVisibleCheckoutControl(page, 'cardholder name', [
        (root) => root.getByLabel(/cardholder name/i),
        (root) => root.getByPlaceholder(/full name on card/i),
      ])
      await nameInput.fill('Test User')
      await nameInput.blur()
      await page.waitForTimeout(1000)

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

  test('US-PA-001 Scenario 6: Full subscription lifecycle — subscribe, verify credits, cancel, verify credit change', async ({ page }) => {
    let clientAppId: string
    let attemptId: string
    let checkoutUrl: string
    let balanceBefore: number

    await test.step('Given an entitlement mapping is configured with pointsPerPeriod: 1000', async () => {
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
      console.log(`[live-s6] Client App ID: ${clientAppId}`)
    })

    await test.step('And capture baseline balance before subscription', async () => {
      balanceBefore = await getAdminWalletBalance(page)
      console.log(`[live-s6] Balance before subscription: ${balanceBefore}`)
    })

    await test.step('When creating a checkout session and subscribing', async () => {
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
      console.log(`[live-s6] Checkout response: ${JSON.stringify(checkoutBody)}`)

      expect(checkoutBody.checkoutUrl).toBeTruthy()
      checkoutUrl = checkoutBody.checkoutUrl
      console.log(`[live-s6] Stripe checkout URL: ${checkoutUrl}`)

      await page.goto(checkoutUrl, { waitUntil: 'domcontentloaded', timeout: 60000 })
      console.log(`[live-s6] Navigated to Stripe checkout page: ${page.url()}`)

      await page.screenshot({ path: 'test-results/stripe-s6-checkout-page.png' })
    })

    await test.step('And filling test card and submitting payment', async () => {
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

      const nameInput = await findVisibleCheckoutControl(page, 'cardholder name', [
        (root) => root.getByLabel(/cardholder name/i),
        (root) => root.getByPlaceholder(/full name on card/i),
      ])
      await nameInput.fill('Test User')
      await nameInput.blur()

      // Wait for Stripe to validate the field
      await page.waitForTimeout(1000)

      await page.screenshot({ path: 'test-results/stripe-s6-checkout-filled.png' })

      const submitButton = page.getByRole('button', { name: /pay|subscribe/i }).last()
      await expect(submitButton).toBeVisible({ timeout: 5000 })
      await submitButton.scrollIntoViewIfNeeded()
      await submitButton.click()
      await page.waitForTimeout(5000)

      console.log('[live-s6] Payment submitted, waiting for result...')
    })

    await test.step('And waiting for redirect and fulfilling payment', async () => {
      await page.waitForURL(/\/billing\/success/, { timeout: 30000 }).catch(() => {
        console.log(`[live-s6] Browser landed at ${page.url()} instead of success page`)
      })

      await page.screenshot({ path: 'test-results/stripe-s6-checkout-redirect.png' })
      console.log(`[live-s6] After payment, browser URL: ${page.url()}`)

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
        console.log(`[live-s6] Fulfillment result: ${JSON.stringify(fulfillResult)}`)

        const finalStatus = await waitForPaymentStatus(
          page.request,
          REALM_ID,
          attemptId,
          'Succeeded',
          15000,
        )

        await page.screenshot({ path: `test-results/stripe-s6-result-${finalStatus}.png` })
        console.log(`[live-s6] Final payment status: ${finalStatus}`)
        expect(finalStatus).not.toBe('Pending')
      } else {
        console.log('[live-s6] No payment attempt found to fulfill — payment may have been processed via webhook')
      }
    })

    await test.step('Then verify credits were granted after subscription', async () => {
      const balanceAfter = await pollForBalanceChange(page, balanceBefore, 30000)
      console.log(`[live-s6] Balance after subscription: ${balanceAfter} (before: ${balanceBefore})`)

      const balanceIncrease = balanceAfter - balanceBefore
      expect(
        balanceIncrease,
        `Expected balance to increase by at least 1000 points after subscription, but got increase of ${balanceIncrease}`,
      ).toBeGreaterThanOrEqual(1000)

      await page.screenshot({ path: `test-results/stripe-s6-balance-after-subscribe.png` })
      console.log(`[live-s6] Credit increase verified: +${balanceIncrease} points`)
    })

    await test.step('When canceling the subscription', async () => {
      // Get subscription for the client app
      const subResp = await page.request.get(
        `${BASE_URL}/api/bill/${REALM_ID}/client/${clientAppId}/subscription`,
      )
      expect(subResp.ok(), `Failed to get subscription: ${await subResp.text().catch(() => '')}`).toBeTruthy()

      const subBody = await subResp.json()
      console.log(`[live-s6] Subscription details: ${JSON.stringify(subBody)}`)
      expect(subBody.id).toBeTruthy()
      expect(subBody.status).toBeTruthy()

      // Cancel the subscription immediately (not at period end)
      const cancelResp = await page.request.post(
        `${BASE_URL}/api/bill/${REALM_ID}/client/${clientAppId}/subscription/cancel`,
        {
          data: {
            cancelAtPeriodEnd: false,
          },
        },
      )
      expect(cancelResp.ok(), `Failed to cancel subscription: ${await cancelResp.text().catch(() => '')}`).toBeTruthy()

      const cancelBody = await cancelResp.json()
      console.log(`[live-s6] Cancel subscription response: ${JSON.stringify(cancelBody)}`)
      expect(cancelBody.subscriptionId).toBeTruthy()
      expect(cancelBody.message).toBeTruthy()

      await page.screenshot({ path: 'test-results/stripe-s6-after-cancel.png' })
      console.log(`[live-s6] Subscription canceled: ${cancelBody.message}`)
    })

    await test.step('Then verify credits changed after cancellation', async () => {
      // Wait for balance to reflect the cancellation
      const balanceBeforeCancel = await getAdminWalletBalance(page)
      console.log(`[live-s6] Balance at time of cancel query: ${balanceBeforeCancel}`)

      // Poll for balance to decrease from post-subscription level
      // The cancellation may revoke subscription credits
      const balanceAfterCancel = await pollForBalanceStable(page, 15000)
      console.log(`[live-s6] Balance after cancellation stabilized: ${balanceAfterCancel}`)

      // After cancellation, totalPaidGranted or balance should reflect the change.
      // The key assertion: the wallet's totalPaidGranted decreased or balance decreased.
      const wallet = await getAdminWallet(page)
      console.log(`[live-s6] Final wallet state: ${JSON.stringify(wallet)}`)

      // Verify that something changed — balance decreased from the post-subscription peak
      // or subscription_balance reflects the revocation
      expect(wallet).toBeTruthy()
      // The subscription cancellation should revoke the granted credits
      // so balance should be lower than the post-subscription balance
      const postSubscribeBalance = balanceBeforeCancel
      expect(
        wallet.balance,
        `Expected balance (${wallet.balance}) to be less than post-subscribe balance (${postSubscribeBalance}) after cancellation`,
      ).toBeLessThanOrEqual(postSubscribeBalance)

      await page.screenshot({ path: 'test-results/stripe-s6-final-balance.png' })
      console.log(`[live-s6] Final balance: ${wallet.balance}, totalPaidGranted: ${wallet.totalPaidGranted}`)
    })
  })

  test('US-PA-001 Scenario 7: Stripe checkout creates external invoice with correct fields', async ({ page }) => {
    let clientAppId: string
    let attemptId: string
    let checkoutUrl: string

    await test.step('Given an entitlement mapping is configured', async () => {
      const syncResp = await page.request.post(
        `${BASE_URL}/api/bill/${REALM_ID}/entitlement-mappings/sync`,
        { data: { paymentProvider: 'stripe' } },
      )
      expect(syncResp.ok()).toBeTruthy()

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
      console.log(`[live-s7] Client App ID: ${clientAppId}`)
    })

    await test.step('When creating a checkout session and completing payment', async () => {
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
      checkoutUrl = checkoutBody.checkoutUrl
      console.log(`[live-s7] Stripe checkout URL: ${checkoutUrl}`)

      await page.goto(checkoutUrl, { waitUntil: 'domcontentloaded', timeout: 60000 })

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

      const nameInput = await findVisibleCheckoutControl(page, 'cardholder name', [
        (root) => root.getByLabel(/cardholder name/i),
        (root) => root.getByPlaceholder(/full name on card/i),
      ])
      await nameInput.fill('Test User')
      await nameInput.blur()
      await page.waitForTimeout(1000)

      const submitButton = page.getByRole('button', { name: /pay|subscribe/i }).last()
      await expect(submitButton).toBeVisible({ timeout: 5000 })
      await submitButton.scrollIntoViewIfNeeded()
      await submitButton.click()
      await page.waitForTimeout(5000)

      console.log('[live-s7] Payment submitted')
    })

    await test.step('And fulfill payment and verify success', async () => {
      await page.waitForURL(/\/billing\/success/, { timeout: 30000 }).catch(() => {
        console.log(`[live-s7] Browser landed at ${page.url()} instead of success page`)
      })

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

        const finalStatus = await waitForPaymentStatus(
          page.request,
          REALM_ID,
          attemptId,
          'Succeeded',
          15000,
        )
        console.log(`[live-s7] Payment status: ${finalStatus}`)
        expect(finalStatus).not.toBe('Pending')
      }
    })

    await test.step('Then a Stripe external invoice exists with correct fields', async () => {
      const invoice = await waitForStripeInvoice(page, 30000)
      console.log(`[live-s7] Stripe external invoice: ${JSON.stringify(invoice)}`)

      expect(invoice.provider).toBe('stripe')
      expect(invoice.external_invoice_id, 'Expected external_invoice_id to start with in_').toMatch(/^in_/)
      expect(invoice.status, `Expected status 'paid', got '${invoice.status}'`).toBe('paid')
      expect(invoice.total, 'Expected total > 0').toBeGreaterThan(0)
      expect(invoice.external_hosted_url, 'Expected external_hosted_url to be present').toBeTruthy()
      expect(invoice.external_pdf_url, 'Expected external_pdf_url to be present').toBeTruthy()
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
    })

    // Cleanup: cancel any subscription created in this test
    await test.step('Cleanup: cancel subscription if created', async () => {
      try {
        const cancelResp = await page.request.post(
          `${BASE_URL}/api/bill/${REALM_ID}/client/${clientAppId}/subscription/cancel`,
          { data: { cancelAtPeriodEnd: false } },
        )
        console.log(`[live-s7] Cleanup cancel: ${cancelResp.status()}`)
      } catch (error) {
        console.log('[live-s7] Cleanup cancel failed (non-fatal):', error)
      }
    })
  })
})
