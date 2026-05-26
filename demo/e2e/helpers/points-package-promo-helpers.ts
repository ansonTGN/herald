/**
 * Promo Package Test Helpers
 *
 * Shared helpers and types for promotional points package demo tests.
 * Covers admin CRUD operations and user purchase page interactions
 * for promotional packages (discounted pricing with time-limited validity).
 */

import { Page, expect } from '@playwright/test'
import { SELECTORS } from '../selectors'

// ============================================================================
// Types
// ============================================================================

/**
 * Form data for creating/editing a promotional points package.
 * All fields are required for creation; edits use Partial via editPromoPackage.
 */
export interface PromoPackageFormData {
  name: string
  title: string
  points: number
  price: string
  currency: string
  sortOrder: number
  enabled: boolean
  originalPrice: string
  promoStartTime: string
  promoEndTime: string
}

/**
 * Time range for promotional packages.
 * Values are ISO strings suitable for datetime-local input elements.
 */
export interface PromoTimeRange {
  startTime: string
  endTime: string
}

// ============================================================================
// Time Generators
// ============================================================================

/**
 * Generate a promo time range starting now and ending after the given duration.
 *
 * @param durationDays - Number of days the promotion should last
 * @returns PromoTimeRange with ISO datetime-local formatted strings
 */
export function generatePromoTimeRange(durationDays: number): PromoTimeRange {
  const now = new Date()
  const end = new Date(now)
  end.setDate(end.getDate() + durationDays)

  return {
    startTime: toDatetimeLocal(now),
    endTime: toDatetimeLocal(end),
  }
}

/**
 * Generate an expired promo time range where the end time is in the past.
 *
 * @returns PromoTimeRange with startTime and endTime both in the past
 */
export function generateExpiredPromoTimeRange(): PromoTimeRange {
  const now = new Date()
  const start = new Date(now)
  start.setDate(start.getDate() - 10)
  const end = new Date(now)
  end.setDate(end.getDate() - 1)

  return {
    startTime: toDatetimeLocal(start),
    endTime: toDatetimeLocal(end),
  }
}

/**
 * Convert a Date to a datetime-local input value string (YYYY-MM-DDTHH:mm).
 */
function toDatetimeLocal(date: Date): string {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  const hours = String(date.getHours()).padStart(2, '0')
  const minutes = String(date.getMinutes()).padStart(2, '0')
  return `${year}-${month}-${day}T${hours}:${minutes}`
}

// ============================================================================
// Admin Helpers
// ============================================================================

/**
 * Navigate to the admin points packages list page.
 *
 * @param page - Playwright Page object
 * @param realmId - The realm identifier
 */
export async function navigateToPointsPackagesAdmin(
  page: Page,
  realmId: string
): Promise<void> {
  await page.goto(`/${realmId}/manage/points-packages`)
  await expect(page.locator(SELECTORS.pointsPackages.page)).toBeVisible()
}

/**
 * Create a promotional points package via the admin form.
 *
 * Navigates to the create page, fills all base fields (name, title, points,
 * price, currency, sortOrder, enabled), selects the "promotional" package type,
 * fills promo-specific fields (originalPrice, promoStartTime, promoEndTime),
 * then submits the form and waits for redirect back to the list page.
 *
 * @param page - Playwright Page object
 * @param realmId - The realm identifier
 * @param data - The promotional package form data
 */
export async function createPromoPackage(
  page: Page,
  realmId: string,
  data: PromoPackageFormData
): Promise<void> {
  // Navigate to create page
  await navigateToPointsPackagesAdmin(page, realmId)
  await page.locator(SELECTORS.pointsPackages.addButton).click()
  await page.waitForURL('**/manage/points-packages/new', { timeout: 10000 })
  await expect(page.locator(SELECTORS.pointsPackageForm.page)).toBeVisible()

  // Fill base fields
  await page.locator(SELECTORS.pointsPackageForm.nameInput).fill(data.name)
  await page.locator(SELECTORS.pointsPackageForm.titleInput).fill(data.title)
  await page.locator(SELECTORS.pointsPackageForm.pointsInput).fill(
    data.points.toString()
  )
  await page.locator(SELECTORS.pointsPackageForm.priceInput).fill(data.price)
  await page.locator(SELECTORS.pointsPackageForm.currencySelect).fill(
    data.currency
  )
  await page.locator(SELECTORS.pointsPackageForm.sortOrderInput).fill(
    data.sortOrder.toString()
  )

  // Set enabled switch
  const isEnabled = await page
    .locator(SELECTORS.pointsPackageForm.enabledSwitch)
    .isChecked()
  if (data.enabled && !isEnabled) {
    await page.locator(SELECTORS.pointsPackageForm.enabledSwitch).click()
  } else if (!data.enabled && isEnabled) {
    await page.locator(SELECTORS.pointsPackageForm.enabledSwitch).click()
  }

  // Select "promotional" package type
  await page.locator(SELECTORS.pointsPackageForm.packageTypePromotional).click()

  // Fill promo-specific fields
  await page.locator(SELECTORS.pointsPackageForm.originalPriceInput).fill(
    data.originalPrice
  )
  await page.locator(SELECTORS.pointsPackageForm.promoStartInput).fill(
    data.promoStartTime
  )
  await page.locator(SELECTORS.pointsPackageForm.promoEndInput).fill(
    data.promoEndTime
  )

  // Submit form
  await page.locator(SELECTORS.pointsPackageForm.submitButton).click()

  // Wait for redirect back to list page
  await page.waitForURL('**/manage/points-packages', { timeout: 10000 })
  await expect(page.locator(SELECTORS.pointsPackages.table)).toBeVisible()
}

/**
 * Edit an existing promotional points package.
 *
 * Navigates to the edit page for the given package, updates the specified
 * fields, and submits the form.
 *
 * @param page - Playwright Page object
 * @param realmId - The realm identifier
 * @param packageId - The package UUID
 * @param updates - Partial form data with fields to update
 */
export async function editPromoPackage(
  page: Page,
  realmId: string,
  packageId: string,
  updates: Partial<PromoPackageFormData>
): Promise<void> {
  // Navigate to edit page
  await navigateToPointsPackagesAdmin(page, realmId)
  await page
    .locator(SELECTORS.pointsPackages.editButton(packageId))
    .click()
  await page.waitForURL(
    `**/manage/points-packages/${packageId}/edit`,
    { timeout: 10000 }
  )
  await expect(page.locator(SELECTORS.pointsPackageForm.page)).toBeVisible()

  // Update base fields if provided
  if (updates.name !== undefined) {
    await page.locator(SELECTORS.pointsPackageForm.nameInput).clear()
    await page.locator(SELECTORS.pointsPackageForm.nameInput).fill(updates.name)
  }
  if (updates.title !== undefined) {
    await page.locator(SELECTORS.pointsPackageForm.titleInput).clear()
    await page
      .locator(SELECTORS.pointsPackageForm.titleInput)
      .fill(updates.title)
  }
  if (updates.points !== undefined) {
    await page.locator(SELECTORS.pointsPackageForm.pointsInput).clear()
    await page
      .locator(SELECTORS.pointsPackageForm.pointsInput)
      .fill(updates.points.toString())
  }
  if (updates.price !== undefined) {
    await page.locator(SELECTORS.pointsPackageForm.priceInput).clear()
    await page.locator(SELECTORS.pointsPackageForm.priceInput).fill(updates.price)
  }
  if (updates.currency !== undefined) {
    await page.locator(SELECTORS.pointsPackageForm.currencySelect).fill(
      updates.currency
    )
  }
  if (updates.sortOrder !== undefined) {
    await page.locator(SELECTORS.pointsPackageForm.sortOrderInput).clear()
    await page
      .locator(SELECTORS.pointsPackageForm.sortOrderInput)
      .fill(updates.sortOrder.toString())
  }
  if (updates.enabled !== undefined) {
    const isEnabled = await page
      .locator(SELECTORS.pointsPackageForm.enabledSwitch)
      .isChecked()
    if (updates.enabled && !isEnabled) {
      await page.locator(SELECTORS.pointsPackageForm.enabledSwitch).click()
    } else if (!updates.enabled && isEnabled) {
      await page.locator(SELECTORS.pointsPackageForm.enabledSwitch).click()
    }
  }

  // Update promo-specific fields if provided
  if (updates.originalPrice !== undefined) {
    await page.locator(SELECTORS.pointsPackageForm.originalPriceInput).clear()
    await page
      .locator(SELECTORS.pointsPackageForm.originalPriceInput)
      .fill(updates.originalPrice)
  }
  if (updates.promoStartTime !== undefined) {
    await page.locator(SELECTORS.pointsPackageForm.promoStartInput).fill(
      updates.promoStartTime
    )
  }
  if (updates.promoEndTime !== undefined) {
    await page.locator(SELECTORS.pointsPackageForm.promoEndInput).fill(
      updates.promoEndTime
    )
  }

  // Submit form
  await page.locator(SELECTORS.pointsPackageForm.submitButton).click()

  // Wait for redirect back to list page
  await page.waitForURL('**/manage/points-packages', { timeout: 10000 })
  await expect(page.locator(SELECTORS.pointsPackages.table)).toBeVisible()
}

/**
 * Find a package row in the admin list by its name.
 *
 * @param page - Playwright Page object
 * @param name - The package name to search for
 * @returns The locator for the matching table row
 */
export function findPackageRowByName(page: Page, name: string) {
  return page.locator('tr').filter({ hasText: name })
}

// ============================================================================
// User Purchase Helpers
// ============================================================================

/**
 * Navigate to the user purchase points page.
 *
 * @param page - Playwright Page object
 * @param realmId - The realm identifier
 */
export async function navigateToPurchasePage(
  page: Page,
  realmId: string
): Promise<void> {
  await page.goto(`/${realmId}/user/purchase-points`)
  await expect(
    page.locator(SELECTORS.packageSelector.container)
  ).toBeVisible()
}

/**
 * Find a promotional package card on the user purchase page by its title.
 *
 * @param page - Playwright Page object
 * @param title - The package title to search for
 * @returns The locator for the matching package card
 */
export function findPromoCardByTitle(page: Page, title: string) {
  return page
    .locator(SELECTORS.packageSelector.container)
    .locator('[data-testid^="points-package-card-"]')
    .filter({ hasText: title })
}
