/**
 * WeChat Pay Configuration Demo Tests
 *
 * User Stories:
 * - docs/user-stories/billing/wechat-pay.md:
 *   - US-WP-005: User scans QR code to pay
 *
 * Design Doc: .ai/design/wechat-pay.md
 *
 * IMPORTANT NOTE:
 * Scenarios 1-4 below test user-facing subscription flow (QR code payment).
 * These are DISABLED because the subscription flow UI does not exist yet.
 *
 * Missing frontend components:
 * - User-facing subscription/browse plans page at `/${realmId}/billing`
 * - Plan cards with payment method buttons
 * - Integration of WechatQrCodePayment component (exists but unused)
 *
 * Currently tested functionality:
 * - WeChat Pay provider configuration (Scenario 5)
 * - Admin can create, view, edit, and delete WeChat Pay configurations
 *
 * Future implementation:
 * When the subscription flow is implemented, Scenarios 1-4 can be re-enabled.
 * The following components already exist and are ready for integration:
 * - WechatQrCodePayment component
 * - CheckoutProviderSelector component
 * - All necessary API endpoints
 *
 * Test Scenarios:
 * 1. ~~Normal QR code payment flow with polling~~ (US-WP-005.1) - DISABLED
 * 2. ~~QR code expiration handling~~ (US-WP-005.2) - DISABLED
 * 3. ~~Payment failure handling~~ (US-WP-005.3) - DISABLED
 * 4. ~~User cancels payment~~ (US-WP-005.4) - DISABLED
 * 5. WeChat Pay provider configuration (ENABLED)
 *
 * UnifiedLogger Usage:
 * - All tests use UnifiedLogger through 'demoLogger' fixture
 * - Logger is automatically initialized and finalized by fixture
 * - Logs are saved to demo/test-results/console-logs/
 */

import { test, cleanupTestData, expect } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { DEMO_ADMIN, DEMO_USERS } from '../helpers/auth'
import { SELECTORS } from '../selectors'
import {
  generateWechatPayTestData,
  createWechatPayConfig,
  deleteWechatPayConfigIfExists,
  assertElementVisibleOrLog,
} from '../helpers/wechat-pay-test.helpers'

test.describe('[Billing Admin] WeChat Pay Configuration', () => {
  let testStartTime: number
  const realmId = DEMO_ADMIN.realmId

  test.beforeEach(async ({ demoLogger }) => {
    testStartTime = Date.now()
    demoLogger.testCode.log('Test started')
  })

  test.afterEach(async ({ page, demoLogger }) => {
    await cleanupTestData(page, realmId, {
      keepUsers: [DEMO_ADMIN.email],
      timestamp: testStartTime,
    })
    await demoLogger.testCode.log('Test data cleaned up')
  })

  // ============================================================================
  // DISABLED SCENARIOS 1-4: User-facing subscription flow
  // ============================================================================
  //
  // The following test scenarios are DISABLED because the user-facing
  // subscription flow UI does not exist yet in the frontend.
  //
  // Missing components:
  // - Route: /${realmId}/billing (user-facing plan browsing page)
  // - Plan cards with payment method selection buttons
  // - Integration of WechatQrCodePayment component (component exists but unused)
  //
  // When the subscription flow is implemented, these scenarios can be re-enabled.
  //
  // Scenario 1: Normal QR code payment flow with polling (US-WP-005.1)
  // Scenario 2: QR code expiration handling (US-WP-005.2)
  // Scenario 3: Payment failure handling (US-WP-005.3)
  // Scenario 4: User cancels payment (US-WP-005.4)
  //
  // See: git history for original test implementation

  test.skip('Scenario 1-4: Subscription flow tests - DISABLED (UI not implemented)', async () => {
    // These tests are kept in git history for future reference
    // They will be re-enabled when the subscription flow UI is implemented
  })

  // ============================================================================
  // Scenario 5: WeChat Pay Provider Configuration (US-WP-005.5)
  // ============================================================================

  test.describe('Scenario 5: WeChat Pay Provider Configuration', () => {
    test('should allow admin to configure WeChat Pay provider', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      await test.step('Given: 已登录管理员', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, DEMO_ADMIN.password, realmId)
      })

      await test.step('Given: 当前 Realm 未配置微信支付', async () => {
        // Ensure WeChat Pay is not configured
        await deleteWechatPayConfigIfExists(page, realmId)
        await demoLogger.testCode.log('WeChat Pay configuration cleaned up')
      })

      await test.step('When: 管理员访问支付提供商配置页面', async () => {
        // Navigate to payment providers page
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        await demoLogger.testCode.log('Navigated to payment providers page')
      })

      await test.step('Then: 显示 "Add WeChat Pay" 按钮', async () => {
        // Verify Add WeChat Pay button is visible
        const addButton = page.getByRole('button', { name: /Add WeChat Pay/i })
          .or(page.locator(SELECTORS.wechatPay.addWechatButton))
        await expect(addButton).toBeVisible()
        await demoLogger.testCode.log('Add WeChat Pay button is visible')
      })

      await test.step('When: 点击 "Add WeChat Pay" 并填写配置', async () => {
        const config = generateWechatPayTestData(testStartTime, realmId)
        await createWechatPayConfig(page, realmId, config)
        await demoLogger.testCode.log('WeChat Pay configuration created')
      })

      await test.step('Then: 显示配置成功消息', async () => {
        // Verify success message is shown (frontend: "WeChat Pay configuration created successfully")
        await expect(page.getByText(/WeChat Pay configuration created successfully/i)).toBeVisible({ timeout: 10000 })
        await demoLogger.testCode.log('Configuration created successfully message displayed')
      })

      await test.step('Then: WeChat Pay 配置卡片显示在列表中', async () => {
        // Verify WeChat Pay config card is displayed
        await expect(page.locator(SELECTORS.wechatPay.configCard)).toBeVisible({ timeout: 5000 })
        await demoLogger.testCode.log('WeChat Pay configuration card is visible')
      })

      await test.step('Then: WeChat Pay 行显示 Edit 和 Delete 按钮', async () => {
        await expect(page.locator(SELECTORS.wechatPay.editConfigButton)).toBeVisible()
        await expect(page.locator(SELECTORS.wechatPay.deleteConfigButton)).toBeVisible()
        await demoLogger.testCode.log('Edit and Delete buttons are visible on the row')
      })
    })

    test('should allow admin to edit WeChat Pay configuration', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      // Generate test data once at the start
      const config = generateWechatPayTestData(testStartTime, realmId)

      await test.step('Given: 已登录管理员', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, DEMO_ADMIN.password, realmId)
      })

      await test.step('Given: 已配置微信支付', async () => {
        // Clean up any existing configuration first
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        await page.waitForLoadState('networkidle')

        const configCard = page.locator(SELECTORS.wechatPay.configCard)
        const isConfigured = await configCard.isVisible().catch(() => false)

        if (isConfigured) {
          await demoLogger.testCode.log('Existing config found, deleting...')
          await page.locator(SELECTORS.wechatPay.deleteConfigButton).click()
          await expect(page.locator('[data-testid="delete-confirm-dialog"]')).toBeVisible()
          await page.locator('[data-testid="delete-confirm-button"]').click()
          await expect(page.getByText(/Payment provider deleted successfully/i)).toBeVisible({ timeout: 10000 })
          await page.waitForTimeout(500)
          await page.reload()
          await page.waitForLoadState('networkidle')
        }

        // Now create new configuration
        await createWechatPayConfig(page, realmId, config)
        await expect(page.getByText(/WeChat Pay configuration created successfully/i)).toBeVisible()
        await demoLogger.testCode.log('WeChat Pay configured')
      })

      await test.step('When: 点击 "Edit" 按钮', async () => {
        // Click edit button
        await page.locator(SELECTORS.wechatPay.editConfigButton).click()
        await demoLogger.testCode.log('Edit button clicked')
      })

      await test.step('Then: 显示编辑页面', async () => {
        // Verify config form page is shown (route-based, not dialog)
        await expect(page.locator(SELECTORS.wechatPay.configFormPage)).toBeVisible()
        await demoLogger.testCode.log('Edit form page is visible')
      })

      await test.step('When: 修改 AppId 并保存', async () => {
        // Update AppId to a new value
        const newAppId = `wx${(testStartTime + 1).toString(16).padStart(16, '0')}`

        // Fill ALL required fields (sensitive fields are empty when edit dialog opens)
        await page.locator(SELECTORS.wechatPay.appIdInput).clear()
        await page.locator(SELECTORS.wechatPay.appIdInput).fill(newAppId)

        await page.locator(SELECTORS.wechatPay.merchantIdInput).clear()
        await page.locator(SELECTORS.wechatPay.merchantIdInput).fill(config.merchantId)

        await page.locator(SELECTORS.wechatPay.serialNoInput).clear()
        await page.locator(SELECTORS.wechatPay.serialNoInput).fill(config.serialNo)

        await page.locator(SELECTORS.wechatPay.v3KeyInput).clear()
        await page.locator(SELECTORS.wechatPay.v3KeyInput).fill(config.v3Key)

        await page.locator(SELECTORS.wechatPay.notifyUrlInput).clear()
        await page.locator(SELECTORS.wechatPay.notifyUrlInput).fill(config.notifyUrl)

        await page.locator(SELECTORS.wechatPay.privateKeyInput).clear()
        await page.locator(SELECTORS.wechatPay.privateKeyInput).fill(config.privateKey)

        // Submit form
        await page.locator(SELECTORS.wechatPay.configSubmitButton).click()
        await demoLogger.testCode.log('All fields filled and form submitted')
      })

      await test.step('Then: 显示更新成功消息', async () => {
        // Verify update success message (frontend: "WeChat Pay configuration updated successfully")
        await expect(page.getByText(/WeChat Pay configuration updated successfully/i)).toBeVisible({ timeout: 10000 })
        await demoLogger.testCode.log('Configuration updated successfully message displayed')
      })

      await test.step('Then: WeChat Pay 行仍显示在列表中', async () => {
        await expect(page.locator(SELECTORS.wechatPay.configCard)).toBeVisible({ timeout: 5000 })
        await demoLogger.testCode.log('WeChat Pay row still visible after edit')
      })
    })

    test('should allow admin to delete WeChat Pay configuration', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      await test.step('Given: 已登录管理员', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, DEMO_ADMIN.password, realmId)
      })

      await test.step('Given: 已配置微信支付', async () => {
        const config = generateWechatPayTestData(testStartTime, realmId)
        await deleteWechatPayConfigIfExists(page, realmId)
        await createWechatPayConfig(page, realmId, config)
        await expect(page.getByText(/WeChat Pay configuration created successfully/i)).toBeVisible()
        await demoLogger.testCode.log('WeChat Pay configured')
      })

      await test.step('When: 点击 "Delete" 按钮', async () => {
        // Click delete button
        await page.locator(SELECTORS.wechatPay.deleteConfigButton).click()
        await demoLogger.testCode.log('Delete button clicked')
      })

      await test.step('Then: 显示确认对话框', async () => {
        // Verify confirmation dialog is shown
        await expect(page.locator('[data-testid="delete-confirm-dialog"]')).toBeVisible()
        await demoLogger.testCode.log('Delete confirmation dialog is visible')
      })

      await test.step('When: 确认删除', async () => {
        // Confirm deletion
        await page.locator('[data-testid="delete-confirm-button"]').click()
        await demoLogger.testCode.log('Deletion confirmed')
      })

      await test.step('Then: 显示删除成功消息', async () => {
        // Verify delete success message
        await expect(page.getByText(/Configuration deleted successfully|deleted/i)).toBeVisible({ timeout: 10000 })
        await demoLogger.testCode.log('Configuration deleted successfully message displayed')
      })

      await test.step('Then: WeChat Pay 配置卡片不再显示', async () => {
        // Verify config card is no longer visible
        await expect(page.locator(SELECTORS.wechatPay.configCard)).toBeHidden({ timeout: 5000 })
        await demoLogger.testCode.log('WeChat Pay configuration card is hidden')
      })

      await test.step('Then: 显示 "Add WeChat Pay" 按钮', async () => {
        // Verify add button is visible again
        const addButton = page.getByRole('button', { name: /Add WeChat Pay/i })
          .or(page.locator(SELECTORS.wechatPay.addWechatButton))
        await expect(addButton).toBeVisible()
        await demoLogger.testCode.log('Add WeChat Pay button is visible again')
      })
    })
  })
})
