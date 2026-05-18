/**
 * Invoice Admin Helpers
 *
 * Shared helper functions for admin invoice lifecycle demo tests.
 * Covers: create, edit, issue, void, mark-paid, and table verification.
 *
 * All amounts are in smallest currency unit (cents).
 * unitPrice in the form is an integer (cents).
 * Fee value inputs are in display/major-currency units (e.g. "5" means 5.00 = 500 cents for fixed,
 * "6" means 6% for percent).
 */

import { Page, expect } from '@playwright/test'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface InvoiceLineItemData {
  name: string
  quantity?: string   // default '1'
  unitPrice?: number  // in cents, default 0
}

export interface InvoiceCreateData {
  accountId: string
  billingName: string
  billingEmail?: string
  billingTaxId?: string
  sellerName: string
  sellerEmail?: string
  sellerTaxId?: string
  lineItems: InvoiceLineItemData[]
  dueDate: string           // ISO date string, e.g. '2026-12-31'
  discountMode?: 'fixed' | 'percent' | null
  discountValue?: string | null
  taxMode?: 'fixed' | 'percent' | null
  taxValue?: string | null
  shippingMode?: 'fixed' | null
  shippingValue?: string | null
}

export interface InvoiceEditChanges {
  billingName?: string
  sellerName?: string
  lineItems?: InvoiceLineItemData[]
  dueDate?: string
  discountMode?: 'fixed' | 'percent' | null
  discountValue?: string | null
  taxMode?: 'fixed' | 'percent' | null
  taxValue?: string | null
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

/**
 * Navigate to the admin invoices page for the given realm.
 */
export async function navigateToInvoiceAdminPage(
  page: Page,
  realmId: string,
): Promise<void> {
  await page.goto(`/${realmId}/manage/billing/invoices`)
  await expect(page.getByTestId('invoice-admin-page')).toBeVisible({ timeout: 10000 })
}

// ---------------------------------------------------------------------------
// Create Invoice
// ---------------------------------------------------------------------------

/**
 * Create an invoice draft by navigating to the create page, filling the form, and submitting.
 *
 * Prerequisite: user is on the invoice admin page.
 * Returns: the auto-generated invoice number visible in the table after creation.
 */
export async function createInvoice(
  page: Page,
  realmId: string,
  data: InvoiceCreateData,
): Promise<string> {
  // Navigate to the create page
  await page.getByTestId('create-invoice-button').click()
  await expect(page.getByTestId('invoice-form-page')).toBeVisible({ timeout: 5000 })

  // Account ID (create mode only)
  await page.getByTestId('invoice-account-id').fill(data.accountId)

  // Buyer info
  await page.getByTestId('invoice-billing-name').fill(data.billingName)
  if (data.billingEmail) {
    await page.getByTestId('invoice-billing-email').fill(data.billingEmail)
  }
  await page.getByTestId('invoice-billing-tax-id').fill(data.billingTaxId ?? 'N/A')

  // Seller info
  await page.getByTestId('invoice-seller-name').fill(data.sellerName)
  if (data.sellerEmail) {
    await page.getByTestId('invoice-seller-email').fill(data.sellerEmail)
  }
  await page.getByTestId('invoice-seller-tax-id').fill(data.sellerTaxId ?? 'N/A')

  // Fill the first line item (always present with default values)
  const firstItem = data.lineItems[0]
  await page.getByTestId('invoice-line-item-name-0').fill(firstItem.name)
  if (firstItem.quantity !== undefined) {
    await page.getByTestId('invoice-line-item-quantity-0').fill(firstItem.quantity)
  }
  if (firstItem.unitPrice !== undefined) {
    await page.getByTestId('invoice-line-item-unit-price-0').fill((firstItem.unitPrice / 100).toFixed(2))
  }

  // Add additional line items if needed
  for (let i = 1; i < data.lineItems.length; i++) {
    await page.getByTestId('invoice-add-line-item').click()
    const item = data.lineItems[i]
    await page.getByTestId(`invoice-line-item-name-${i}`).fill(item.name)
    if (item.quantity !== undefined) {
      await page.getByTestId(`invoice-line-item-quantity-${i}`).fill(item.quantity)
    }
    if (item.unitPrice !== undefined) {
      await page.getByTestId(`invoice-line-item-unit-price-${i}`).fill((item.unitPrice / 100).toFixed(2))
    }
  }

  // Fees: discount
  if (data.discountMode) {
    await selectFeeMode(page, 'invoice-discount', data.discountMode)
    if (data.discountValue != null) {
      await page.getByTestId('invoice-discount-value').fill(data.discountValue)
    }
  }

  // Fees: tax
  if (data.taxMode) {
    await selectFeeMode(page, 'invoice-tax', data.taxMode)
    if (data.taxValue != null) {
      await page.getByTestId('invoice-tax-value').fill(data.taxValue)
    }
  }

  // Fees: shipping
  if (data.shippingMode) {
    await selectFeeMode(page, 'invoice-shipping', data.shippingMode)
    if (data.shippingValue != null) {
      await page.getByTestId('invoice-shipping-value').fill(data.shippingValue)
    }
  }

  // Due date
  await page.getByTestId('invoice-due-date').fill(data.dueDate)

  // Submit
  await page.getByTestId('invoice-form-submit-button').click()

  // Wait for navigation back to the invoices list page
  await page.waitForURL(`**/${realmId}/manage/billing/invoices`, { timeout: 10000 })
  await expect(page.getByTestId('invoice-admin-page')).toBeVisible({ timeout: 10000 })

  // Extract invoice number from the table row that contains the billingName
  const row = page.locator('tr').filter({ hasText: data.billingName }).first()
  await expect(row).toBeVisible({ timeout: 10000 })
  const invoiceNumber = await row.locator('td').nth(1).textContent()
  if (!invoiceNumber) {
    throw new Error('Could not extract invoice number from table')
  }
  return invoiceNumber.trim()
}

// ---------------------------------------------------------------------------
// Edit Invoice
// ---------------------------------------------------------------------------

/**
 * Navigate to the edit page for a draft invoice by its invoice number, apply changes, and save.
 *
 * Prerequisite: user is on the invoice admin page and the invoice is in draft status.
 */
export async function editInvoice(
  page: Page,
  realmId: string,
  invoiceNumber: string,
  changes: InvoiceEditChanges,
): Promise<void> {
  // Open edit page via row action menu
  const row = page.locator('tr').filter({ hasText: invoiceNumber }).first()
  await expect(row).toBeVisible({ timeout: 5000 })

  // Click the actions menu button in this row
  const menuButton = row.getByRole('button', { name: 'Open menu' })
  await menuButton.click()
  await page.getByRole('menuitem', { name: 'Edit' }).click()

  // Wait for edit page to load
  await expect(page.getByTestId('invoice-form-page')).toBeVisible({ timeout: 5000 })

  // Apply changes
  if (changes.billingName) {
    await page.getByTestId('invoice-billing-name').clear()
    await page.getByTestId('invoice-billing-name').fill(changes.billingName)
  }

  if (changes.sellerName) {
    await page.getByTestId('invoice-seller-name').clear()
    await page.getByTestId('invoice-seller-name').fill(changes.sellerName)
  }

  if (changes.lineItems) {
    // Determine how many line items already exist in the form
    const existingCount = await page.locator('[data-testid^="invoice-line-item-name-"]').count()

    for (let i = 0; i < changes.lineItems.length; i++) {
      // Add new line items if the changes array is longer than the form
      if (i >= existingCount) {
        await page.getByTestId('invoice-add-line-item').click()
        await page.waitForTimeout(200)
      }

      const item = changes.lineItems[i]
      const nameInput = page.getByTestId(`invoice-line-item-name-${i}`)
      await nameInput.fill(item.name)
      await expect(nameInput).toHaveValue(item.name)

      if (item.quantity !== undefined) {
        const quantityInput = page.getByTestId(`invoice-line-item-quantity-${i}`)
        await quantityInput.fill(item.quantity)
        await expect(quantityInput).toHaveValue(item.quantity)
      }
      if (item.unitPrice !== undefined) {
        const unitPriceStr = (item.unitPrice / 100).toFixed(2)
        const unitPriceInput = page.getByTestId(`invoice-line-item-unit-price-${i}`)
        await unitPriceInput.fill(unitPriceStr)
        await expect(unitPriceInput).toHaveValue(unitPriceStr)
      }
    }
  }

  if (changes.dueDate) {
    await page.getByTestId('invoice-due-date').fill(changes.dueDate)
  }

  if (changes.discountMode) {
    await selectFeeMode(page, 'invoice-discount', changes.discountMode)
    if (changes.discountValue != null) {
      await page.getByTestId('invoice-discount-value').clear()
      await page.getByTestId('invoice-discount-value').fill(changes.discountValue)
    }
  }

  if (changes.taxMode) {
    await selectFeeMode(page, 'invoice-tax', changes.taxMode)
    if (changes.taxValue != null) {
      await page.getByTestId('invoice-tax-value').clear()
      await page.getByTestId('invoice-tax-value').fill(changes.taxValue)
    }
  }

  // Submit
  await page.getByTestId('invoice-form-submit-button').click()

  // Wait for navigation back to the invoices list page
  await page.waitForURL(`**/${realmId}/manage/billing/invoices`, { timeout: 10000 })
  await expect(page.getByTestId('invoice-admin-page')).toBeVisible({ timeout: 10000 })
}

// ---------------------------------------------------------------------------
// Issue Invoice
// ---------------------------------------------------------------------------

/**
 * Issue a draft invoice (draft -> issued) via the row action menu.
 *
 * Prerequisite: user is on the invoice admin page and the invoice is in draft status.
 */
export async function issueInvoice(
  page: Page,
  invoiceNumber: string,
): Promise<void> {
  // Click Issue via row action menu
  const row = page.locator('tr').filter({ hasText: invoiceNumber }).first()
  await expect(row).toBeVisible({ timeout: 5000 })

  const menuButton = row.getByRole('button', { name: 'Open menu' })
  await menuButton.click()
  await page.getByRole('menuitem', { name: 'Issue' }).click()

  // Confirm in dialog
  await expect(page.getByTestId('issue-confirm-dialog')).toBeVisible({ timeout: 5000 })
  await page.getByTestId('issue-confirm-button').click()

  // Wait for dialog to close
  await expect(page.getByTestId('issue-confirm-dialog')).toBeHidden({ timeout: 10000 })
  await page.waitForLoadState('networkidle')
  // Technical delay: allow table to refresh
  await page.waitForTimeout(300)
}

// ---------------------------------------------------------------------------
// Void Invoice
// ---------------------------------------------------------------------------

/**
 * Void an invoice (draft/issued -> void) via the row action menu.
 *
 * Prerequisite: user is on the invoice admin page and the invoice is in draft or issued status.
 */
export async function voidInvoice(
  page: Page,
  invoiceNumber: string,
  reason?: string,
): Promise<void> {
  const row = page.locator('tr').filter({ hasText: invoiceNumber }).first()
  await expect(row).toBeVisible({ timeout: 5000 })

  const menuButton = row.getByRole('button', { name: 'Open menu' })
  await menuButton.click()
  await page.getByRole('menuitem', { name: 'Void' }).click()

  // Confirm in dialog
  await expect(page.getByTestId('void-confirm-dialog')).toBeVisible({ timeout: 5000 })

  if (reason) {
    await page.getByTestId('void-reason-input').fill(reason)
  }

  await page.getByTestId('void-confirm-button').click()

  // Wait for dialog to close
  await expect(page.getByTestId('void-confirm-dialog')).toBeHidden({ timeout: 10000 })
  await page.waitForLoadState('networkidle')
  // Technical delay: allow table to refresh
  await page.waitForTimeout(300)
}

// ---------------------------------------------------------------------------
// Mark as Paid
// ---------------------------------------------------------------------------

/**
 * Mark an invoice as paid (issued/overdue -> paid) via the row action menu.
 *
 * Prerequisite: user is on the invoice admin page and the invoice is in issued or overdue status.
 */
export async function markPaidInvoice(
  page: Page,
  invoiceNumber: string,
): Promise<void> {
  const row = page.locator('tr').filter({ hasText: invoiceNumber }).first()
  await expect(row).toBeVisible({ timeout: 5000 })

  const menuButton = row.getByRole('button', { name: 'Open menu' })
  await menuButton.click()
  await page.getByRole('menuitem', { name: 'Mark Paid' }).click()

  // Confirm in dialog
  await expect(page.getByTestId('mark-paid-confirm-dialog')).toBeVisible({ timeout: 5000 })
  await page.getByTestId('mark-paid-confirm-button').click()

  // Wait for dialog to close
  await expect(page.getByTestId('mark-paid-confirm-dialog')).toBeHidden({ timeout: 10000 })
  await page.waitForLoadState('networkidle')
  // Technical delay: allow table to refresh
  await page.waitForTimeout(300)
}

// ---------------------------------------------------------------------------
// Table Verification
// ---------------------------------------------------------------------------

/**
 * Verify that a specific invoice appears in the table with the expected status.
 *
 * Status labels (from InvoiceStatusBadge): Draft, Issued, Paid, Void, Overdue
 */
export async function verifyInvoiceInTable(
  page: Page,
  invoiceNumber: string,
  expectedStatus: string,
): Promise<void> {
  const row = page.locator('tr').filter({ hasText: invoiceNumber }).first()
  await expect(row).toBeVisible({ timeout: 10000 })

  // The status column contains a Badge with the status label
  // The status label is the capitalized form: Draft, Issued, Paid, Void, Overdue
  const statusLabel = expectedStatus.charAt(0).toUpperCase() + expectedStatus.slice(1).toLowerCase()
  await expect(row.getByText(statusLabel, { exact: true })).toBeVisible()
}

/**
 * Verify that a specific invoice does NOT appear in the table.
 */
export async function verifyInvoiceNotInTable(
  page: Page,
  invoiceNumber: string,
): Promise<void> {
  await expect(page.locator('tr').filter({ hasText: invoiceNumber })).not.toBeVisible()
}

// ---------------------------------------------------------------------------
// Internal Helpers
// ---------------------------------------------------------------------------

/**
 * Select a fee mode (None / Fixed / Percent) via the dropdown trigger.
 */
export async function selectFeeMode(
  page: Page,
  testIdPrefix: string,
  mode: 'fixed' | 'percent' | 'none',
): Promise<void> {
  const trigger = page.getByTestId(`${testIdPrefix}-mode-trigger`)
  await trigger.click()
  await page.getByRole('option', { name: mode === 'fixed' ? 'Fixed' : mode === 'percent' ? 'Percent' : 'None' }).click()
}
