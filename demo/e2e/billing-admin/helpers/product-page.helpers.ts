import { Page, expect } from '@playwright/test'
import { clickRowMenuItem } from './table-actions.helpers'

export interface ProductFormData {
  name: string
  title: string
  description?: string
  sortOrder?: number
  enabled?: boolean
}

export async function createProduct(page: Page, formData: ProductFormData): Promise<void> {
  await page.getByTestId('add-product-button').click()
  await expect(page.getByRole('dialog')).toBeVisible()

  await expect(page.getByTestId('product-name-input')).toBeVisible()

  await page.getByTestId('product-name-input').fill(formData.name)
  await page.getByTestId('product-title-input').fill(formData.title)

  if (formData.description) {
    await page.getByTestId('product-description-input').fill(formData.description)
  }

  if (formData.sortOrder !== undefined) {
    await page.getByTestId('product-sort-order-input').fill(formData.sortOrder.toString())
  }

  if (formData.enabled === false) {
    await page.getByTestId('product-enabled-switch').click()
  }

  await page.getByTestId('product-form-submit-button').click()

  await expect(page.getByTestId('product-form-dialog')).toBeHidden({ timeout: 5000 })
}

export async function openEditProductDialog(page: Page, productName: string): Promise<void> {
  await clickRowMenuItem(page, productName, 'Edit')
  await expect(page.getByRole('dialog')).toBeVisible()
}

export async function openDeleteProductDialog(page: Page, productName: string): Promise<void> {
  await clickRowMenuItem(page, productName, 'Delete')
  await expect(page.getByTestId('product-delete-confirm-dialog')).toBeVisible()
}

export async function confirmDeleteProduct(page: Page): Promise<void> {
  await page.getByTestId('product-delete-confirm-button').click()
  await expect(page.getByTestId('product-delete-confirm-dialog')).toBeHidden({ timeout: 5000 })
}

/**
 * Confirm product deletion and wait for page to stabilize.
 * This prevents race conditions when deleting multiple products in sequence.
 */
export async function confirmDeleteProductAndWait(page: Page): Promise<void> {
  await page.getByTestId('product-delete-confirm-button').click()
  await expect(page.getByTestId('product-delete-confirm-dialog')).toBeHidden({ timeout: 5000 })

  // Wait for page to stabilize after deletion
  await page.waitForLoadState('networkidle', { timeout: 10000 })

  // Additional safety margin for UI to fully refresh
  // This is a technical delay to ensure the product list is fully updated
  await page.waitForTimeout(300)
}

export async function verifyProductInTable(page: Page, productName: string): Promise<void> {
  await expect(page.locator(`tr:has-text("${productName}")`)).toBeVisible()
}

export async function verifyProductNotInTable(page: Page, productName: string): Promise<void> {
  await expect(page.locator(`tr:has-text("${productName}")`)).not.toBeVisible()
}
