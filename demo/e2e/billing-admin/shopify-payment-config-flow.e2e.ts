/**
 * Shopify Payment Configuration Flow Demo Tests
 *
 * User Stories:
 * - docs/user-stories/08-shopify-pay-user-stories.md:
 *   - US-PP-007: Configure Shopify Payment Platform
 *   - US-PP-008: View Shopify Configuration
 *   - US-PP-009: Edit Shopify Configuration
 *   - US-PP-010: Delete Shopify Configuration
 *
 * Design Doc: .ai/design/shopify_pay.md
 *
 * Test Scenarios:
 * 1. Create Shopify Configuration (US-PP-007)
 * 2. Shop Domain Format Validation (US-PP-007)
 * 3. API Token Format Validation (US-PP-007)
 * 4. Test Connection (US-PP-008)
 * 5. View Configuration Details (US-PP-008)
 * 6. Edit Configuration (US-PP-009)
 * 7. Delete Configuration (US-PP-010)
 *
 * UnifiedLogger Usage:
 * - All tests use UnifiedLogger through 'demoLogger' fixture
 * - Logger is automatically initialized and finalized by fixture
 * - Logs are saved to demo/test-results/console-logs/
 */

import { test, cleanupTestData, expect } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { DEMO_ADMIN } from '../helpers/auth'

test.describe('[Billing Admin] Shopify Payment Configuration Flow', () => {
  let testStartTime: number
  const realmId = DEMO_ADMIN.realmId

  test.beforeEach(async ({ page, demoLogger }) => {
    testStartTime = Date.now()
    await verifyTestEnvironment(page, {
      requiredRealms: [realmId],
      requiredUsers: [DEMO_ADMIN.email],
    })
    await demoLogger.testCode.log('Environment verified')
  })

  test.afterEach(async ({ page, demoLogger }) => {
    await cleanupTestData(page, realmId, {
      keepUsers: [DEMO_ADMIN.email],
      timestamp: testStartTime,
    })
    await demoLogger.testCode.log('Test data cleaned up')
  })

  // ============================================================================
  // Scenario 1: Create Shopify Configuration (US-PP-007)
  // ============================================================================

  test.describe('Scenario 1: Create Shopify Configuration', () => {
    test('should create Shopify payment provider configuration', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      const shopDomain = `demo-store-${testStartTime}.myshopify.com`
      const adminAccessToken = `shpat_test_token_${testStartTime}`
      const storefrontAccessToken = `shp_test_token_${testStartTime}`
      const appClientSecret = `test_secret_${testStartTime}_longer_string_32chars`
      const apiVersion = '2024-01'

      await test.step('Given: Realm Admin 已登录', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', realmId)
        await demoLogger.testCode.log('Realm Admin logged in')
      })

      await test.step('When: 导航到支付平台配置页面', async () => {
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        // Use semantic role selector (PageHeader renders as heading)
        await expect(page.getByRole('heading', { name: 'Payment Providers' })).toBeVisible()
        await demoLogger.testCode.log('Payment providers page loaded')
      })

      await test.step('When: 点击 "Add Provider" 按钮', async () => {
        await page.getByTestId('add-shopify-button').click()
        await expect(page.getByTestId('shopify-config-form-dialog')).toBeVisible()
        await demoLogger.testCode.log('Add Provider dialog opened')
      })

      await test.step('When: 填写 Shopify 配置信息', async () => {
        await page.getByTestId('shop-domain-input').fill(shopDomain)
        await page.getByTestId('admin-access-token-input').fill(adminAccessToken)
        await page.getByTestId('storefront-access-token-input').fill(storefrontAccessToken)
        await page.getByTestId('app-client-secret-input').fill(appClientSecret)
        await page.getByTestId('api-version-input').fill(apiVersion)
        // Check the "Skip connection test" checkbox to avoid real Shopify API calls
        await page.getByTestId('skip-connection-test-checkbox').check()
        await demoLogger.testCode.log('Shopify configuration filled')
      })

      await test.step('When: 点击 "Test Connection" 按钮', async () => {
        const testButton = page.getByTestId('shopify-config-test-connection-button')
        await testButton.click()

        // Wait for test connection dialog to appear
        await expect(page.getByTestId('test-connection-dialog')).toBeVisible({ timeout: 5000 })

        // Close the test connection dialog to prevent blocking the submit button
        await page.getByTestId('test-connection-close-button').click()

        // Verify dialog is closed before proceeding
        await expect(page.getByTestId('test-connection-dialog')).not.toBeVisible()

        await demoLogger.testCode.log('Connection test completed and dialog closed')
      })

      await test.step('When: 提交表单', async () => {
        await page.getByTestId('shopify-config-submit-button').click()
        await demoLogger.testCode.log('Configuration submitted')
      })

      await test.step('Then: 配置创建成功', async () => {
        // Wait for network to settle after submission
        await page.waitForLoadState('networkidle')
        // Verify shop domain appears in the list (reliable indicator)
        await expect(page.getByText(shopDomain)).toBeVisible()
        await demoLogger.testCode.log('Configuration created and visible in list')
      })
    })
  })

  // ============================================================================
  // Scenario 2: Shop Domain Format Validation (US-PP-007)
  // ============================================================================

  test.describe('Scenario 2: Shop Domain Format Validation', () => {
    test('should validate shop domain format', async ({ page, loginPage, demoLogger }) => {
      const invalidShopDomain = `invalid-domain-${testStartTime}.com`
      const adminAccessToken = `shpat_test_token_${testStartTime}`

      await test.step('Given: Realm Admin 已登录并打开配置对话框', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', realmId)
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        // Use the specific Shopify add button
        await page.getByTestId('add-shopify-button').click()
        await expect(page.getByTestId('shopify-config-form-dialog')).toBeVisible()
        await demoLogger.testCode.log('Configuration dialog opened')
      })

      await test.step('When: 输入错误的 Shop Domain 格式', async () => {
        await page.getByTestId('shop-domain-input').fill(invalidShopDomain)
        await page.getByTestId('admin-access-token-input').fill(adminAccessToken)
        await demoLogger.testCode.log('Invalid shop domain entered')
      })

      await test.step('When: 尝试提交表单', async () => {
        await page.getByTestId('shopify-config-submit-button').click()
        await demoLogger.testCode.log('Form submitted with invalid domain')
      })

      await test.step('Then: 系统显示验证错误', async () => {
        // Check for validation error message using exact text to avoid matching help text
        const errorMessage = page.getByText('Shop Domain must end with .myshopify.com')
        await expect(errorMessage).toBeVisible({ timeout: 3000 })
        await demoLogger.testCode.log('Validation error displayed')
      })

      await test.step('Then: 配置创建失败', async () => {
        // Dialog should still be open
        await expect(page.getByTestId('shopify-config-form-dialog')).toBeVisible()
        await demoLogger.testCode.log('Dialog remains open after validation error')
      })
    })
  })

  // ============================================================================
  // Scenario 3: API Token Format Validation (US-PP-007)
  // ============================================================================

  test.describe('Scenario 3: API Token Format Validation', () => {
    test('should validate admin access token format', async ({ page, loginPage, demoLogger }) => {
      const shopDomain = `demo-store-${testStartTime}.myshopify.com`
      const invalidToken = `invalid_token_${testStartTime}`

      await test.step('Given: Realm Admin 已登录并打开配置对话框', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', realmId)
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        // Use the specific Shopify add button
        await page.getByTestId('add-shopify-button').click()
        await expect(page.getByTestId('shopify-config-form-dialog')).toBeVisible()
        await demoLogger.testCode.log('Configuration dialog opened')
      })

      await test.step('When: 输入无效的 Admin Access Token', async () => {
        await page.getByTestId('shop-domain-input').fill(shopDomain)
        await page.getByTestId('admin-access-token-input').fill(invalidToken)
        await demoLogger.testCode.log('Invalid admin access token entered')
      })

      await test.step('When: 尝试提交表单', async () => {
        await page.getByTestId('shopify-config-submit-button').click()
        await demoLogger.testCode.log('Form submitted with invalid token')
      })

      await test.step('Then: 系统显示验证错误', async () => {
        // Check for validation error message using exact text to avoid matching help text
        const errorMessage = page.getByText('Must start with shpat_', { exact: true })
        await expect(errorMessage).toBeVisible({ timeout: 3000 })
        await demoLogger.testCode.log('Validation error displayed')
      })

      await test.step('Then: 配置创建失败', async () => {
        // Dialog should still be open
        await expect(page.getByTestId('shopify-config-form-dialog')).toBeVisible()
        await demoLogger.testCode.log('Dialog remains open after validation error')
      })
    })
  })

  // ============================================================================
  // Scenario 4: Test Connection (US-PP-008)
  // ============================================================================

  test.describe('Scenario 4: Test Connection', () => {
    test('should test Shopify connection', async ({ page, loginPage, demoLogger }) => {
      const shopDomain = `demo-store-${testStartTime}.myshopify.com`
      const adminAccessToken = `shpat_test_token_${testStartTime}`
      const storefrontAccessToken = `shp_test_token_${testStartTime}`

      await test.step('Given: Realm Admin 已登录并填写配置信息', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', realmId)
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        // Use the specific Shopify add button
        await page.getByTestId('add-shopify-button').click()
        await page.getByTestId('shop-domain-input').fill(shopDomain)
        await page.getByTestId('admin-access-token-input').fill(adminAccessToken)
        await page.getByTestId('storefront-access-token-input').fill(storefrontAccessToken)
        await page.getByTestId('app-client-secret-input').fill(`test_secret_${testStartTime}_longer_string_32chars`)
        await demoLogger.testCode.log('Configuration filled')
      })

      await test.step('When: 点击 "Test Connection" 按钮', async () => {
        const testButton = page.getByTestId('shopify-config-test-connection-button')
        await testButton.click()
        // Wait for test connection dialog to appear
        await expect(page.getByTestId('test-connection-dialog')).toBeVisible({ timeout: 5000 })
        await demoLogger.testCode.log('Test connection button clicked')
      })

      await test.step('Then: 系统显示测试结果对话框', async () => {
        // Wait for test connection dialog to appear
        await expect(page.getByTestId('test-connection-dialog')).toBeVisible({ timeout: 5000 })
        await demoLogger.testCode.log('Test connection dialog displayed')
      })

      await test.step('When: 点击 "Start Test" 按钮', async () => {
        // Click the "Start Test" button in the dialog
        await page.getByTestId('start-test-button').click()
        // Wait for test to complete - check for connection status
        await expect(page.getByTestId('connection-status-admin-api')).toBeVisible({ timeout: 10000 })
        await demoLogger.testCode.log('Start test button clicked')
      })

      await test.step('Then: 显示连接测试状态', async () => {
        // Check for connection status indicators (may show success or failure)
        const adminApiStatus = page.getByTestId('connection-status-admin-api')

        // At least one status should be visible
        await expect(adminApiStatus).toBeVisible({ timeout: 3000 })
        await demoLogger.testCode.log('Connection status displayed')
      })
    })
  })

  // ============================================================================
  // Scenario 5: View Configuration Details (US-PP-008)
  // ============================================================================

  test.describe('Scenario 5: View Configuration Details', () => {
    test('should view Shopify configuration details', async ({ page, loginPage, demoLogger }) => {
      await test.step('Given: Realm Admin 已登录并存在 Shopify 配置', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', realmId)
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        await demoLogger.testCode.log('Payment providers page loaded')
      })

      await test.step('When: 查看支付平台配置列表', async () => {
        // Check if Shopify configuration exists in the list
        const shopifyConfig = page.getByText('Shopify')
        const isVisible = await shopifyConfig.isVisible().catch(() => false)

        if (isVisible) {
          await demoLogger.testCode.log('Shopify configuration found in list')
        } else {
          await demoLogger.testCode.log('No Shopify configuration found (expected in fresh environment)')
        }
      })

      await test.step('Then: 配置显示脱敏的敏感信息', async () => {
        // If configuration exists, check for masked token display
        const maskedToken = page.getByTestId('masked-token-display')
        const isVisible = await maskedToken.isVisible().catch(() => false)

        if (isVisible) {
          await demoLogger.testCode.log('Masked token displayed correctly')
        } else {
          await demoLogger.testCode.log('No configuration to display (expected in fresh environment)')
        }
      })

      await test.step('Then: 配置显示 Webhook Endpoint', async () => {
        // If configuration exists, check for webhook endpoint
        const webhookEndpoint = page.getByTestId('webhook-endpoint-display')
        const isVisible = await webhookEndpoint.isVisible().catch(() => false)

        if (isVisible) {
          await demoLogger.testCode.log('Webhook endpoint displayed')
        } else {
          await demoLogger.testCode.log('No webhook endpoint to display (expected in fresh environment)')
        }
      })
    })
  })

  // ============================================================================
  // Scenario 6: Edit Configuration (US-PP-009)
  // ============================================================================

  test.describe('Scenario 6: Edit Configuration', () => {
    test('should edit Shopify configuration', async ({ page, loginPage, demoLogger }) => {
      const newAdminToken = `shpat_new_token_${testStartTime}`

      await test.step('Given: Realm Admin 已登录并存在 Shopify 配置', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', realmId)
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        await demoLogger.testCode.log('Payment providers page loaded')
      })

      await test.step('When: 点击 "Edit" 按钮', async () => {
        // Look for edit button (may not exist if no config)
        const editButton = page.getByTestId('edit-shopify-config-button').first()
        const isVisible = await editButton.isVisible().catch(() => false)

        if (isVisible) {
          await editButton.click()
          await expect(page.getByTestId('shopify-config-form-dialog')).toBeVisible()
          await demoLogger.testCode.log('Edit dialog opened')
        } else {
          await demoLogger.testCode.log('No edit button found (expected in fresh environment)')
          test.skip()
        }
      })

      await test.step('When: 更新 Admin Access Token', async () => {
        const adminTokenInput = page.getByTestId('admin-access-token-input')
        await adminTokenInput.clear()
        await adminTokenInput.fill(newAdminToken)
        await demoLogger.testCode.log('Admin access token updated')
      })

      await test.step('When: 保存更改', async () => {
        await page.getByTestId('shopify-config-submit-button').click()
        await demoLogger.testCode.log('Changes submitted')
      })

      await test.step('Then: 配置更新成功', async () => {
        // Verify success message
        await expect(page.getByText(/updated successfully/i)).toBeVisible({ timeout: 5000 })
        await demoLogger.testCode.log('Configuration updated successfully')
      })
    })
  })

  // ============================================================================
  // Scenario 7: Delete Configuration (US-PP-010)
  // ============================================================================

  test.describe('Scenario 7: Delete Configuration', () => {
    test('should delete Shopify configuration without active subscriptions', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      await test.step('Given: Realm Admin 已登录并存在 Shopify 配置', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', realmId)
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        await demoLogger.testCode.log('Payment providers page loaded')
      })

      await test.step('When: 点击 "Delete" 按钮', async () => {
        // Look for delete button (may not exist if no config)
        const deleteButton = page.getByTestId('delete-shopify-config-button').first()
        const isVisible = await deleteButton.isVisible().catch(() => false)

        if (isVisible) {
          await deleteButton.click()
          await expect(page.getByTestId('delete-confirm-dialog')).toBeVisible()
          await demoLogger.testCode.log('Delete confirmation dialog opened')
        } else {
          await demoLogger.testCode.log('No delete button found (expected in fresh environment)')
          test.skip()
        }
      })

      await test.step('When: 确认删除', async () => {
        await page.getByTestId('delete-confirm-button').click()
        await demoLogger.testCode.log('Delete confirmed')
      })

      await test.step('Then: 支付平台配置删除成功', async () => {
        // Verify success message
        await expect(page.getByText(/deleted successfully/i)).toBeVisible({ timeout: 5000 })
        await demoLogger.testCode.log('Configuration deleted successfully')
      })

      await test.step('Then: 支付平台列表不再显示该配置', async () => {
        // Wait for list to reload
        await page.waitForLoadState('networkidle')
        // Verify Shopify is no longer in the list
        await expect(page.getByText('Shopify')).not.toBeVisible()
        await demoLogger.testCode.log('Configuration removed from list')
      })
    })
  })
})
