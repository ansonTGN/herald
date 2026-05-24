/**
 * WeChat QR Code Payment Flow Demo Test
 *
 * User Stories: US-PU-06 Scenario 1, US-WP-005, US-PA-001, US-PA-002
 *
 * Verifies that selecting WeChat as payment provider renders the QR code
 * section with correct UI elements. Does NOT verify payment completion
 * (requires webhook simulation).
 *
 * - All tests use UnifiedLogger through 'demoLogger' fixture
 */

import { test, expect, cleanupTestData } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { loginWithCredentials } from '../helpers/auth'
import { SELECTORS } from '../selectors'
import { initiatePurchaseFlow, TEST_DATA } from '../helpers/unified-purchase.helpers'

const REALM_ID = TEST_DATA.REALMS.REALM_001
const USER_EMAIL = TEST_DATA.USERS.USER_REALM_001

test.describe('[P0] WeChat QR Code Payment Flow', () => {
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

  test('should display WeChat QR code when WeChat payment is initiated', async ({ page, demoLogger }) => {
    await test.step('Initiate WeChat purchase flow', async () => {
      const attemptId = await initiatePurchaseFlow(page, TEST_DATA.PAYMENT_PROVIDERS.WECHAT, REALM_ID)
      expect(attemptId).toBeTruthy()
    })

    await test.step('Verify WeChat QR code section rendered', async () => {
      const qrSection = page.locator(SELECTORS.paymentProviderUI.wechatQrSection)
      await expect(qrSection).toBeVisible()

      await expect(page.locator(SELECTORS.paymentProviderUI.wechatQrCode)).toBeVisible()

      await expect(qrSection.getByText('WeChat Pay')).toBeVisible()
      await expect(qrSection.getByText('Scan with WeChat to pay')).toBeVisible()
    })

    await test.step('Verify countdown timer displayed', async () => {
      const countdownTimer = page.locator(SELECTORS.paymentProviderUI.countdownTimer)
      await expect(countdownTimer).toBeVisible()

      const timerText = await countdownTimer.textContent()
      expect(timerText).toMatch(/^\d+:\d{2}$/)
    })

    await test.step('Verify cancel button available', async () => {
      const cancelButton = page.locator(SELECTORS.paymentProviderUI.cancelButton)
      await expect(cancelButton).toBeVisible()
      await expect(cancelButton).toContainText('Cancel Payment')
    })

    await test.step('Verify scan instructions present', async () => {
      const qrSection = page.locator(SELECTORS.paymentProviderUI.wechatQrSection)
      await expect(qrSection.getByText('Open WeChat on your phone')).toBeVisible()
      await expect(qrSection.getByText('Scan the QR code above')).toBeVisible()
    })
  })
})
