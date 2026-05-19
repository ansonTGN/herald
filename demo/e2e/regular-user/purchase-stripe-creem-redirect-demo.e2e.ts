/**
 * Stripe/Creem Redirect Payment Flow Demo Test
 *
 * User Stories: US-PU-06 Scenario 2, US-PA-001, US-PA-002
 *
 * Verifies that selecting Stripe or Creem as payment provider renders the
 * redirect prompt with manual link. Handles both cases: checkout URL
 * present (redirect prompt) and absent (degraded UI).
 *
 * - All tests use UnifiedLogger through 'demoLogger' fixture
 */

import { test, expect, cleanupTestData } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { loginWithCredentials } from '../helpers/auth'
import { initiatePurchaseFlow, verifyRedirectPromptOrDegraded, TEST_DATA } from '../helpers/unified-purchase.helpers'

const REALM_ID = TEST_DATA.REALMS.REALM_001
const USER_EMAIL = TEST_DATA.USERS.USER_REALM_001

test.describe('[P0] Stripe/Creem Redirect Payment Flow', () => {
  let testStartTime: number

  test.beforeEach(async ({ page }) => {
    testStartTime = Date.now()

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

  test.afterEach(async ({ page }) => {
    await cleanupTestData(page, REALM_ID, {
      keepUsers: [USER_EMAIL],
      timestamp: testStartTime,
    })
  })

  test('should display redirect prompt for Stripe and Creem payments', async ({ page, demoLogger }) => {
    await test.step('Initiate Stripe purchase flow', async () => {
      const attemptId = await initiatePurchaseFlow(page, TEST_DATA.PAYMENT_PROVIDERS.STRIPE, REALM_ID)
      expect(attemptId).toBeTruthy()
    })

    await test.step('Verify Stripe redirect prompt or degraded UI', async () => {
      await verifyRedirectPromptOrDegraded(page, 'Stripe')
    })

    await test.step('Initiate Creem purchase flow', async () => {
      const attemptId = await initiatePurchaseFlow(page, TEST_DATA.PAYMENT_PROVIDERS.CREEM, REALM_ID)
      expect(attemptId).toBeTruthy()
    })

    await test.step('Verify Creem redirect prompt or degraded UI', async () => {
      await verifyRedirectPromptOrDegraded(page, 'Creem')
    })
  })
})
