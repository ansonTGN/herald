/**
 * Product Management Demo Tests
 *
 * User Stories:
 * - US-PR-001: Create Product (code uniqueness, format validation)
 * - US-PR-002: Edit Product (basic info, code immutable)
 * - US-PR-003: View Product List (table display, plans count)
 * - US-PR-004: Enable/Disable Product (toggle status)
 * - US-PR-005: Delete Product (without plans, confirmation dialog)
 * - US-PR-006: Plan-Product Relationship (create plan under product, move plan, delete blocked)
 *
 * Design Doc: .ai/design/add-product.md
 * User Stories: docs/user-stories/product-management.md
 *
 * UnifiedLogger Usage:
 * - All tests use UnifiedLogger through 'demoLogger' fixture
 * - Logger is automatically initialized and finalized by fixture
 */

import { test, cleanupTestData, expect, type Page } from '../fixtures/demo-page.fixtures'
import { createSubscriptionPlan, openEditSubscriptionPlanDialog, openDeleteSubscriptionPlanDialog, confirmDeleteSubscriptionPlan } from './helpers/billing-page.helpers'
import {
  createProduct,
  openEditProductDialog,
  openDeleteProductDialog,
  confirmDeleteProduct,
  confirmDeleteProductAndWait,
  verifyProductInTable,
  verifyProductNotInTable,
} from './helpers/product-page.helpers'
import { DEMO_ADMIN } from '../helpers/auth'
import { verifyTestEnvironment } from '../helpers/environment-setup'

/**
 * Helper function to wait for old toasts to disappear.
 * Sonner toasts have a 4-second duration, which can cause duplicate
 * notification detection when test steps run in quick succession.
 *
 * This function waits for existing toasts matching the given pattern
 * to disappear before proceeding, ensuring only new toasts are detected.
 *
 * @param page - Playwright page object
 * @param toastPattern - Regular expression pattern matching toast text
 */
async function waitForOldToastsToDisappear(page: Page, toastPattern: RegExp): Promise<void> {
  try {
    // Wait for any existing toasts matching the pattern to disappear
    // Timeout: 4500ms (Sonner toast duration + 500ms safety margin)
    const existingToasts = page.getByText(toastPattern)
    if (await existingToasts.count() > 0) {
      await expect(existingToasts.first()).toHaveCount(0, { timeout: 4500 })
    }
  } catch {
    // If toasts are already gone or timing out, continue anyway
    // This prevents the test from failing if toasts disappear faster than expected
  }
}

test.describe('[Billing Admin] Product Management Demo Tests', () => {
  // Verify test environment before each test
  test.beforeEach(async ({ page }) => {
    await verifyTestEnvironment(page, {
      requiredRealms: [DEMO_ADMIN.realmId],
      requiredUsers: [DEMO_ADMIN.email],
    })
  })

  // Single test.afterEach for cleanup
  test.afterEach(async ({ page, testStartTime }) => {
    await cleanupTestData(page, DEMO_ADMIN.realmId, {
      keepUsers: [DEMO_ADMIN.email],
      timestamp: testStartTime,
    })
  })

  // ============================================================================
  // User Stories US-PR-001 ~ US-PR-005: Product CRUD and Status Management
  // ============================================================================

  test.describe('US-PR-001 ~ US-PR-005: Product CRUD and Status', () => {
    test('Product CRUD lifecycle', async ({
      page,
      loginPage,
      demoLogger,
      testStartTime,
    }) => {
      const productACode = `test-product-a-${testStartTime}`
      const productATitle = `Test Product A`
      const productBCode = `test-product-b-${testStartTime}`
      const productBTitle = `Test Product B`

      // ------------------------------------------------------------------
      // Step 1: Login and Navigate to Products page
      // ------------------------------------------------------------------
      await test.step('Step 1: Login and Navigate to Products page', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', DEMO_ADMIN.realmId)
        await page.goto(`/${DEMO_ADMIN.realmId}/manage/products`)
        await expect(page.getByTestId('products-page')).toBeVisible()
        console.log('[Test] Navigated to Products page')
      })

      // ------------------------------------------------------------------
      // Step 2: Create First Product (US-PR-001 Scene 1)
      // ------------------------------------------------------------------
      await test.step('Step 2: Create First Product (US-PR-001 Scene 1)', async () => {
        await createProduct(page, {
          code: productACode,
          title: productATitle,
          description: 'Product for E2E testing',
          enabled: true,
        })

        // Verify success toast
        await expect(
          page.getByText(/Product "Test Product A" created successfully/)
        ).toBeVisible()

        // Verify product appears in table
        await verifyProductInTable(page, productACode)
        console.log('[Test] First product created successfully')
      })

      // ------------------------------------------------------------------
      // Step 3: Create Second Product
      // ------------------------------------------------------------------
      await test.step('Step 3: Create Second Product', async () => {
        await createProduct(page, {
          code: productBCode,
          title: productBTitle,
          description: 'Second product for E2E',
          enabled: true,
        })

        await expect(
          page.getByText(/Product "Test Product B" created successfully/)
        ).toBeVisible()

        await verifyProductInTable(page, productBCode)
        console.log('[Test] Second product created successfully')
      })

      // ------------------------------------------------------------------
      // Step 4: Validate Code Uniqueness (US-PR-001 Scene 2)
      // ------------------------------------------------------------------
      await test.step('Step 4: Validate Code Uniqueness (US-PR-001 Scene 2)', async () => {
        await page.getByTestId('add-product-button').click()
        await expect(page.getByRole('dialog')).toBeVisible()

        await page.getByTestId('product-code-input').fill(productACode)
        await page.getByTestId('product-title-input').fill('Duplicate Test')
        await page.getByTestId('product-form-submit-button').click()

        // Dialog stays open on validation error (either form-level or toast error)
        await expect(page.getByRole('dialog')).toBeVisible()

        // Close the dialog via cancel button
        await page.getByTestId('product-form-cancel-button').click()
        await expect(page.getByRole('dialog')).toBeHidden()
        console.log('[Test] Code uniqueness validation verified')
      })

      // ------------------------------------------------------------------
      // Step 5: Validate Code Format (US-PR-001 Scene 3)
      // ------------------------------------------------------------------
      await test.step('Step 5: Validate Code Format (US-PR-001 Scene 3)', async () => {
        await page.getByTestId('add-product-button').click()
        await expect(page.getByRole('dialog')).toBeVisible()

        // Fill with invalid code (spaces and special characters)
        await page.getByTestId('product-code-input').fill('invalid code!@#')
        await page.getByTestId('product-title-input').fill('Invalid Code Test')

        // Trigger validation by submitting
        await page.getByTestId('product-form-submit-button').click()

        // Inline validation error should appear
        await expect(
          page.getByText(/Product code can only contain lowercase letters/)
        ).toBeVisible()

        // Close the dialog
        await page.getByTestId('product-form-cancel-button').click()
        await expect(page.getByRole('dialog')).toBeHidden()
        console.log('[Test] Code format validation verified')
      })

      // ------------------------------------------------------------------
      // Step 6: View Product List (US-PR-003 Scene 1)
      // ------------------------------------------------------------------
      await test.step('Step 6: View Product List (US-PR-003 Scene 1)', async () => {
        // Both products should be visible in the table
        await expect(page.getByTestId('product-table')).toBeVisible()
        await verifyProductInTable(page, productACode)
        await verifyProductInTable(page, productBCode)

        // Verify table headers
        await expect(page.getByRole('columnheader', { name: 'Code' })).toBeVisible()
        await expect(page.getByRole('columnheader', { name: 'Title' })).toBeVisible()
        await expect(page.getByRole('columnheader', { name: 'Status' })).toBeVisible()

        // Verify status badges show "Enabled" for both products
        const productARow = page.locator(`tr:has-text("${productACode}")`)
        const productBRow = page.locator(`tr:has-text("${productBCode}")`)
        await expect(productARow.getByText('Enabled')).toBeVisible()
        await expect(productBRow.getByText('Enabled')).toBeVisible()
        console.log('[Test] Product list view verified')
      })

      // ------------------------------------------------------------------
      // Step 7: Edit Product (US-PR-002 Scene 1 + Scene 2)
      // ------------------------------------------------------------------
      await test.step('Step 7: Edit Product (US-PR-002 Scene 1+2)', async () => {
        await openEditProductDialog(page, productACode)

        // US-PR-002 Scene 2: Code field is disabled in edit mode
        const codeInput = page.getByTestId('product-code-input')
        await expect(codeInput).toBeDisabled()

        // Update title and description
        await page.getByTestId('product-title-input').fill('Test Product A Updated')
        await page.getByTestId('product-description-input').fill('Updated description')

        await page.getByTestId('product-form-submit-button').click()

        // Verify success toast
        await expect(
          page.getByText(/Product "Test Product A Updated" updated successfully/)
        ).toBeVisible()

        // Verify updated values in table (use specific row locator to avoid matching toast)
        const productARow = page.locator(`tr:has-text("${productACode}")`)
        await expect(productARow.getByText('Test Product A Updated')).toBeVisible()
        console.log('[Test] Product edited successfully')
      })

      // ------------------------------------------------------------------
      // Step 8: Disable Product (US-PR-004 Scene 1)
      // ------------------------------------------------------------------
      await test.step('Step 8: Disable Product (US-PR-004 Scene 1)', async () => {
        await openEditProductDialog(page, productBCode)

        // Toggle enabled switch OFF
        await page.getByTestId('product-enabled-switch').click()

        await page.getByTestId('product-form-submit-button').click()

        // Verify success toast
        await expect(
          page.getByText(/Product "Test Product B" updated successfully/)
        ).toBeVisible()

        // Verify status badge changed to "Disabled"
        const productBRow = page.locator(`tr:has-text("${productBCode}")`)
        await expect(productBRow.getByText('Disabled')).toBeVisible()
        console.log('[Test] Product disabled successfully')
      })

      // ------------------------------------------------------------------
      // Step 9: Re-enable Product (US-PR-004 Scene 2)
      // ------------------------------------------------------------------
      await test.step('Step 9: Re-enable Product (US-PR-004 Scene 2)', async () => {
        // Wait for old toasts from Step 8 to disappear before opening edit dialog
        // This prevents duplicate toast detection (Step 8 and Step 9 both show "Test Product B updated successfully")
        await waitForOldToastsToDisappear(page, /Product "Test Product B" updated successfully/)

        await openEditProductDialog(page, productBCode)

        // Toggle enabled switch back ON
        await page.getByTestId('product-enabled-switch').click()

        await page.getByTestId('product-form-submit-button').click()

        // Verify success toast (use .first() to ensure we only match the newest toast)
        await expect(
          page.getByText(/Product "Test Product B" updated successfully/).first()
        ).toBeVisible()

        // Verify status badge reverted to "Enabled"
        const productBRow = page.locator(`tr:has-text("${productBCode}")`)
        await expect(productBRow.getByText('Enabled')).toBeVisible()
        console.log('[Test] Product re-enabled successfully')
      })

      // ------------------------------------------------------------------
      // Step 10: Delete Product Without Plans (US-PR-005 Scene 1 + Scene 4)
      // ------------------------------------------------------------------
      await test.step('Step 10: Delete Product Without Plans (US-PR-005 Scene 1+4)', async () => {
        // Open delete dialog for product-b (has 0 plans)
        await openDeleteProductDialog(page, productBCode)

        // US-PR-005 Scene 4: Verify confirmation dialog content
        await expect(
          page.getByText(`Are you sure you want to delete product "${productBTitle}"?`)
        ).toBeVisible()

        // Delete button should be enabled (no plans blocking)
        const deleteButton = page.getByTestId('product-delete-confirm-button')
        await expect(deleteButton).toBeEnabled()

        // Confirm deletion
        await confirmDeleteProduct(page)

        // Verify success toast
        await expect(
          page.getByText(/Product "Test Product B" deleted successfully/)
        ).toBeVisible()

        // Verify product no longer appears in table
        await verifyProductNotInTable(page, productBCode)
        console.log('[Test] Product deleted successfully')
      })

      // ------------------------------------------------------------------
      // Cleanup: Delete product-a via UI
      // ------------------------------------------------------------------
      await test.step('Cleanup: Delete remaining test products', async () => {
        try {
          await openDeleteProductDialog(page, productACode)
          await confirmDeleteProductAndWait(page)
          console.log('[Test] Cleanup: product-a deleted')
        } catch {
          console.log('[Test] Cleanup: product-a already removed or not found')
        }
      })
    })
  })

  // ============================================================================
  // User Story US-PR-006: Plan-Product Relationship
  // ============================================================================

  test.describe('US-PR-006: Plan-Product Relationship', () => {
    test('Plan creation under Product and move between Products', async ({
      page,
      loginPage,
      demoLogger,
      testStartTime,
    }) => {
      const planHostCode = `plan-host-${testStartTime}`
      const planHostTitle = `Plan Host Product ${testStartTime}`
      const planTargetCode = `plan-target-${testStartTime}`
      const planTargetTitle = `Plan Target Product ${testStartTime}`
      const planName = `test-plan-${testStartTime}`

      // ------------------------------------------------------------------
      // Step 1: Setup - Create Products
      // ------------------------------------------------------------------
      await test.step('Step 1: Setup - Create Products', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', DEMO_ADMIN.realmId)
        await page.goto(`/${DEMO_ADMIN.realmId}/manage/products`)
        await expect(page.getByTestId('products-page')).toBeVisible()

        // Create plan-host product
        await createProduct(page, {
          code: planHostCode,
          title: planHostTitle,
        })
        await expect(
          page.getByText(new RegExp(`Product "${planHostTitle}" created successfully`))
        ).toBeVisible()

        // Create plan-target product
        await createProduct(page, {
          code: planTargetCode,
          title: planTargetTitle,
        })
        await expect(
          page.getByText(new RegExp(`Product "${planTargetTitle}" created successfully`))
        ).toBeVisible()

        console.log('[Test] Setup products created')
      })

      // ------------------------------------------------------------------
      // Step 2: Create Plan Under Product (US-PR-006 Scene 1)
      // ------------------------------------------------------------------
      await test.step('Step 2: Create Plan Under Product (US-PR-006 Scene 1)', async () => {
        // Navigate to billing plans page
        await page.goto(`/${DEMO_ADMIN.realmId}/manage/billing`)
        await expect(page.getByTestId('billing-page')).toBeVisible()

        // Create plan with product selector
        await createSubscriptionPlan(page, {
          productTitle: planHostTitle,
          planName,
          title: 'Test Plan',
          price: '1000',
          type: 'monthly',
          provider: 'creem',
          externalProductId: `ext_${testStartTime}`,
        })

        // Verify plan created and visible
        await expect(page.getByText(planName)).toBeVisible({ timeout: 5000 })
        console.log('[Test] Plan created under product')
      })

      // ------------------------------------------------------------------
      // Step 3: Verify Plan Product Selector is Required (US-PR-006 Scene 5)
      // ------------------------------------------------------------------
      await test.step('Step 3: Verify Plan Product Selector is Required (US-PR-006 Scene 5)', async () => {
        await page.getByTestId('add-plan-button').click()
        await page.waitForURL('**/manage/billing/plans/new', { timeout: 10000 })
        await expect(page.getByTestId('plan-form-page')).toBeVisible()

        // Verify product selector is visible
        await expect(page.getByTestId('plan-product-select-trigger')).toBeVisible()

        // Attempt to submit without selecting a product
        await page.getByTestId('plan-name-input').fill('no-product-plan')
        await page.getByTestId('plan-title-input').fill('No Product Plan')
        await page.getByTestId('plan-price-input').fill('500')

        // Select type
        const typeSelectTrigger = page.getByTestId('plan-type-select-trigger')
        await expect(typeSelectTrigger).toBeVisible({ timeout: 5000 })
        await typeSelectTrigger.click()
        await page.getByTestId('plan-type-monthly').click()

        // Submit without product
        await page.getByTestId('plan-form-submit-button').click()

        // Validation error should appear
        await expect(page.getByText('Product is required')).toBeVisible()

        // Navigate back to billing page via cancel button
        await page.getByTestId('plan-form-cancel-button').click()
        await page.waitForURL(/\/manage\/billing/, { timeout: 10000 })
        console.log('[Test] Product selector required validation verified')
      })

      // ------------------------------------------------------------------
      // Step 4: Move Plan Between Products (US-PR-006 Scene 3)
      // ------------------------------------------------------------------
      await test.step('Step 4: Move Plan Between Products (US-PR-006 Scene 3)', async () => {
        await openEditSubscriptionPlanDialog(page, planName)

        // Change product selector from "Plan Host Product" to "Plan Target Product"
        const productSelectTrigger = page.getByTestId('plan-product-select-trigger')
        await expect(productSelectTrigger).toBeVisible({ timeout: 5000 })
        await productSelectTrigger.click()
        await page.getByRole('option', { name: planTargetTitle }).click()

        // Submit the edit
        await page.getByTestId('plan-form-submit-button').click()

        // Verify success
        await expect(page.getByText(/Plan "Test Plan" updated successfully/)).toBeVisible()
        console.log('[Test] Plan moved between products')
      })

      // ------------------------------------------------------------------
      // Step 5: Verify Product Plans Count (US-PR-003 Scene 3)
      // ------------------------------------------------------------------
      await test.step('Step 5: Verify Product Plans Count (US-PR-003 Scene 3)', async () => {
        // Navigate back to Products page
        await page.goto(`/${DEMO_ADMIN.realmId}/manage/products`)
        await expect(page.getByTestId('products-page')).toBeVisible()

        // plan-host should show Plans count = 0
        const planHostRow = page.locator(`tr:has-text("${planHostCode}")`)
        await expect(planHostRow).toBeVisible()
        // The plans count column shows the number
        await expect(planHostRow.locator('td').nth(4)).toHaveText('0')

        // plan-target should show Plans count = 1
        const planTargetRow = page.locator(`tr:has-text("${planTargetCode}")`)
        await expect(planTargetRow).toBeVisible()
        await expect(planTargetRow.locator('td').nth(4)).toHaveText('1')

        console.log('[Test] Product plans count verified')
      })

      // ------------------------------------------------------------------
      // Step 6: Verify Product Cannot Be Deleted with Plans (US-PR-006 Scene 4)
      // ------------------------------------------------------------------
      await test.step('Step 6: Verify Product Cannot Be Deleted with Plans (US-PR-006 Scene 4)', async () => {
        // plan-target has 1 plan, should not be deletable
        await openDeleteProductDialog(page, planTargetCode)

        // Verify warning message about plans
        await expect(
          page.getByText(/cannot be deleted because it has associated plans/)
        ).toBeVisible()

        // Delete button should be disabled
        const deleteButton = page.getByTestId('product-delete-confirm-button')
        await expect(deleteButton).toBeDisabled()

        // Cancel the dialog (do not actually delete)
        await page.getByTestId('product-delete-cancel-button').click()
        await expect(page.getByTestId('product-delete-confirm-dialog')).toBeHidden()
        console.log('[Test] Delete blocked with plans verified')
      })

      // ------------------------------------------------------------------
      // Cleanup: Delete plan and products via UI
      // ------------------------------------------------------------------
      await test.step('Cleanup: Delete plan and products', async () => {
        try {
          // Navigate to billing to delete the plan first
          await page.goto(`/${DEMO_ADMIN.realmId}/manage/billing`)
          await expect(page.getByTestId('billing-page')).toBeVisible()

          await openDeleteSubscriptionPlanDialog(page, planName)
          await confirmDeleteSubscriptionPlan(page)
          console.log('[Test] Cleanup: plan deleted')

          // Now delete products
          await page.goto(`/${DEMO_ADMIN.realmId}/manage/products`)
          await expect(page.getByTestId('products-page')).toBeVisible()

          // Use confirmDeleteProductAndWait to prevent race conditions
          await openDeleteProductDialog(page, planHostCode)
          await confirmDeleteProductAndWait(page)
          console.log('[Test] Cleanup: plan-host product deleted')

          await openDeleteProductDialog(page, planTargetCode)
          await confirmDeleteProductAndWait(page)
          console.log('[Test] Cleanup: plan-target product deleted')
        } catch {
          console.log('[Test] Cleanup: some items already removed or not found')
        }
      })
    })
  })

  // ============================================================================
  // User Story US-PR-001: Product Form Validation
  // ============================================================================

  test.describe('US-PR-001: Product Form Validation', () => {
    test('Product form validation edge cases', async ({
      page,
      loginPage,
      demoLogger,
      testStartTime,
    }) => {
      await test.step('Given: Admin is logged in and on Products page', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', DEMO_ADMIN.realmId)
        await page.goto(`/${DEMO_ADMIN.realmId}/manage/products`)
        await expect(page.getByTestId('products-page')).toBeVisible()
      })

      // ------------------------------------------------------------------
      // Step 1: Code Too Short
      // ------------------------------------------------------------------
      await test.step('Code Too Short (less than 3 characters)', async () => {
        await page.getByTestId('add-product-button').click()
        await expect(page.getByRole('dialog')).toBeVisible()

        await page.getByTestId('product-code-input').fill('ab')
        await page.getByTestId('product-title-input').fill('Short Code Test')

        // Trigger validation by submitting
        await page.getByTestId('product-form-submit-button').click()

        // Verify inline validation error
        await expect(
          page.getByText('Product code must be at least 3 characters')
        ).toBeVisible()
        console.log('[Test] Code too short validation verified')
      })

      // ------------------------------------------------------------------
      // Step 2: Code Too Long
      // ------------------------------------------------------------------
      await test.step('Code Too Long (exceeds 50 characters)', async () => {
        const longCode = 'a'.repeat(51)
        await page.getByTestId('product-code-input').fill(longCode)

        // Trigger validation
        await page.getByTestId('product-form-submit-button').click()

        // Verify inline validation error
        await expect(
          page.getByText('Product code must not exceed 50 characters')
        ).toBeVisible()
        console.log('[Test] Code too long validation verified')
      })

      // ------------------------------------------------------------------
      // Step 3: Code with Spaces
      // ------------------------------------------------------------------
      await test.step('Code with Spaces', async () => {
        await page.getByTestId('product-code-input').fill('has spaces')
        await page.getByTestId('product-title-input').fill('Space Code Test')

        // Trigger validation
        await page.getByTestId('product-form-submit-button').click()

        // Verify inline validation error
        await expect(
          page.getByText(/Product code can only contain lowercase letters/)
        ).toBeVisible()
        console.log('[Test] Code with spaces validation verified')
      })

      // ------------------------------------------------------------------
      // Step 4: Code with Special Characters
      // ------------------------------------------------------------------
      await test.step('Code with Special Characters', async () => {
        await page.getByTestId('product-code-input').fill('special!@#')
        await page.getByTestId('product-title-input').fill('Special Char Test')

        // Trigger validation
        await page.getByTestId('product-form-submit-button').click()

        // Verify inline validation error
        await expect(
          page.getByText(/Product code can only contain lowercase letters/)
        ).toBeVisible()
        console.log('[Test] Code with special chars validation verified')
      })

      // ------------------------------------------------------------------
      // Step 5: Empty Title
      // ------------------------------------------------------------------
      await test.step('Empty Title', async () => {
        // Fill valid code but clear title
        await page.getByTestId('product-code-input').fill('valid-code-test')
        await page.getByTestId('product-title-input').clear()

        // Trigger validation
        await page.getByTestId('product-form-submit-button').click()

        // Verify title required error
        await expect(page.getByText(/Title is required/)).toBeVisible()
        console.log('[Test] Empty title validation verified')
      })

      // ------------------------------------------------------------------
      // Step 6: Close dialog after validation tests
      // ------------------------------------------------------------------
      await test.step('Close dialog', async () => {
        await page.getByTestId('product-form-cancel-button').click()
        await expect(page.getByRole('dialog')).toBeHidden()
        console.log('[Test] Dialog closed after validation tests')
      })

      // No cleanup needed - no data was successfully created
    })
  })
})
