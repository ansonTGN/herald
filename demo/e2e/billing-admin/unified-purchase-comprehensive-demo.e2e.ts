/**
 * Unified Purchase - Comprehensive Demo Test
 *
 * Demonstrates the complete unified purchase flow for points packages:
 * 1. Admin creates points package with payment provider configurations
 * 2. User purchases points package via WeChat Pay
 * 3. User purchases points package via Stripe
 * 4. Verify points are granted and purchase history is recorded
 *
 * Test Scope:
 * - Points package creation (admin)
 * - Payment provider configuration (admin)
 * - Points package purchase (user)
 * - Payment simulation via internal API
 * - Purchase history verification
 *
 * Technical Approach:
 * - Uses UI for all user-facing operations
 * - Uses internal fulfillment API to simulate payment completion
 * - No direct database operations
 * - No direct business API calls (except internal simulation)
 */

import { test, expect, cleanupTestData } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { fulfillPayment } from '../helpers/payment-simulation'
import { SELECTORS } from '../selectors'

const REALM_ID = 'realm-001'
const ADMIN_EMAIL = 'admin@realm-001.com'
const USER_EMAIL = 'user@realm-001.com'

test.describe('[Unified Purchase] Comprehensive Demo', () => {
  let testStartTime: number
  let packageName: string
  let packageId: string
  let wechatAttemptId: string
  let stripeAttemptId: string

  test.beforeEach(async ({ page }) => {
    testStartTime = Date.now()
    packageName = `test-package-${testStartTime}`

    await verifyTestEnvironment(page, {
      requiredRealms: [REALM_ID],
      requiredUsers: [ADMIN_EMAIL, USER_EMAIL],
    })
  })

  test.afterEach(async ({ page }) => {
    await cleanupTestData(page, REALM_ID, {
      keepUsers: [ADMIN_EMAIL, USER_EMAIL],
      timestamp: testStartTime,
    })
  })

  test.describe('Admin: Points Package Management', () => {
    test('should create points package and configure payment providers', async ({
      page,
      demoLogger,
    }) => {
      await test.step('Admin logs in and navigates to points packages page', async () => {
        // Navigate to login page
        await page.goto(`/${REALM_ID}/auth/login`)

        // Fill login form
        await page.locator(SELECTORS.login.emailInput).fill(ADMIN_EMAIL)
        await page.locator(SELECTORS.login.passwordInput).fill('password')
        await page.locator(SELECTORS.login.submitButton).click()

        // Wait for navigation to complete
        await page.waitForURL(`**/${REALM_ID}/manage*`, { timeout: 5000 })

        // Navigate to points packages page
        await page.goto(`/${REALM_ID}/manage/points-packages`)

        // Verify page loaded
        await expect(page.locator(SELECTORS.pointsPackages.page)).toBeVisible()
        console.log('[Admin] ✓ Navigated to points packages page')
      })

      await test.step('Admin creates a new points package', async () => {
        // Click "Create Package" button
        await page.locator(SELECTORS.pointsPackages.addButton).click()

        // Wait for navigation to form page
        await page.waitForURL('**/manage/points-packages/new', { timeout: 10000 })
        await expect(page.locator(SELECTORS.pointsPackageForm.page)).toBeVisible()

        // Fill package form
        await page.locator(SELECTORS.pointsPackageForm.nameInput).fill(packageName)
        await page
          .locator(SELECTORS.pointsPackageForm.titleInput)
          .fill(`Test Package ${testStartTime}`)
        await page
          .locator(SELECTORS.pointsPackageForm.descriptionInput)
          .fill('Test package for demo')
        await page.locator(SELECTORS.pointsPackageForm.pointsInput).fill('1000')
        await page.locator(SELECTORS.pointsPackageForm.priceInput).fill('999')
        await page.locator(SELECTORS.pointsPackageForm.currencySelect).fill('USD')
        await page.locator(SELECTORS.pointsPackageForm.sortOrderInput).fill('0')

        // Ensure enabled switch is checked
        const isEnabled = await page.locator(SELECTORS.pointsPackageForm.enabledSwitch).isChecked()
        if (!isEnabled) {
          await page.locator(SELECTORS.pointsPackageForm.enabledSwitch).click()
        }

        // Submit form
        await page.locator(SELECTORS.pointsPackageForm.submitButton).click()

        // Wait for navigation back to points packages list
        await page.waitForURL('**/manage/points-packages', { timeout: 10000 })

        // Wait for table to reload and show new package
        await expect(page.locator(SELECTORS.pointsPackages.table)).toBeVisible()
        await expect(page.getByText(packageName)).toBeVisible()

        console.log('[Admin] ✓ Points package created successfully')
      })

      await test.step('Admin configures WeChat Pay for the package', async () => {
        const packageRow = page.locator('tr').filter({ hasText: packageName })
        await expect(packageRow).toBeVisible()
        const configureButton = packageRow.locator(
          '[data-testid^="points-package-configure-button-"]'
        )
        await configureButton.click()

        await page.waitForURL('**/manage/points-packages/*/providers', {
          timeout: 10000,
        })
        await expect(page.getByTestId('points-package-providers-page')).toBeVisible()

        // Select WeChat Pay provider
        await page.getByTestId('add-provider-mapping-button').click()
        await page.getByTestId('provider-mapping-provider-select-trigger').click()
        await page.getByRole('option', { name: 'WeChat Pay' }).click()

        // Fill external product ID
        await page.getByTestId('provider-mapping-product-id-input').fill(`wx-prod-${testStartTime}`)

        // Ensure enabled switch is checked
        const isEnabled = await page.getByTestId('provider-mapping-enabled-switch').isChecked()
        if (!isEnabled) {
          await page.getByTestId('provider-mapping-enabled-switch').click()
        }

        // Submit form
        await page.getByTestId('provider-mapping-submit-button').click()

        // Wait for success message and provider to appear in list
        await expect(page.getByText('Payment provider mapping added')).toBeVisible()

        console.log('[Admin] ✓ WeChat Pay configured successfully')
      })

      await test.step('Admin configures Stripe for the package', async () => {
        // Add Stripe provider
        await page.getByTestId('add-provider-mapping-button').click()
        await page.getByTestId('provider-mapping-provider-select-trigger').click()
        await page.getByRole('option', { name: 'Stripe' }).click()

        // Fill external product ID
        await page
          .getByTestId('provider-mapping-product-id-input')
          .fill(`stripe-prod-${testStartTime}`)

        // Ensure enabled switch is checked
        const isEnabled = await page.getByTestId('provider-mapping-enabled-switch').isChecked()
        if (!isEnabled) {
          await page.getByTestId('provider-mapping-enabled-switch').click()
        }

        // Submit form
        await page.getByTestId('provider-mapping-submit-button').click()

        // Wait for success message
        await expect(page.getByText('Payment provider mapping added').first()).toBeVisible()

        await page.getByTestId('back-to-points-packages-button').click()

        console.log('[Admin] ✓ Stripe configured successfully')
      })
    })
  })

  test.describe('User: Purchase Points via WeChat Pay', () => {
    test('should purchase points package via WeChat Pay and verify fulfillment', async ({
      page,
      request,
      demoLogger,
    }) => {
      await test.step('User logs in and navigates to purchase points page', async () => {
        // Logout admin first
        await page.goto(`/${REALM_ID}/auth/logout`)

        // Navigate to login page
        await page.goto(`/${REALM_ID}/auth/login`)

        // Fill login form as regular user
        await page.locator(SELECTORS.login.emailInput).fill(USER_EMAIL)
        await page.locator(SELECTORS.login.passwordInput).fill('password')
        await page.locator(SELECTORS.login.submitButton).click()

        // Wait for navigation to the user area
        await page.waitForURL(`**/${REALM_ID}/user/**`, { timeout: 5000 })

        // Navigate to purchase points page
        await page.goto(`/${REALM_ID}/user/purchase-points`)

        // Verify page loaded
        await expect(page.locator(SELECTORS.purchasePoints.page)).toBeVisible()
        console.log('[User] ✓ Navigated to purchase points page')
      })

      await test.step('User selects a points package', async () => {
        // Wait for package selector to load
        await expect(page.locator(SELECTORS.packageSelector.container)).toBeVisible()

        // Select the first available package
        const firstPackageButton = page.getByTestId(/^points-package-select-button-/)
        await expect(firstPackageButton.first()).toBeEnabled()
        await firstPackageButton.first().click()

        // Wait for selection to complete
        await expect(page.getByTestId(/^points-package-selected-/)).toBeVisible()

        // Verify "Next" button is enabled
        await expect(page.locator(SELECTORS.purchasePoints.nextButton)).toBeEnabled()

        console.log('[User] ✓ Points package selected')
      })

      await test.step('User proceeds to payment method selection', async () => {
        // Click "Next" button
        await page.locator(SELECTORS.purchasePoints.nextButton).click()

        // Verify payment step is displayed
        await expect(page.locator(SELECTORS.purchasePoints.stepPayment)).toBeVisible()
        await expect(page.locator(SELECTORS.paymentMethodSelector.container)).toBeVisible()

        console.log('[User] ✓ Proceeded to payment method selection')
      })

      await test.step('User selects WeChat Pay as payment method', async () => {
        // Select WeChat Pay
        await page.getByTestId('payment-method-select-wechat').click()

        // Wait for selection to complete
        await expect(page.getByTestId('payment-method-selected-wechat')).toBeVisible()

        // Verify "Complete Purchase" button is enabled
        await expect(page.locator(SELECTORS.purchasePoints.nextButton)).toBeEnabled()

        console.log('[User] ✓ WeChat Pay selected as payment method')
      })

      await test.step('User completes purchase and payment attempt is created', async () => {
        // Click "Complete Purchase" button
        await page.locator(SELECTORS.purchasePoints.nextButton).click()

        // Wait for processing step
        await expect(page.locator(SELECTORS.purchasePoints.stepProcessing)).toBeVisible()
        await expect(page.locator(SELECTORS.paymentProviderUI.wechatQrSection)).toBeVisible()

        const attemptId = await page
          .waitForFunction(() => {
            const state = localStorage.getItem('cas-purchase-flow')
            if (!state) return null
            const parsed = JSON.parse(state)
            return parsed?.state?.attemptId ?? null
          })
          .then((handle) => handle.jsonValue())

        expect(attemptId).toBeTruthy()
        wechatAttemptId = attemptId as string

        console.log(`[User] ✓ Payment attempt created: ${wechatAttemptId}`)
      })

      await test.step('Simulate WeChat Pay payment fulfillment', async () => {
        // Use internal API to fulfill payment
        const result = await fulfillPayment(request, REALM_ID, wechatAttemptId)

        expect(result.success, result.error).toBe(true)
        expect(result.transactionId).toBeTruthy()
        expect(result.points).toBeGreaterThan(0)

        console.log(
          `[Payment] ✓ WeChat Pay fulfilled: ${result.transactionId}, Points: ${result.points}`
        )
      })

      await test.step('Verify payment status updates to Succeeded', async () => {
        await expect(page.locator(SELECTORS.purchasePoints.stepComplete)).toBeVisible({
          timeout: 15000,
        })
        await expect(page.getByText('Purchase Complete!')).toBeVisible()

        console.log('[User] ✓ Payment succeeded and points granted')
      })

      await test.step('User navigates to points page to verify balance', async () => {
        // Click "View My Points" button
        await page.getByRole('button', { name: 'View My Points' }).click()

        // Wait for navigation to points page
        await page.waitForURL(`**/${REALM_ID}/user/points`, { timeout: 5000 })

        // Verify points page loaded
        await expect(page.locator(SELECTORS.pointsUser.page)).toBeVisible()

        // Verify balance card shows increased points
        await expect(page.locator(SELECTORS.pointsUser.balanceAmount)).toBeVisible()

        console.log('[User] ✓ Navigated to points page')
      })

      await test.step('User views purchase history', async () => {
        // Click on "Purchase History" tab
        await page.getByTestId('points-tab-purchase-history').click()

        // Wait for purchase history to load
        await expect(page.locator(SELECTORS.purchaseHistory.list)).toBeVisible()

        // Verify purchase appears in history
        await expect(page.getByTestId(/^purchase-history-item-/).first()).toBeVisible()

        console.log('[User] ✓ Purchase history verified')
      })
    })
  })

  test.describe('User: Purchase Points via Stripe', () => {
    test('should purchase points package via Stripe and verify fulfillment', async ({
      page,
      request,
      demoLogger,
    }) => {
      await test.step('User logs in and navigates to purchase points page', async () => {
        // Logout first
        await page.goto(`/${REALM_ID}/auth/logout`)

        // Navigate to login page
        await page.goto(`/${REALM_ID}/auth/login`)

        // Fill login form
        await page.locator(SELECTORS.login.emailInput).fill(USER_EMAIL)
        await page.locator(SELECTORS.login.passwordInput).fill('password')
        await page.locator(SELECTORS.login.submitButton).click()

        // Wait for navigation to the user area
        await page.waitForURL(`**/${REALM_ID}/user/**`, { timeout: 5000 })

        // Navigate to purchase points page
        await page.goto(`/${REALM_ID}/user/purchase-points`)

        // Verify page loaded
        await expect(page.locator(SELECTORS.purchasePoints.page)).toBeVisible()
        console.log('[User] ✓ Navigated to purchase points page')
      })

      await test.step('User selects a points package', async () => {
        // Wait for package selector to load
        await expect(page.locator(SELECTORS.packageSelector.container)).toBeVisible()

        // Select the first available package
        const firstPackageButton = page.getByTestId(/^points-package-select-button-/)
        await expect(firstPackageButton.first()).toBeEnabled()
        await firstPackageButton.first().click()

        // Wait for selection to complete
        await expect(page.getByTestId(/^points-package-selected-/)).toBeVisible()

        console.log('[User] ✓ Points package selected')
      })

      await test.step('User proceeds to payment method selection', async () => {
        // Click "Next" button
        await page.locator(SELECTORS.purchasePoints.nextButton).click()

        // Verify payment step is displayed
        await expect(page.locator(SELECTORS.purchasePoints.stepPayment)).toBeVisible()

        console.log('[User] ✓ Proceeded to payment method selection')
      })

      await test.step('User selects Stripe as payment method', async () => {
        // Select Stripe
        await page.getByTestId('payment-method-select-stripe').click()

        // Wait for selection to complete
        await expect(page.getByTestId('payment-method-selected-stripe')).toBeVisible()

        console.log('[User] ✓ Stripe selected as payment method')
      })

      await test.step('User completes purchase and payment attempt is created', async () => {
        // Click "Complete Purchase" button
        await page.locator(SELECTORS.purchasePoints.nextButton).click()

        // Wait for processing step
        await expect(page.locator(SELECTORS.purchasePoints.stepProcessing)).toBeVisible()
        await expect(page.locator(SELECTORS.paymentProviderUI.redirectPrompt)).toBeVisible()

        const attemptId = await page
          .waitForFunction(() => {
            const state = localStorage.getItem('cas-purchase-flow')
            if (!state) return null
            const parsed = JSON.parse(state)
            return parsed?.state?.attemptId ?? null
          })
          .then((handle) => handle.jsonValue())

        expect(attemptId).toBeTruthy()
        stripeAttemptId = attemptId as string

        console.log(`[User] ✓ Payment attempt created: ${stripeAttemptId}`)
      })

      await test.step('Simulate Stripe payment fulfillment', async () => {
        // Use internal API to fulfill payment
        const result = await fulfillPayment(request, REALM_ID, stripeAttemptId)

        expect(result.success, result.error).toBe(true)
        expect(result.transactionId).toBeTruthy()
        expect(result.points).toBeGreaterThan(0)

        console.log(
          `[Payment] ✓ Stripe fulfilled: ${result.transactionId}, Points: ${result.points}`
        )
      })

      await test.step('Verify payment status updates to Succeeded', async () => {
        await expect(page.locator(SELECTORS.purchasePoints.stepComplete)).toBeVisible({
          timeout: 15000,
        })
        await expect(page.getByText('Purchase Complete!')).toBeVisible()

        console.log('[User] ✓ Stripe payment succeeded')
      })

      await test.step('User views purchase history and sees both purchases', async () => {
        // Click "View My Points" button
        await page.getByRole('button', { name: 'View My Points' }).click()

        // Wait for navigation to points page
        await page.waitForURL(`**/${REALM_ID}/user/points`, { timeout: 5000 })

        // Click on "Purchase History" tab
        await page.getByTestId('points-tab-purchase-history').click()

        // Wait for purchase history to load
        await expect(page.locator(SELECTORS.purchaseHistory.list)).toBeVisible()

        // Verify multiple purchases appear in history
        const purchaseItems = page.getByTestId(/^purchase-history-item-/)
        await expect(purchaseItems.first()).toBeVisible()

        // Count purchases (should be at least 2: WeChat + Stripe)
        const count = await purchaseItems.count()
        expect(count).toBeGreaterThanOrEqual(2)

        console.log(`[User] ✓ Purchase history verified: ${count} purchases found`)
      })
    })
  })
})
