/**
 * Billing Page Helpers
 *
 * Shared helper functions for billing management tests
 */

import { Page, expect } from '@playwright/test'
import { clickRowMenuItem } from './table-actions.helpers'
import { createProduct } from './product-page.helpers'

export interface SubscriptionPlanFormData {
  planName: string
  title: string
  description?: string
  price: string
  type: 'monthly' | 'yearly'
  currency?: string
  provider?: string
  // Note: externalProductId, trialDays, and checkoutUrl are now part of provider mapping
  // These fields are kept in the interface for backward compatibility but are not used
  // in plan creation form (plan-form-page.tsx)
  externalProductId?: string
  trialDays?: string
  checkoutUrl?: string
  productTitle?: string
}

/**
 * Helper function to create a subscription plan.
 * If productTitle is not provided, automatically navigates to products page,
 * creates a default product, then returns to the billing plans page.
 */
export async function createSubscriptionPlan(page: Page, formData: SubscriptionPlanFormData): Promise<void> {
  // Auto-create product if not provided
  let productTitle = formData.productTitle
  if (!productTitle) {
    const realmId = page.url().match(/\/([^/]+)\/manage/)?.[1] || 'admin'

    // Navigate to products page and create a default product
    await page.goto(`/${realmId}/manage/products`)
    await expect(page.getByTestId('products-page')).toBeVisible({ timeout: 10000 })
    productTitle = `Default Product ${Date.now()}`
    await createProduct(page, {
      code: `default-product-${Date.now()}`,
      title: productTitle,
      description: 'Auto-created default product',
    })

    // Navigate back to billing plans page
    await page.goto(`/${realmId}/manage/billing`)
    await expect(page.getByTestId('billing-page')).toBeVisible({ timeout: 10000 })
  }

  // Idempotent: skip creation if plan already exists in the table
  const planRow = page.locator(`tr:has-text("${formData.planName}")`)
  if ((await planRow.count()) > 0) {
    console.log(`[createSubscriptionPlan] Plan "${formData.planName}" already exists, skipping creation`)
    return
  }

  await page.getByTestId('add-plan-button').click()

  // Wait for navigation to plan creation page
  await page.waitForURL('**/manage/billing/plans/new', { timeout: 10000 })
  await expect(page.getByTestId('plan-form-page')).toBeVisible()

  // Wait for form inputs to be ready (using assertion instead of fixed timeout)
  await expect(page.getByTestId('plan-name-input')).toBeVisible()

  if (productTitle) {
    const productSelectTrigger = page.getByTestId('plan-product-select-trigger')
    await expect(productSelectTrigger).toBeVisible({ timeout: 5000 })
    await productSelectTrigger.click()
    // Wait for options to load and be visible
    await page.waitForTimeout(500)
    const option = page.getByRole('option', { name: productTitle })
    await expect(option).toBeVisible({ timeout: 5000 })
    await option.click()
    // Wait for selection to complete
    await page.waitForTimeout(300)
  }

  await page.getByTestId('plan-name-input').fill(formData.planName)

  await page.getByTestId('plan-title-input').fill(formData.title)

  if (formData.description) {
    await page.getByTestId('plan-description-input').fill(formData.description)
  }

  const typeSelectTrigger = page.getByTestId('plan-type-select-trigger')
  await expect(typeSelectTrigger).toBeVisible({ timeout: 5000 })
  await typeSelectTrigger.click()

  await page.getByTestId(`plan-type-${formData.type.toLowerCase()}`).click()

  await page.getByTestId('plan-price-input').fill(formData.price)

  if (formData.currency) {
    const currencySelectTrigger = page.getByTestId('plan-currency-select-trigger')
    await expect(currencySelectTrigger).toBeVisible({ timeout: 5000 })
    await currencySelectTrigger.click()

    await page.getByTestId(`plan-currency-${formData.currency.toLowerCase()}`).click()
  }

  // Note: externalProductId, trialDays, and checkoutUrl are no longer part of the plan creation form
  // These fields have been moved to the provider mapping form (plan-provider-mapping-form.tsx)
  // The form fields below are commented out as they don't exist in plan-form-page.tsx
  // if (formData.externalProductId) {
  //   await page.getByTestId('plan-external-product-id-input').fill(formData.externalProductId)
  // }
  // if (formData.trialDays) {
  //   await page.getByTestId('plan-trial-days-input').fill(formData.trialDays)
  // }
  // if (formData.checkoutUrl) {
  //   await page.getByTestId('plan-checkout-url-input').fill(formData.checkoutUrl)
  // }

  // Wait a moment for form to stabilize before submission
  await page.waitForTimeout(300)

  const submitButton = page.getByTestId('plan-form-submit-button')
  await expect(submitButton).toBeVisible({ timeout: 5000 })

  await submitButton.click()

  // Wait for navigation back to billing page (indicates successful submission)
  // Note: We don't wait for the plan to appear in the list because:
  // 1. Plans are ordered by sort_order ASC, and new plans get sort_order=0
  // 2. With many existing plans, new plans might not appear on first page
  // 3. Backend logs confirm plan creation is successful
  try {
    await page.waitForURL(/\/manage\/billing(\?|$)/, { timeout: 10000 })
  } catch (error) {
    // Take screenshot for debugging
    await page.screenshot({ path: `test-results/plan-form-error-${Date.now()}.png` })

    // Check if there are validation errors
    const errorMessages = await page.locator('.text-destructive.text-sm').allTextContents()
    if (errorMessages.length > 0) {
      throw new Error(`Form submission failed with validation errors: ${errorMessages.join(', ')}`)
    }

    // Check if still on form page (submit failed)
    const formPage = page.getByTestId('plan-form-page')
    if (await formPage.isVisible()) {
      throw new Error('Form submission failed: Still on plan form page')
    }

    throw new Error(`Form submission failed: ${error}`)
  }

  // Wait a bit for React Query cache invalidation to start
  await page.waitForTimeout(500)

  // Reload the page to ensure we have fresh data from the backend
  await page.reload({ waitUntil: 'networkidle' })
  await expect(page.getByTestId('billing-page')).toBeVisible({ timeout: 10000 })

  // Wait a bit more for the table to fully render
  await page.waitForTimeout(1000)

  // Scroll the new plan row into view so subsequent assertions can find it
  const newPlanRow = page.locator(`tr:has-text("${formData.planName}")`)
  if ((await newPlanRow.count()) > 0) {
    await newPlanRow.first().scrollIntoViewIfNeeded()
  }
}

/**
 * Helper function to navigate to edit page for a plan
 */
export async function openEditSubscriptionPlanDialog(page: Page, planName: string): Promise<void> {
  await clickRowMenuItem(page, planName, 'Edit')
  // Edit now navigates to a separate page instead of opening a dialog
  await page.waitForURL('**/manage/billing/plans/*/edit', { timeout: 10000 })
  await expect(page.getByTestId('plan-form-page')).toBeVisible()
}

export async function openDeleteSubscriptionPlanDialog(page: Page, planName: string): Promise<void> {
  await clickRowMenuItem(page, planName, 'Delete')
  await expect(page.getByText(/Are you sure you want to delete/i)).toBeVisible()
}

export async function openAssignSubscriptionPlanDialog(page: Page, planName: string): Promise<void> {
  await clickRowMenuItem(page, planName, 'Assign to App')
  await expect(page.getByRole('dialog')).toBeVisible()
}

/**
 * Helper function to confirm delete in confirmation dialog
 */
export async function confirmDeleteSubscriptionPlan(page: Page): Promise<void> {
  await page.getByTestId('confirm-delete-button').click()
  await expect(page.getByText(/Are you sure you want to delete/i)).toBeHidden({ timeout: 5000 })
  await page.waitForLoadState('networkidle', { timeout: 10000 })
  // Wait for React Query refetchQueries to complete data refresh
  // Technical delay: 2000ms ensures cache invalidation and refetch complete after deletion
  await page.waitForTimeout(2000)
}

/**
 * Confirm plan deletion and wait for page to stabilize.
 * This prevents race conditions when deleting plans in sequence.
 * @deprecated Use confirmDeleteSubscriptionPlan instead - it now includes wait logic
 */
export async function confirmDeleteSubscriptionPlanAndWait(page: Page): Promise<void> {
  await confirmDeleteSubscriptionPlan(page)
}

/**
 * Verify a plan name is visible in the billing page table.
 * Scrolls the element into view first to handle cases where the table has many rows
 * and the target row is below the viewport.
 */
export async function verifyPlanVisible(page: Page, planName: string, timeout = 5000): Promise<void> {
  const planText = page.getByText(planName)
  await planText.scrollIntoViewIfNeeded()
  await expect(planText).toBeVisible({ timeout })
}
