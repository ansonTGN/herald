/**
 * One-Time Mapping Purchase Flow Demo (P0)
 *
 * User Stories: US-PU-006
 * Coverage: mapping card selection, WeChat QR payment, Stripe/Creem redirect payment,
 *   page refresh state recovery
 *
 * Uses Demo Seed data (realm-001 with pre-configured one-time entitlement mappings).
 * Per spec/demo/e2e-testing.md Section 8: no admin data creation in tests.
 *
 * NOTE: Payment completion requires webhook simulation by demo infrastructure.
 * Tests verify payment INITIATION and UI states, not fulfillment.
 *
 * - All tests use UnifiedLogger through 'demoLogger' fixture
 */

import { test, expect, cleanupTestData } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { loginWithCredentials } from '../helpers/auth'
import { SELECTORS } from '../selectors'
import {
  initiatePurchaseFlow,
  selectFirstMappingAndProceed,
  selectPaymentMethodAndProceed,
  extractPaymentAttemptId,
  verifyRedirectPromptOrDegraded,
  TEST_DATA,
} from '../helpers/unified-purchase.helpers'

const REALM_ID = TEST_DATA.REALMS.REALM_001
const USER_EMAIL = TEST_DATA.USERS.USER_REALM_001

test.describe('[Regular User] US-PU-006: One-Time Mapping Purchase Flow', () => {
  test.beforeEach(async ({ page }) => {
    await verifyTestEnvironment(page, {
      requiredRealms: [REALM_ID],
      requiredUsers: [USER_EMAIL],
    })

    await loginWithCredentials(page, {
      realmId: REALM_ID,
      email: USER_EMAIL,
      password: TEST_DATA.CREDENTIALS.DEFAULT_PASSWORD,
    })

    await page.waitForURL(`**/${REALM_ID}/user**`)
  })

  test.afterEach(async ({ page, testStartTime }) => {
    await cleanupTestData(page, REALM_ID, {
      keepUsers: [USER_EMAIL],
      timestamp: testStartTime,
    })
  })

  test('should display available one-time mapping cards on purchase page (US-PU-006 S7)', async ({
    page,
    demoLogger,
  }) => {
    await test.step('Navigate to purchase page', async () => {
      await page.goto(`/${REALM_ID}/user/purchase-points`)
      await expect(page.locator(SELECTORS.purchasePoints.page)).toBeVisible()
    })

    await test.step('Verify mapping card selection step is displayed', async () => {
      await expect(page.locator(SELECTORS.purchasePoints.stepPackages)).toBeVisible()
    })

    await test.step('Verify mapping cards grid contains at least one card', async () => {
      await expect(page.locator(SELECTORS.mappingCard.grid)).toBeVisible()

      const firstCard = page.locator(SELECTORS.mappingCard.firstCard()).first()
      await expect(firstCard).toBeVisible()
    })

    await test.step('Verify step indicator shows select step as active', async () => {
      const stepIndicator = page.locator(SELECTORS.purchasePoints.stepIndicator)
      await expect(stepIndicator).toBeVisible()
      // First step (select) should have bold/primary styling
      const selectStep = stepIndicator.locator('span').first()
      await expect(selectStep).toHaveClass(/font-bold|text-primary/)
    })

    await test.step('Verify cards display entitlement key text', async () => {
      const firstCard = page.locator(SELECTORS.mappingCard.firstCard()).first()
      const cardText = await firstCard.textContent()
      expect(cardText).toBeTruthy()
      // Each card should show entitlement key as text content
      expect(cardText!.length).toBeGreaterThan(0)
    })

    await test.step('Verify no empty state is shown (mappings exist from Demo Seed)', async () => {
      const emptyState = page.locator(SELECTORS.mappingCard.emptyState)
      await expect(emptyState).not.toBeVisible()
    })
  })

  test('should initiate WeChat QR payment when selecting mapping and WeChat provider (US-PU-006 S1)', async ({
    page,
    demoLogger,
  }) => {
    await test.step('Clear previous purchase state', async () => {
      await page.evaluate(() => localStorage.removeItem('cas-purchase-flow'))
    })

    await test.step('Navigate to purchase page', async () => {
      await page.goto(`/${REALM_ID}/user/purchase-points`)
      await expect(page.locator(SELECTORS.purchasePoints.page)).toBeVisible()
    })

    await test.step('Select first mapping card and proceed to payment', async () => {
      await selectFirstMappingAndProceed(page)
      await expect(page.locator(SELECTORS.purchasePoints.stepPayment)).toBeVisible()
    })

    await test.step('Select WeChat payment method', async () => {
      await selectPaymentMethodAndProceed(page, TEST_DATA.PAYMENT_PROVIDERS.WECHAT)
    })

    await test.step('Verify processing step is visible', async () => {
      await expect(page.locator(SELECTORS.purchasePoints.stepProcessing)).toBeVisible({
        timeout: TEST_DATA.TIMEOUTS.ELEMENT_VISIBLE,
      })
    })

    await test.step('Verify WeChat QR section rendered', async () => {
      await expect(page.locator(SELECTORS.paymentProviderUI.wechatQrSection)).toBeVisible()
      await expect(page.locator(SELECTORS.paymentProviderUI.wechatQrCode)).toBeVisible()
    })

    await test.step('Verify countdown timer is visible', async () => {
      const countdownTimer = page.locator(SELECTORS.paymentProviderUI.countdownTimer)
      await expect(countdownTimer).toBeVisible()

      const timerText = await countdownTimer.textContent()
      expect(timerText).toMatch(/^\d+:\d{2}$/)
    })

    await test.step('Verify payment attempt ID persisted in localStorage', async () => {
      const attemptId = await extractPaymentAttemptId(page)
      expect(attemptId).toBeTruthy()
    })

    await test.step('Verify cancel button is available', async () => {
      const cancelButton = page.locator(SELECTORS.paymentProviderUI.cancelButton)
      await expect(cancelButton).toBeVisible()
    })
  })

  test('should initiate Stripe redirect payment when selecting mapping and Stripe provider (US-PU-006 S2)', async ({
    page,
    demoLogger,
  }) => {
    await test.step('Clear previous purchase state', async () => {
      await page.evaluate(() => localStorage.removeItem('cas-purchase-flow'))
    })

    await test.step('Navigate to purchase page', async () => {
      await page.goto(`/${REALM_ID}/user/purchase-points`)
      await expect(page.locator(SELECTORS.purchasePoints.page)).toBeVisible()
    })

    await test.step('Select first mapping card and proceed to payment', async () => {
      await selectFirstMappingAndProceed(page)
      await expect(page.locator(SELECTORS.purchasePoints.stepPayment)).toBeVisible()
    })

    await test.step('Select Stripe payment method', async () => {
      await selectPaymentMethodAndProceed(page, TEST_DATA.PAYMENT_PROVIDERS.STRIPE)
    })

    await test.step('Verify processing step is visible', async () => {
      await expect(page.locator(SELECTORS.purchasePoints.stepProcessing)).toBeVisible({
        timeout: TEST_DATA.TIMEOUTS.ELEMENT_VISIBLE,
      })
    })

    await test.step('Verify redirect prompt or degraded UI', async () => {
      // Stripe may show redirect prompt (with checkout URL) or degraded UI (without)
      await verifyRedirectPromptOrDegraded(page, 'Stripe')
    })

    await test.step('Verify payment attempt ID persisted in localStorage', async () => {
      const attemptId = await extractPaymentAttemptId(page)
      expect(attemptId).toBeTruthy()
    })
  })

  test('should recover payment state after page refresh (US-PU-006)', async ({
    page,
    demoLogger,
  }) => {
    let attemptIdBeforeRefresh: string

    await test.step('Initiate WeChat purchase flow via helper', async () => {
      attemptIdBeforeRefresh = await initiatePurchaseFlow(
        page,
        TEST_DATA.PAYMENT_PROVIDERS.WECHAT,
        REALM_ID
      )
      expect(attemptIdBeforeRefresh).toBeTruthy()
    })

    await test.step('Verify processing step visible before refresh', async () => {
      await expect(page.locator(SELECTORS.purchasePoints.stepProcessing)).toBeVisible()
    })

    await test.step('Reload page', async () => {
      await page.reload()
      await page.waitForLoadState('domcontentloaded')
    })

    await test.step('Verify processing step is still visible after refresh', async () => {
      await expect(page.locator(SELECTORS.purchasePoints.stepProcessing)).toBeVisible({
        timeout: TEST_DATA.TIMEOUTS.ELEMENT_VISIBLE,
      })
    })

    await test.step('Verify attempt ID is preserved after refresh', async () => {
      const attemptIdAfterRefresh = await page.evaluate(() => {
        const state = localStorage.getItem('cas-purchase-flow')
        if (state) {
          const parsed = JSON.parse(state)
          return parsed?.state?.attemptId
        }
        return null
      })

      expect(attemptIdAfterRefresh).toBe(attemptIdBeforeRefresh)
    })
  })
})
