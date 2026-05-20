/**
 * Live WeChat QR Code Payment Flow Test
 *
 * Validates that real WeChat Pay credentials from .env.demo are correctly
 * configured and can produce a scannable QR code for native payment.
 *
 * Prerequisites:
 *   - WECHAT_APP_ID, WECHAT_MCH_ID, WECHAT_V3_KEY, WECHAT_SERIAL_NO,
 *     WECHAT_PRIVATE_KEY set in demo/.env.demo
 *   - WECHAT_NOTIFY_URL set to a publicly reachable HTTPS endpoint
 *     (e.g. an ngrok tunnel) so WeChat can deliver payment webhooks
 *   - Demo seed data loaded (admin realm, test user)
 *   - Backend and frontend running
 *
 * WeChat Merchant Setup (https://pay.weixin.qq.com):
 *   1. API Security → Apply for API certificate → download cert and private key
 *      → set WECHAT_PRIVATE_KEY in .env.demo
 *   2. API Security → Set API v3 key (32 chars) → set WECHAT_V3_KEY in .env.demo
 *   3. API Security → Get certificate serial number → set WECHAT_SERIAL_NO in .env.demo
 *   4. Development config → AppID → set WECHAT_APP_ID in .env.demo
 *   5. Account info → Merchant ID → set WECHAT_MCH_ID in .env.demo
 *
 * Skips gracefully when credentials are absent.
 *
 * User Stories: US-PU-06 Scenario 1, US-WP-005, US-PA-001, US-PA-002
 */

import { test, expect } from '@playwright/test'
import { secrets, requireWechatPayment } from '../secrets/env'
import { seedWechatConfig } from '../secrets/realm-seed'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { loginWithCredentials, loginAsAdmin } from '../helpers/auth'
import { SELECTORS } from '../selectors'
import { initiatePurchaseFlow, TEST_DATA } from '../helpers/unified-purchase.helpers'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'
const REALM_ID = TEST_DATA.REALMS.REALM_001
const USER_EMAIL = TEST_DATA.USERS.USER_REALM_001

test.describe('Live: WeChat QR Code Payment', () => {
  test.beforeEach(async ({ page }) => {
    requireWechatPayment()

    await verifyTestEnvironment(page, {
      requiredRealms: [REALM_ID],
      requiredUsers: [USER_EMAIL],
    })

    // Seed real WeChat credentials via API
    await loginAsAdmin(page, { realmId: REALM_ID })
    await seedWechatConfig(page.request, REALM_ID, {
      appId: secrets.wechat.appId!,
      mchId: secrets.wechat.mchId!,
      v3Key: secrets.wechat.v3Key!,
      serialNo: secrets.wechat.serialNo!,
      privateKey: secrets.wechat.privateKey!,
      notifyUrl: secrets.wechat.notifyUrl || `${BASE_URL}/api/third/pay/${REALM_ID}/wechat/webhooks`,
    })

    // Login as regular user for purchase flow
    await loginWithCredentials(page, {
      realmId: REALM_ID,
      email: USER_EMAIL,
      password: TEST_DATA.CREDENTIALS.DEFAULT_PASSWORD,
    })

    await page.waitForURL(`**/${REALM_ID}/user**`)
  })

  test.afterEach(async ({ page }) => {
    // Clean up WeChat config
    try {
      const resp = await page.request.delete(
        `${BASE_URL}/api/third/pay/${REALM_ID}/providers/wechat`,
      )
      console.log(`[cleanup] WeChat config delete: ${resp.status()}`)
    } catch (error) {
      console.error('[cleanup] Error during WeChat config cleanup:', error)
    }
  })

  test('WeChat credentials produce scannable QR code', async ({ page }) => {
    await test.step('Initiate WeChat purchase flow', async () => {
      const attemptId = await initiatePurchaseFlow(page, TEST_DATA.PAYMENT_PROVIDERS.WECHAT, REALM_ID)
      expect(attemptId).toBeTruthy()
      console.log(`[live] Payment attempt created: ${attemptId}`)
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

      // Cancel to clean up the pending order
      await cancelButton.click()
    })
  })
})
