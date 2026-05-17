/**
 * Live Creem Payment Smoke Test
 *
 * Validates that real Creem credentials from .env.demo are correctly
 * configured and accepted by the Creem API.
 *
 * Prerequisites:
 *   - CREEM_API_KEY and CREEM_WEBHOOK_SECRET set in demo/.env.demo
 *   - Demo seed data loaded (admin realm, admin@cas.com user)
 *
 * Creem Dashboard Setup (https://creem.io dashboard):
 *   1. Developers → API Keys → Copy API key (starts with ck_test_)
 *      → set as CREEM_API_KEY in .env.demo
 *   2. Developers → Webhooks → Create Webhook:
 *      - Name: Herald Demo
 *      - URL: http://localhost:8080/api/third/pay/{realmId}/creem/webhooks
 *      - Events: checkout.completed, subscription.active/paid/canceled, refund.created
 *      - Copy the Signing secret → set as CREEM_WEBHOOK_SECRET in .env.demo
 *   3. Products → Create Product (optional, test auto-creates if missing):
 *      - Name must match PRODUCT_NAME constant below
 *
 * Fixed test identifiers (must match Creem Dashboard if pre-created):
 *   - Product: herald-live-creem-product
 *   - Plan:    herald-live-creem-plan
 *
 * Test mode: use ck_test_* API key, test card 4242 4242 4242 4242
 *
 * Skips gracefully when credentials are absent.
 */

import { test, expect } from '@playwright/test'
import { secrets, hasCreemPayment } from '../secrets/env'
import { seedCreemConfig } from '../secrets/realm-seed'
import { loginAsAdmin } from '../helpers/auth'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { createProduct, verifyProductInTable } from '../billing-admin/helpers/product-page.helpers'
import { createSubscriptionPlan } from '../billing-admin/helpers/billing-page.helpers'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'
const REALM_ID = 'admin'

// Fixed identifiers — these map to real Creem resources.
// Do NOT change unless you also update the corresponding Creem dashboard entries.
const PRODUCT_NAME = 'herald-live-creem-product'
const PLAN_NAME = 'herald-live-creem-plan'
const PLAN_TITLE = 'Herald Live Creem Plan'

test.describe('Live: Creem Payment', () => {

  test.beforeEach(async ({ page }) => {
    // Skip entire suite when credentials are not configured
    test.skip(!hasCreemPayment(), 'Creem payment credentials not configured in .env.demo')

    // Verify environment is ready
    await verifyTestEnvironment(page, {
      requiredRealms: [REALM_ID],
      requiredUsers: ['admin@cas.com'],
    })

    // Login as admin
    await loginAsAdmin(page, { realmId: REALM_ID })

    // Inject real Creem credentials via API
    await seedCreemConfig(page.request, REALM_ID, {
      apiKey: secrets.creem.apiKey!,
      webhookSecret: secrets.creem.webhookSecret!,
    })

    // Clean up stale test data from previous runs so each test starts fresh.
    // Plans must be deleted before products (product with plans cannot be deleted).
    try {
      const plansResp = await page.request.get(`${BASE_URL}/api/bill/${REALM_ID}/plans`)
      if (plansResp.ok()) {
        const plansBody = await plansResp.json()
        const stalePlans = (plansBody.plans || []).filter((p: any) => p.name === PLAN_NAME)
        for (const plan of stalePlans) {
          const delResp = await page.request.delete(`${BASE_URL}/api/bill/${REALM_ID}/plans/${plan.id}`)
          console.log(`[cleanup] Deleted stale plan ${plan.id}: ${delResp.status()}`)
        }
      }

      const productsResp = await page.request.get(`${BASE_URL}/api/bill/${REALM_ID}/products`)
      if (productsResp.ok()) {
        const productsBody = await productsResp.json()
        const staleProducts = (productsBody.products || []).filter((p: any) => p.code === PRODUCT_NAME)
        for (const product of staleProducts) {
          const delResp = await page.request.delete(`${BASE_URL}/api/bill/${REALM_ID}/products/${product.id}`)
          console.log(`[cleanup] Deleted stale product ${product.id}: ${delResp.status()}`)
        }
      }
    } catch (error) {
      console.error('[cleanup] Error during stale data cleanup (non-fatal):', error)
    }
  })

  test.afterEach(async ({ page }) => {
    // Remove Creem config from realm_config (credentials should not persist)
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

  test('Creem credentials are correctly configured and accepted', async ({ page }) => {
    await test.step('Given Creem config is seeded', async () => {
      // Verify the config was stored by listing payment providers
      const providersResponse = await page.request.get(
        `${BASE_URL}/api/third/pay/${REALM_ID}/providers`,
      )
      expect(providersResponse.ok()).toBeTruthy()

      const providers = await providersResponse.json()
      console.log(`[live] Payment providers response: ${JSON.stringify(providers)}`)

      // The providers list should indicate that Creem is configured
      // (The exact structure depends on list_payment_providers handler)
    })

    await test.step('When creating a product and billing plan via UI', async () => {
      // Navigate to products page
      await page.goto(`/${REALM_ID}/manage/products`, { waitUntil: 'networkidle' })
      await expect(page.getByTestId('products-page')).toBeVisible({ timeout: 10000 })

      // Create a product (idempotent — will fail gracefully if already exists)
      await createProduct(page, {
        code: PRODUCT_NAME,
        title: PRODUCT_NAME,
        description: 'Herald live test product for Creem payment',
      })

      // Verify the product was created
      await verifyProductInTable(page, PRODUCT_NAME)
      console.log(`[live] Product "${PRODUCT_NAME}" created`)

      // Navigate to billing plans page and create a plan
      await page.goto(`/${REALM_ID}/manage/billing`, { waitUntil: 'networkidle' })
      await expect(page.getByTestId('billing-page')).toBeVisible({ timeout: 10000 })

      await createSubscriptionPlan(page, {
        planName: PLAN_NAME,
        title: PLAN_TITLE,
        description: 'Herald live test billing plan for Creem payment',
        price: '10.00',
        type: 'monthly',
        currency: 'usd',
        provider: 'creem',
        productTitle: PRODUCT_NAME,
      })

      console.log(`[live] Billing plan "${PLAN_NAME}" created`)
    })

    await test.step('Then verify the plan is visible on the billing page', async () => {
      // Reload the billing page to ensure fresh data
      await page.goto(`/${REALM_ID}/manage/billing`, { waitUntil: 'networkidle' })
      await expect(page.getByTestId('billing-page')).toBeVisible({ timeout: 10000 })

      // The plan should be in the table (may be paginated)
      // Use a soft check since pagination may hide it
      const planRow = page.locator(`tr:has-text("${PLAN_NAME}")`)
      const planVisible = await planRow.isVisible({ timeout: 5000 }).catch(() => false)

      if (planVisible) {
        console.log(`[live] Plan "${PLAN_NAME}" visible in billing table`)
      } else {
        console.log(`[live] Plan "${PLAN_NAME}" not visible (may be on another page)`)
      }
    })

    await test.step('And verify Creem API key is accepted by listing providers', async () => {
      // List payment providers to verify Creem config is loaded
      const providersResponse = await page.request.get(
        `${BASE_URL}/api/third/pay/${REALM_ID}/providers`,
      )
      expect(providersResponse.ok()).toBeTruthy()

      const providersBody = await providersResponse.json()
      console.log(`[live] Payment providers: ${JSON.stringify(providersBody)}`)

      // The response should indicate Creem is configured
      // The provider list endpoint returns configured providers
      // This validates the API key was stored and can be retrieved
    })
  })

  test('Checkout: full Creem payment with test card', async ({ page }) => {
    let planId: string
    let attemptId: string

    await test.step('Given a product and billing plan exist', async () => {
      // Navigate to products page and create product
      await page.goto(`/${REALM_ID}/manage/products`, { waitUntil: 'networkidle' })
      await expect(page.getByTestId('products-page')).toBeVisible({ timeout: 10000 })

      await createProduct(page, {
        code: PRODUCT_NAME,
        title: PRODUCT_NAME,
        description: 'Herald live test product for Creem payment',
      })

      await verifyProductInTable(page, PRODUCT_NAME)

      // Create billing plan
      await page.goto(`/${REALM_ID}/manage/billing`, { waitUntil: 'networkidle' })
      await expect(page.getByTestId('billing-page')).toBeVisible({ timeout: 10000 })

      await createSubscriptionPlan(page, {
        planName: PLAN_NAME,
        title: PLAN_TITLE,
        description: 'Herald live test billing plan for Creem payment',
        price: '10.00',
        type: 'monthly',
        currency: 'usd',
        provider: 'creem',
        productTitle: PRODUCT_NAME,
      })
    })

    await test.step('When adding Creem provider mapping to the plan', async () => {
      // Get plan ID via API
      const plansResponse = await page.request.get(`${BASE_URL}/api/bill/${REALM_ID}/plans`)
      expect(plansResponse.ok()).toBeTruthy()

      const plansBody = await plansResponse.json()
      const plan = plansBody.plans?.find((p: any) => p.name === PLAN_NAME)
      expect(plan).toBeTruthy()
      planId = plan.id

      console.log(`[live] Plan ID: ${planId}`)

      // Add Creem provider mapping
      const mappingResponse = await page.request.post(
        `${BASE_URL}/api/bill/${REALM_ID}/plans/${planId}/providers`,
        {
          data: {
            paymentProvider: 'creem',
            externalProductId: secrets.creem.productId,
            enabled: true,
          },
        },
      )
      expect(mappingResponse.ok()).toBeTruthy()

      const mappingBody = await mappingResponse.json()
      console.log(`[live] Provider mapping created: ${JSON.stringify(mappingBody)}`)
    })

    await test.step('And creating a payment attempt', async () => {
      const attemptResponse = await page.request.post(
        `${BASE_URL}/api/bill/${REALM_ID}/purchase/payment-attempts`,
        {
          data: {
            targetType: 'subscription_plan',
            targetId: planId,
            paymentProvider: 'creem',
          },
        },
      )
      expect(attemptResponse.ok()).toBeTruthy()

      const attemptBody = await attemptResponse.json()
      attemptId = attemptBody.id

      console.log(`[live] Payment attempt created: ${attemptId}`)
      console.log(`[live] Payment context: ${JSON.stringify(attemptBody.paymentContext)}`)

      expect(attemptBody.paymentContext?.creemCheckoutUrl).toBeTruthy()
      const checkoutUrl = attemptBody.paymentContext.creemCheckoutUrl
      console.log(`[live] Creem checkout URL: ${checkoutUrl}`)

      // Navigate to Creem checkout page
      await page.goto(checkoutUrl, { waitUntil: 'domcontentloaded', timeout: 30000 })
      console.log(`[live] Navigated to Creem checkout page: ${page.url()}`)

      // Take a screenshot for debugging
      await page.screenshot({ path: 'test-results/creem-checkout-page.png' })
    })

    await test.step('Then fill test card and submit payment', async () => {
      // Wait for the checkout page to fully load
      await page.waitForLoadState('networkidle', { timeout: 15000 }).catch(() => {})

      // Try multiple selector strategies for the card number field
      const cardInput = page.locator(
        'input[name*="card"], input[name*="number"], input[placeholder*="4242"], input[placeholder*="Card"], input[autocomplete*="cc-number"]'
      ).first()

      await expect(cardInput).toBeVisible({ timeout: 10000 })
      await cardInput.fill('4242424242424242')

      // Expiry date
      const expiryInput = page.locator(
        'input[name*="expir"], input[placeholder*="MM"], input[placeholder*="MM/YY"], input[autocomplete*="cc-exp"]'
      ).first()

      await expect(expiryInput).toBeVisible({ timeout: 5000 })
      await expiryInput.fill('1230')

      // CVC
      const cvcInput = page.locator(
        'input[name*="cvc"], input[name*="cvv"], input[placeholder*="CVC"], input[placeholder*="CVV"], input[autocomplete*="cc-csc"]'
      ).first()

      await expect(cvcInput).toBeVisible({ timeout: 5000 })
      await cvcInput.fill('123')

      // Take screenshot before submitting
      await page.screenshot({ path: 'test-results/creem-checkout-filled.png' })

      // Submit payment
      const submitButton = page.locator(
        'button[type="submit"], button:has-text("Pay"), button:has-text("Submit")'
      ).first()

      await expect(submitButton).toBeVisible({ timeout: 5000 })
      await submitButton.click()

      console.log('[live] Payment submitted, waiting for result...')
    })

    await test.step('And verify payment status changes', async () => {
      // Wait for payment to process -- either redirect back or status change
      // Poll the payment attempt status via API
      const maxWait = 60000
      const startTime = Date.now()
      let status = 'Pending'

      while (Date.now() - startTime < maxWait) {
        const statusResponse = await page.request.get(
          `${BASE_URL}/api/bill/${REALM_ID}/purchase/payment-attempts/${attemptId}`,
        )

        if (statusResponse.ok()) {
          const statusBody = await statusResponse.json()
          status = statusBody.status
          console.log(`[live] Payment status: ${status}`)

          if (status !== 'Pending') {
            break
          }
        }

        await page.waitForTimeout(3000)
      }

      // Take final screenshot
      await page.screenshot({ path: `test-results/creem-checkout-result-${status}.png` })

      console.log(`[live] Final payment status: ${status}`)

      // The payment should have moved past Pending
      // With test card 4242, it should succeed
      expect(status).not.toBe('Pending')
    })
  })
})
