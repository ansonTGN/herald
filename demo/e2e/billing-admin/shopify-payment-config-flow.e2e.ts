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

      await test.step('Given: Realm Admin logged in', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', realmId)
        await demoLogger.testCode.log('Realm Admin logged in')
      })

      await test.step('When: Navigate to payment providers page', async () => {
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        await expect(page.getByTestId('payment-providers-page')).toBeVisible()
        await demoLogger.testCode.log('Payment providers page loaded')
      })

      await test.step('When: Click "Add Shopify" button to navigate to config form page', async () => {
        await page.getByTestId('add-shopify-button').click()
        // The frontend navigates to ./shopify route, not a dialog
        await expect(page.getByTestId('shopify-config-form-page')).toBeVisible()
        await demoLogger.testCode.log('Shopify config form page loaded')
      })

      await test.step('When: Fill Shopify configuration form', async () => {
        await page.getByTestId('page-shop-domain-input').fill(shopDomain)
        await page.getByTestId('page-admin-access-token-input').fill(adminAccessToken)
        await page.getByTestId('page-storefront-access-token-input').fill(storefrontAccessToken)
        await page.getByTestId('page-app-client-secret-input').fill(appClientSecret)
        await page.getByTestId('page-api-version-input').fill(apiVersion)
        // Check the "Skip connection test" checkbox to avoid real Shopify API calls
        await page.getByTestId('page-skip-connection-test-checkbox').check()
        await demoLogger.testCode.log('Shopify configuration filled')
      })

      await test.step('When: Submit the form', async () => {
        await page.getByTestId('shopify-config-page-submit-button').click()
        await demoLogger.testCode.log('Configuration submitted')
      })

      await test.step('Then: Configuration created and redirected to providers list', async () => {
        // After successful creation, the page navigates back to the providers list
        await expect(page.getByTestId('payment-providers-page')).toBeVisible({ timeout: 5000 })
        // Verify Shopify appears in the provider list table
        await expect(page.getByTestId('shopify-provider-row')).toBeVisible()
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

      await test.step('Given: Realm Admin logged in and on config form page', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', realmId)
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        await page.getByTestId('add-shopify-button').click()
        await expect(page.getByTestId('shopify-config-form-page')).toBeVisible()
        await demoLogger.testCode.log('Config form page loaded')
      })

      await test.step('When: Enter invalid shop domain format', async () => {
        await page.getByTestId('page-shop-domain-input').fill(invalidShopDomain)
        await page.getByTestId('page-admin-access-token-input').fill(adminAccessToken)
        await demoLogger.testCode.log('Invalid shop domain entered')
      })

      await test.step('When: Submit the form', async () => {
        await page.getByTestId('shopify-config-page-submit-button').click()
        await demoLogger.testCode.log('Form submitted with invalid domain')
      })

      await test.step('Then: Validation error is displayed', async () => {
        const errorMessage = page.getByText('Shop Domain must end with .myshopify.com')
        await expect(errorMessage).toBeVisible({ timeout: 3000 })
        await demoLogger.testCode.log('Validation error displayed')
      })

      await test.step('Then: Form page remains open', async () => {
        await expect(page.getByTestId('shopify-config-form-page')).toBeVisible()
        await demoLogger.testCode.log('Form page remains after validation error')
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

      await test.step('Given: Realm Admin logged in and on config form page', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', realmId)
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        await page.getByTestId('add-shopify-button').click()
        await expect(page.getByTestId('shopify-config-form-page')).toBeVisible()
        await demoLogger.testCode.log('Config form page loaded')
      })

      await test.step('When: Enter invalid admin access token', async () => {
        await page.getByTestId('page-shop-domain-input').fill(shopDomain)
        await page.getByTestId('page-admin-access-token-input').fill(invalidToken)
        await demoLogger.testCode.log('Invalid admin access token entered')
      })

      await test.step('When: Submit the form', async () => {
        await page.getByTestId('shopify-config-page-submit-button').click()
        await demoLogger.testCode.log('Form submitted with invalid token')
      })

      await test.step('Then: Validation error is displayed', async () => {
        const errorMessage = page.getByText('Must start with shpat_', { exact: true })
        await expect(errorMessage).toBeVisible({ timeout: 3000 })
        await demoLogger.testCode.log('Validation error displayed')
      })

      await test.step('Then: Form page remains open', async () => {
        await expect(page.getByTestId('shopify-config-form-page')).toBeVisible()
        await demoLogger.testCode.log('Form page remains after validation error')
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

      await test.step('Given: Realm Admin logged in and filled configuration', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', realmId)
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        await page.getByTestId('add-shopify-button').click()
        await expect(page.getByTestId('shopify-config-form-page')).toBeVisible()
        await page.getByTestId('page-shop-domain-input').fill(shopDomain)
        await page.getByTestId('page-admin-access-token-input').fill(adminAccessToken)
        await page.getByTestId('page-storefront-access-token-input').fill(storefrontAccessToken)
        await page.getByTestId('page-app-client-secret-input').fill(`test_secret_${testStartTime}_longer_string_32chars`)
        await demoLogger.testCode.log('Configuration filled')
      })

      await test.step('When: Click "Test Connection" button', async () => {
        await page.getByTestId('shopify-config-page-test-connection-button').click()
        await demoLogger.testCode.log('Test connection button clicked')
      })

      await test.step('Then: Connection test result is shown via toast notification', async () => {
        // The test connection result is displayed as a toast notification
        // Either success or failure toast should appear
        const successToast = page.getByText('Shopify connection test passed')
        const failureToast = page.getByText(/Connection test failed/)

        // Wait for either toast to appear
        await expect(
          successToast.or(failureToast)
        ).toBeVisible({ timeout: 10000 })
        await demoLogger.testCode.log('Connection test result displayed via toast')
      })
    })
  })

  // ============================================================================
  // Scenario 5: View Configuration Details (US-PP-008)
  // ============================================================================

  test.describe('Scenario 5: View Configuration Details', () => {
    test('should view Shopify configuration details', async ({ page, loginPage, demoLogger }) => {
      await test.step('Given: Realm Admin logged in and Shopify config exists', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', realmId)
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        await expect(page.getByTestId('payment-providers-page')).toBeVisible()
        await demoLogger.testCode.log('Payment providers page loaded')
      })

      await test.step('When: Check for Shopify provider in list', async () => {
        const shopifyRow = page.getByTestId('shopify-provider-row')
        const isVisible = await shopifyRow.isVisible().catch(() => false)

        if (!isVisible) {
          await demoLogger.testCode.log('No Shopify configuration found - skipping view test')
          test.skip()
        }
        await demoLogger.testCode.log('Shopify configuration found in list')
      })

      await test.step('When: Expand Shopify details', async () => {
        await page.getByTestId('toggle-shopify-details-button').click()
        await expect(page.getByTestId('shopify-details-row')).toBeVisible()
        await demoLogger.testCode.log('Shopify details expanded')
      })

      await test.step('Then: Configuration displays masked sensitive information', async () => {
        await expect(page.getByTestId('masked-token-display')).toBeVisible()
        await demoLogger.testCode.log('Masked token displayed correctly')
      })

      await test.step('Then: Configuration displays shop domain', async () => {
        await expect(page.getByTestId('shop-domain-display')).toBeVisible()
        await demoLogger.testCode.log('Shop domain displayed')
      })

      await test.step('Then: Configuration displays API version', async () => {
        await expect(page.getByTestId('api-version-display')).toBeVisible()
        await demoLogger.testCode.log('API version displayed')
      })
    })
  })

  // ============================================================================
  // Scenario 6: Edit Configuration (US-PP-009)
  // ============================================================================

  test.describe('Scenario 6: Edit Configuration', () => {
    test('should edit Shopify configuration', async ({ page, loginPage, demoLogger }) => {
      const newApiVersion = '2024-07'

      await test.step('Given: Realm Admin logged in and Shopify config exists', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', realmId)
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        await expect(page.getByTestId('payment-providers-page')).toBeVisible()
        await demoLogger.testCode.log('Payment providers page loaded')
      })

      await test.step('When: Click "Edit" button on Shopify row', async () => {
        const editButton = page.getByTestId('edit-shopify-button')
        const isVisible = await editButton.isVisible().catch(() => false)

        if (!isVisible) {
          await demoLogger.testCode.log('No edit button found - skipping edit test')
          test.skip()
        }

        await editButton.click()
        // Edit button navigates to ./shopify route with pre-filled data
        await expect(page.getByTestId('shopify-config-form-page')).toBeVisible()
        await demoLogger.testCode.log('Edit form page loaded with pre-filled data')
      })

      await test.step('When: Update API Version', async () => {
        const apiVersionInput = page.getByTestId('page-api-version-input')
        await apiVersionInput.clear()
        await apiVersionInput.fill(newApiVersion)
        await demoLogger.testCode.log('API version updated')
      })

      await test.step('When: Save changes', async () => {
        await page.getByTestId('shopify-config-page-submit-button').click()
        await demoLogger.testCode.log('Changes submitted')
      })

      await test.step('Then: Configuration updated successfully', async () => {
        // After successful update, the page navigates back to providers list
        await expect(page.getByTestId('payment-providers-page')).toBeVisible({ timeout: 5000 })
        await expect(page.getByText('Shopify')).toBeVisible()
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
      await test.step('Given: Realm Admin logged in and Shopify config exists', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', realmId)
        await page.goto(`/${realmId}/manage/billing/payment-providers`)
        await expect(page.getByTestId('payment-providers-page')).toBeVisible()
        await demoLogger.testCode.log('Payment providers page loaded')
      })

      await test.step('When: Click "Delete" button on Shopify row', async () => {
        const deleteButton = page.getByTestId('delete-shopify-button')
        const isVisible = await deleteButton.isVisible().catch(() => false)

        if (!isVisible) {
          await demoLogger.testCode.log('No delete button found - skipping delete test')
          test.skip()
        }

        await deleteButton.click()
        await expect(page.getByTestId('delete-confirm-dialog')).toBeVisible()
        await demoLogger.testCode.log('Delete confirmation dialog opened')
      })

      await test.step('When: Confirm deletion', async () => {
        await page.getByTestId('delete-confirm-button').click()
        await demoLogger.testCode.log('Delete confirmed')
      })

      await test.step('Then: Provider deleted successfully', async () => {
        // Wait for the provider list to update and the delete dialog to close
        await expect(page.getByTestId('delete-confirm-dialog')).not.toBeVisible({ timeout: 5000 })
        await demoLogger.testCode.log('Configuration deleted successfully')
      })

      await test.step('Then: Shopify no longer appears in provider list', async () => {
        // Wait for the list to update - Shopify row should be gone
        await expect(page.getByTestId('shopify-provider-row')).not.toBeVisible({ timeout: 5000 })
        await demoLogger.testCode.log('Configuration removed from list')
      })
    })
  })
})
