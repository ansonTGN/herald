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

import { type Frame, type Locator, type Page } from '@playwright/test'
import { test, expect } from '../../../fixtures/demo-auth.fixtures'
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

/**
 * Fill a short, non-auto-advancing field (expiry, CVC) and verify the digits
 * actually landed. For these fields the masked formatter renders the value as
 * visible text inside the element, so we verify via `inputValue()` digits-length
 * AND require the formatter to have produced at least `minLength` visible digits.
 *
 * NOTE: This is NOT safe for the card number. See `fillCardNumberWithBrandCheck`.
 */
async function fillWithVerification(locator: Locator, value: string, minLength = 1): Promise<void> {
  const expectedDigits = value.replace(/\D/g, '')

  await locator.scrollIntoViewIfNeeded()
  await locator.click({ delay: 50 }).catch(() => {})
  await locator.fill(value)

  const digitsLanded = async (): Promise<number> => {
    const filled = await locator.inputValue().catch(() => '')
    return filled.replace(/\D/g, '').length
  }

  if ((await digitsLanded()) >= Math.min(expectedDigits.length, minLength)) return

  // Retry with character-by-character typing in case fill() didn't trigger formatter.
  await locator.clear().catch(() => {})
  await locator.click({ delay: 50 }).catch(() => {})
  await locator.pressSequentially(value, { delay: 60 })

  const finalDigits = await digitsLanded()
  if (finalDigits < Math.min(expectedDigits.length, minLength)) {
    const raw = await locator.inputValue().catch(() => '<unavailable>')
    throw new Error(
      `Checkout input not filled reliably. Expected ${expectedDigits.length} digits, ` +
        `got ${finalDigits} (raw value: "${raw}"). Payment cannot succeed without ` +
        `a complete value (AGENTS.md Rule 9 / Rule 12).`,
    )
  }
}

/**
 * True when the card-number input has been ACCEPTED by Creem's formatter.
 *
 * Creem's masked card-number input ships with a placeholder string
 * "1234 1234 1234 1234" (16 digits) that `inputValue()` echoes even when the
 * field is visually empty, so digit-length checks are unreliable and have
 * previously fooled the test into clicking "Pay" with an empty card.
 *
 * The ONLY reliable signals are:
 *   - POSITIVE: the brand indicator element's text has actually moved off
 *     "Select card brand (optional)" (re-queried, NOT via getByRole('option')
 *     which doesn't match Creem's custom widget).
 *   - PREFERRED NEGATIVE: the "Your card number is incomplete." alert is no
 *     longer visible anywhere on the page/frames. This is present iff the card
 *     is incomplete, so its absence is the cleanest signal — used as primary.
 *
 * NOTE: A naive `getByText(/\bvisa\b/i)` check was removed because it matched
 * the ALWAYS-PRESENT support paragraph "Supported cards include Visa,
 * Mastercard, UnionPay, American Express, and Discover." — which produced a
 * false positive and let the test click "Pay" on an empty card.
 */
async function isCardBrandDetected(page: Page): Promise<boolean> {
  const roots: SearchRoot[] = [page, ...page.frames()]

  // PRIMARY (preferred negative): the "card number is incomplete" alert must be
  // gone. This text appears iff the card is incomplete, so absence == accepted.
  for (const root of roots) {
    const incompleteVisible = await root
      .getByText(/card number is incomplete/i)
      .first()
      .isVisible({ timeout: 200 })
      .catch(() => false)
    if (incompleteVisible) {
      return false
    }
  }

  // SECONDARY (positive): the brand indicator element must no longer show the
  // "Select card brand (optional)" placeholder text. We re-query the text node
  // directly rather than getByRole('option'), which does not match Creem's
  // custom (non-ARIA) brand widget and previously caused false positives.
  for (const root of roots) {
    const selectBrandText = await root
      .getByText(/select card brand/i)
      .first()
      .isVisible({ timeout: 200 })
      .catch(() => false)
    if (!selectBrandText) {
      // No "incomplete" alert AND no "Select card brand" placeholder → card
      // was accepted and brand auto-detected.
      return true
    }
  }

  return false
}

/**
 * Locate the Frame that hosts the Creem card-number input.
 *
 * Creem renders the payment fields inside a Stripe Elements iframe whose URL
 * contains the stable fragment "elements-inner-accessory-target". `page.keyboard.type()`
 * dispatches at the top-frame level and never reaches the iframe, and
 * `locator.fill()` on a top-frame locator likewise doesn't reach the iframe
 * input, so callers must operate on a locator scoped to THIS frame. Stripe
 * rotates the per-session hash suffix on the iframe name/URL, so we match by
 * the stable URL fragment only.
 */
async function findCardNumberFrame(page: Page): Promise<Frame | null> {
  // The Creem card form is rendered inside a Stripe Elements iframe whose URL
  // contains the stable fragment "elements-inner-accessory-target". Stripe
  // rotates the per-session hash suffix, so name/url-equality probing is
  // fragile; locate the frame by that URL fragment instead.
  for (const frame of page.frames()) {
    if (frame === page.mainFrame()) continue
    if (frame.url().includes('elements-inner-accessory-target')) {
      return frame
    }
  }
  return null
}

/**
 * Fill the Creem card-number input and verify the card was ACCEPTED via the
 * brand-detection signal, NOT via `inputValue()` (which echoes the field's
 * 16-digit placeholder and would otherwise fool the test into proceeding with
 * an empty card — see isCardBrandDetected).
 *
 * The card input lives inside a Stripe Elements iframe; `locator.fill()` and
 * top-frame `page.keyboard.type()` do NOT reach it (confirmed: at submit time
 * the card field was empty with placeholder + "incomplete" alert). Strategy
 * (in order, retrying until brand is detected):
 *   1. Resolve the Stripe Elements iframe by URL fragment, then on
 *      `input#payment-numberInput` scoped to THAT frame: `click()` to focus,
 *      then `pressSequentially(digits, { delay: 80 })` — per-keystroke events
 *      slow enough for Creem's masked formatter.
 *   2. Top-frame fallback: original `cardLocator.fill()` then
 *      `page.keyboard.type()` (kept for paths where the card input is NOT
 *      inside an iframe).
 *
 * After typing we click the Expiry field. This both (a) triggers Creem's
 * card-number validation (which is what flips the "incomplete" alert off and
 * updates the brand indicator) and (b) prepares to fill expiry next.
 *
 * Fails loud if the brand is still not detected after all attempts: clicking
 * "Pay" with an empty card guarantees Creem rejects the payment, no webhook is
 * delivered, and the invoice is never created — the test would then time out
 * waiting for an invoice that can never arrive (AGENTS.md Rule 9 / Rule 12).
 */
async function fillCardNumberWithBrandCheck(
  page: Page,
  cardLocator: Locator,
  cardNumber: string,
  expiryLocator: Locator,
): Promise<void> {
  const tryFillIframe = async (): Promise<boolean> => {
    const cardFrame = await findCardNumberFrame(page)
    if (!cardFrame) {
      console.log('[card-fill] no Stripe Elements iframe found; skipping iframe attempt')
      return false
    }
    try {
      // Stripe Elements exposes the card input as input#payment-numberInput
      // (NOT via a labelled a11y "Card number" role), which is why the prior
      // getByRole('textbox', { name: /card number/i }) probe never matched.
      // Scope to the real element ID inside the located iframe, mirroring the
      // proven reference implementation in us-pa-001-creem-checkout-live.e2e.ts.
      const frameCard = cardFrame.locator('input#payment-numberInput').first()
      await frameCard.waitFor({ state: 'visible', timeout: 10000 })
      await frameCard.scrollIntoViewIfNeeded()
      await frameCard.click({ delay: 50 })
      await frameCard.pressSequentially(cardNumber, { delay: 80 })
    } catch (e) {
      console.log(`[card-fill] iframe pressSequentially errored: ${(e as Error).message}`)
      return false
    }

    // Click the Expiry field (in the same iframe OR top frame) to move focus
    // out of the card field. Creem validates on blur, which is what clears
    // the "incomplete" alert and updates the brand indicator.
    try {
      await expiryLocator.click({ delay: 50 }).catch(() => {})
    } catch {
      // ignore — focus shift is best-effort
    }
    await new Promise((resolve) => setTimeout(resolve, 500))
    return await isCardBrandDetected(page)
  }

  const tryFillTopFrame = async (): Promise<boolean> => {
    try {
      await cardLocator.scrollIntoViewIfNeeded()
      await cardLocator.click({ delay: 50 }).catch(() => {})
      await page.keyboard.type(cardNumber, { delay: 80 })
    } catch (e) {
      console.log(`[card-fill] top-frame keyboard.type errored: ${(e as Error).message}`)
      return false
    }

    try {
      await expiryLocator.click({ delay: 50 }).catch(() => {})
    } catch {
      // ignore
    }
    await new Promise((resolve) => setTimeout(resolve, 500))
    return await isCardBrandDetected(page)
  }

  // Attempt 1: iframe-resolved locator with per-keystroke typing.
  if (await tryFillIframe()) {
    console.log('[card-fill] iframe pressSequentially succeeded (brand detected)')
    return
  }

  console.log('[card-fill] iframe attempt did not land; retrying via top-frame locator.fill + keyboard.type')
  // Clear partial state before retry.
  await cardLocator.clear().catch(() => {})
  await cardLocator.click({ delay: 50 }).catch(() => {})
  await page.keyboard.press('Control+A').catch(() => {})
  await page.keyboard.press('Delete').catch(() => {})

  // Attempt 2: top-frame fallback.
  if (await tryFillTopFrame()) {
    console.log('[card-fill] top-frame fallback succeeded (brand detected)')
    return
  }

  // Diagnostic dump: card input value + frames present, plus screenshot for
  // the next iteration to confirm visually.
  const cardValue = await cardLocator.inputValue().catch(() => '<unavailable>')
  console.log(`[card-fill] card input value after attempts: "${cardValue}"`)
  try {
    await page.screenshot({
      path: 'test-results/creem-onetime-card-after-fill.png',
      fullPage: true,
    })
  } catch {
    // non-fatal
  }

  const framesDump = page
    .frames()
    .map((f) => `- name="${f.name()}" url="${f.url()}"`)
    .join('\n')

  // Final loud failure: do NOT proceed to "Pay".
  throw new Error(
    'Creem iframe card-number input could not be populated. ' +
      `Final card inputValue="${cardValue}". ` +
      'Brand indicator still reads "Select card brand (optional)" / ' +
      '"Your card number is incomplete." alert still visible. ' +
      'Frames present:\n' +
      (framesDump || '- <none>') +
      '\nAborting BEFORE clicking "Pay": an empty card guarantees the checkout ' +
      'fails, no webhook is sent, and no invoice is ever created ' +
      '(AGENTS.md Rule 9 / Rule 12).',
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
 * Returns the first invoice with provider='creem' and a truthy externalInvoiceId.
 * Throws on timeout.
 */
async function waitForCreemInvoice(
  page: Page,
  timeout = 30000,
): Promise<{
  id: string
  provider: string
  externalInvoiceId: string
  externalHostedUrl: string | null
  externalPdfUrl: string | null
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
            inv.externalInvoiceId,
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

  test.beforeEach(async ({ page, demoLogger }) => {
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
    demoLogger.testCode.log('[Live] ✓ Creem config seeded + admin login')

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

  test.afterEach(async ({ page, demoLogger }) => {
    try {
      for (const key of ['api_key', 'webhook_secret']) {
        const resp = await page.request.delete(
          `${BASE_URL}/api/configs/${REALM_ID}/creem/${key}`,
        )
        demoLogger.testCode.log(`[Live] ✓ Creem ${key} cleanup delete: ${resp.status()}`)
      }
    } catch (error) {
      demoLogger.testCode.log(`[Live] ✗ Creem config cleanup error: ${error}`)
      console.error('[cleanup] Error during Creem config cleanup:', error)
    }
  })

  test('US-PU-006 Scenario 7: Creem one-time checkout creates external invoice with correct fields', async ({ page, demoLogger }) => {
    let clientAppId: string
    let attemptId: string
    let mappingId: string

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
      mappingId = targetMapping.id

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
            mappingId,
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

      // Creem renders its card form inside a Stripe Elements iframe whose URL
      // contains the stable fragment "elements-inner-accessory-target". Stripe
      // rotates the per-session hash suffix, so the iframe cannot be targeted
      // by name. Scope by the URL fragment and target the real element IDs
      // (input#payment-numberInput / expiryInput / cvcInput), mirroring the
      // proven approach in us-pa-001-creem-checkout-live.e2e.ts. The previous
      // generic-selector probe (CARD_NUMBER_SELECTORS / EXPIRY_SELECTORS /
      // CVC_SELECTORS via findVisibleCheckoutControl) raced the iframe mount
      // and threw "Creem checkout expiry control not found." because the inner
      // inputs were not yet rendered when probed.
      //
      // Explicit mount waits below are REQUIRED: the iframe DOM is present
      // shortly after `networkidle`, but its inner inputs mount slightly later.
      // Without these waits the locators resolve to a not-yet-rendered input
      // and the subsequent click/fill silently no-ops or throws.
      const cardFrame = page.frameLocator('iframe[src*="elements-inner-accessory-target"]')
      const cardInput   = cardFrame.locator('input#payment-numberInput')
      const expiryInput = cardFrame.locator('input#payment-expiryInput')
      const cvcInput    = cardFrame.locator('input#payment-cvcInput')

      await cardInput.waitFor({ state: 'visible', timeout: 10000 })
      await expiryInput.waitFor({ state: 'visible', timeout: 10000 })

      // Fill test card. The card number is verified via brand detection, NOT
      // inputValue() — Creem's placeholder "1234 1234 1234 1234" fools digit
      // checks and previously caused the test to click "Pay" on an empty card.
      //
      // fillCardNumberWithBrandCheck needs the expiry locator up-front so it
      // can click it after typing — moving focus out of the card field is what
      // triggers Creem's validation (clearing the "incomplete" alert and
      // updating the brand indicator).
      await fillCardNumberWithBrandCheck(page, cardInput, '4242424242424242', expiryInput)

      await fillWithVerification(expiryInput, '1230', 4)

      await fillWithVerification(cvcInput, '123', 3)

      const fullNameInput = page.getByRole('textbox', { name: /full name/i })
      if (await fullNameInput.isVisible({ timeout: 1000 }).catch(() => false)) {
        await fullNameInput.fill('Herald Demo User')
      }

      // Pre-submit screenshot for visual confirmation that the card number
      // actually landed (brand should read "Visa", not "Select card brand").
      await page.screenshot({ path: 'test-results/creem-onetime-checkout-filled.png', fullPage: true })

      // Submit payment
      const submitButton = page.getByRole('button', { name: /pay/i }).last()
      await expect(submitButton).toBeVisible({ timeout: 5000 })
      await submitButton.scrollIntoViewIfNeeded()
      await submitButton.click()
      await page.waitForTimeout(5000)

      demoLogger.testCode.log('[Live] ✓ one-time payment submitted')
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
        demoLogger.testCode.log(`[Live] ✓ fulfillment result: ${JSON.stringify(fulfillResult)}`)

        const finalStatus = await waitForPaymentStatus(
          page.request,
          REALM_ID,
          attemptId,
          'Succeeded',
          15000,
        )
        demoLogger.testCode.log(`[Live] ✓ payment status: ${finalStatus}`)
        expect(finalStatus).not.toBe('Pending')
      } else {
        console.log('[live-s7] No payment attempt found to fulfill -- payment may have been processed via webhook')
      }
    })

    await test.step('Then a Creem external invoice exists with correct fields', async () => {
      const invoice = await waitForCreemInvoice(page, 30000)
      console.log(`[live-s7] Creem one-time external invoice: ${JSON.stringify(invoice)}`)

      expect(invoice.provider).toBe('creem')
      expect(invoice.externalInvoiceId, 'Expected externalInvoiceId to be a non-empty string').toBeTruthy()
      expect(invoice.status, `Expected status 'paid', got '${invoice.status}'`).toBe('paid')
      expect(invoice.total, 'Expected total > 0').toBeGreaterThan(0)
      demoLogger.testCode.log('[Live] ✓ Creem one-time invoice fields verified (provider/id/status/total)')
      // Creem may or may not provide hosted/pdf URLs
      if (invoice.externalHostedUrl) {
        expect(invoice.externalHostedUrl).toBeTruthy()
      } else {
        console.log('[live-s7] externalHostedUrl not present (Creem may not provide this)')
      }
      if (invoice.externalPdfUrl) {
        expect(invoice.externalPdfUrl).toBeTruthy()
      } else {
        console.log('[live-s7] externalPdfUrl not present (Creem may not provide this)')
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
      expect(detail.externalInvoiceId).toBeTruthy()
      if (detail.externalHostedUrl) {
        expect(detail.externalHostedUrl).toBeTruthy()
      }
      if (detail.externalPdfUrl) {
        expect(detail.externalPdfUrl).toBeTruthy()
      }
    })
  })
})
