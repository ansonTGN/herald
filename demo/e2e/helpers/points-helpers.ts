/**
 * Points System Test Helpers
 *
 * Helper functions for Points System demo tests
 * Following UI-Only principle - all operations through UI
 */

import { Page, expect } from '@playwright/test'
import { SELECTORS } from '../selectors'
import { TRANSACTION_TYPES, TransactionType, RENEWAL_PERIOD_TYPES, RenewalPeriodType, WAIT_TIMES, PLACEHOLDER_USER_ID } from './points-constants'

// Re-export constants for convenience
export { TRANSACTION_TYPES, RENEWAL_PERIOD_TYPES, WAIT_TIMES, PLACEHOLDER_USER_ID }
export type { TransactionType, RenewalPeriodType }

// ============================================================================
// Retry Logic Helper
// ============================================================================

/**
 * Retry wrapper for async operations with exponential backoff
 * Useful for network-dependent operations that may fail transiently
 *
 * @param operation - Async function to retry
 * @param options - Retry configuration
 * @returns Result of the operation
 */
export async function withRetry<T>(
  operation: () => Promise<T>,
  options: {
    maxRetries?: number
    initialDelay?: number
    backoffMultiplier?: number
    retryableErrors?: string[]
  } = {}
): Promise<T> {
  const {
    maxRetries = 3,
    initialDelay = 1000,
    backoffMultiplier = 2,
    retryableErrors = ['Timeout', 'Network', 'timeout', 'network']
  } = options

  let lastError: Error | null = null
  let currentDelay = initialDelay

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      return await operation()
    } catch (error) {
      lastError = error as Error

      // Check if error is retryable
      const errorMessage = error instanceof Error ? error.message : String(error)
      const isRetryable = retryableErrors.some(errMsg =>
        errorMessage.toLowerCase().includes(errMsg)
      )

      // If not retryable or last attempt, throw immediately
      if (!isRetryable || attempt === maxRetries) {
        throw error
      }

      // Log retry attempt
      console.log(
        `[Retry] Attempt ${attempt + 1}/${maxRetries + 1} failed: ${errorMessage.substring(0, 100)}`
      )
      console.log(`[Retry] Retrying in ${currentDelay}ms...`)

      // Wait before retry
      await new Promise(resolve => setTimeout(resolve, currentDelay))

      // Increase delay for next attempt (exponential backoff)
      currentDelay *= backoffMultiplier
    }
  }

  // This should never be reached, but TypeScript requires it
  throw lastError || new Error('Retry failed')
}

/**
 * Wait for element to be stable with retry logic
 * Combines Playwright's waitFor with retry mechanism for flaky elements
 */
export async function waitForStableElement(
  page: Page,
  selector: string,
  options: {
    timeout?: number
    maxRetries?: number
    state?: 'visible' | 'attached' | 'hidden' | 'detached'
  } = {}
): Promise<void> {
  const {
    timeout = 10000,
    maxRetries = 2,
    state = 'visible'
  } = options

  await withRetry(
    async () => {
      const element = page.locator(selector)
      if (state === 'visible') {
        await element.waitFor({ state: 'visible', timeout })
      } else if (state === 'attached') {
        await element.waitFor({ state: 'attached', timeout })
      } else if (state === 'hidden') {
        await element.waitFor({ state: 'hidden', timeout })
      } else if (state === 'detached') {
        await element.waitFor({ state: 'detached', timeout })
      }
    },
    {
      maxRetries,
      initialDelay: 500,
      backoffMultiplier: 1.5,
      retryableErrors: ['Timeout', 'timeout']
    }
  )
}


// ============================================================================
// Types
// ============================================================================

export interface PointsPlanConfig {
  planId: string
  planTitle?: string
  pointsOnSubscribe: number
  pointsOnRenewal: number
  renewalEnabled: boolean
  renewalPeriodType: RenewalPeriodType
  maxAccumulation: number
}

export interface PointsWalletInfo {
  userId: string
  email: string
  balance: number
  status: string
}

export interface TransactionInfo {
  index: number
  type: string
  amount: number
  balanceAfter: number
  description: string
  time: string
}

// ============================================================================
// Admin Points Page Helpers
// ============================================================================

/**
 * Navigate to Points Admin page
 */
export async function navigateToPointsAdmin(page: Page, realmId: string): Promise<void> {
  await page.goto(`/${realmId}/manage/points`)
  await expect(page.locator(SELECTORS.pointsAdmin.configsPage)).toBeVisible()
}

/**
 * Create a Points Plan Configuration
 */
export async function createPointsPlanConfig(
  page: Page,
  config: PointsPlanConfig
): Promise<void> {
  // Click create button
  await page.locator(SELECTORS.pointsAdmin.createPlanConfigButton).click()

  // Wait for dialog to open
  await expect(page.locator(SELECTORS.pointsAdmin.planConfigDialog)).toBeVisible()

  // Shadcn UI Select interaction: use role-based selector (combobox) to click trigger
  // Note: The dropdown displays plan.title (not plan.id/name), so we must select by title
  const planSelectValue = config.planTitle || config.planId
  await page.getByRole('combobox', { name: 'Plan *' }).click()
  await page.getByRole('option', { name: planSelectValue }).click()

  // Fill form fields
  await page.locator(SELECTORS.pointsAdmin.planConfigPointsOnSubscribe).fill(
    config.pointsOnSubscribe.toString()
  )

  // Set grant on subscribe (switch/toggle)
  if (config.renewalEnabled) {
    await page.locator(SELECTORS.pointsAdmin.planConfigRenewalEnabled).check()
  } else {
    await page.locator(SELECTORS.pointsAdmin.planConfigRenewalEnabled).uncheck()
  }

  // Shadcn UI Select interaction: use getByTestId for the trigger
  await page.getByTestId('grant-period-type').click()
  await page.getByRole('option', { name: config.renewalPeriodType }).click()

  // Set validity days and max periods
  await page.locator(SELECTORS.pointsAdmin.planConfigValidityDays).fill('30')
  await page.locator(SELECTORS.pointsAdmin.planConfigMaxAccumulation).fill(
    config.maxAccumulation.toString()
  )

  // Submit form
  await page.locator(SELECTORS.pointsAdmin.planConfigSubmitButton).click()

  // Wait for dialog to close
  await expect(page.locator(SELECTORS.pointsAdmin.planConfigDialog)).not.toBeVisible()
}

/**
 * Edit a Points Plan Configuration
 */
export async function editPointsPlanConfig(
  page: Page,
  configId: string,
  updates: Partial<PointsPlanConfig>
): Promise<void> {
  // Click edit button
  await page.locator(SELECTORS.pointsAdmin.editPlanConfigButton(configId)).click()

  // Wait for dialog to open
  await expect(page.locator(SELECTORS.pointsAdmin.planConfigDialog)).toBeVisible()

  // Update fields that are provided
  if (updates.pointsOnSubscribe !== undefined) {
    await page.locator(SELECTORS.pointsAdmin.planConfigPointsOnSubscribe).clear()
    await page.locator(SELECTORS.pointsAdmin.planConfigPointsOnSubscribe).fill(
      updates.pointsOnSubscribe.toString()
    )
  }

  if (updates.renewalEnabled !== undefined) {
    if (updates.renewalEnabled) {
      await page.locator(SELECTORS.pointsAdmin.planConfigRenewalEnabled).check()
    } else {
      await page.locator(SELECTORS.pointsAdmin.planConfigRenewalEnabled).uncheck()
    }
  }

  if (updates.renewalPeriodType !== undefined) {
    // Shadcn UI Select interaction: use getByTestId for the trigger
    await page.getByTestId('grant-period-type').click()
    await page.getByRole('option', { name: updates.renewalPeriodType }).click()
  }

  if (updates.maxAccumulation !== undefined) {
    await page.locator(SELECTORS.pointsAdmin.planConfigMaxAccumulation).clear()
    await page.locator(SELECTORS.pointsAdmin.planConfigMaxAccumulation).fill(
      updates.maxAccumulation.toString()
    )
  }

  // Submit form
  await page.locator(SELECTORS.pointsAdmin.planConfigSubmitButton).click()

  // Wait for dialog to close
  await expect(page.locator(SELECTORS.pointsAdmin.planConfigDialog)).not.toBeVisible()
}

/**
 * Delete a Points Plan Configuration
 */
export async function deletePointsPlanConfig(page: Page, configId: string): Promise<void> {
  // Click delete button
  await page.locator(SELECTORS.pointsAdmin.deletePlanConfigButton(configId)).click()

  // Confirm deletion
  await page.locator(SELECTORS.common.dialogSubmitButton).click()

  // Wait for success message
  await expect(page.locator(SELECTORS.common.successMessage)).toBeVisible()
}

/**
 * Search user accounts by email
 */
export async function searchUserAccounts(page: Page, email: string): Promise<void> {
  await page.locator(SELECTORS.pointsAdmin.accountsSearch).clear()
  await page.locator(SELECTORS.pointsAdmin.accountsSearch).fill(email)

  // Wait for debounced search to complete - check for URL update or results
  await page.waitForLoadState('networkidle')
}

/**
 * Select a user account to view transactions
 */
export async function selectUserAccount(page: Page, userId: string): Promise<void> {
  await page.locator(SELECTORS.pointsAdmin.accountRow(userId)).click()

  // Wait for transactions section to appear
  await expect(page.locator(SELECTORS.pointsAdmin.transactionsSection)).toBeVisible()
}

/**
 * Apply transaction filters (Admin)
 */
export async function applyTransactionFilters(
  page: Page,
  filters: {
    type?: TransactionType
    startTime?: string
    endTime?: string
    clientApp?: string
  }
): Promise<void> {
  if (filters.type !== undefined) {
    await page.locator(SELECTORS.pointsAdmin.filterType).selectOption(filters.type)
  }

  if (filters.startTime !== undefined) {
    await page.locator(SELECTORS.pointsAdmin.filterStartTime).fill(filters.startTime)
  }

  if (filters.endTime !== undefined) {
    await page.locator(SELECTORS.pointsAdmin.filterEndTime).fill(filters.endTime)
  }

  if (filters.clientApp !== undefined) {
    await page.locator(SELECTORS.pointsAdmin.filterClientApp).selectOption(filters.clientApp)
  }

  // Apply filters
  await page.locator(SELECTORS.pointsAdmin.applyFiltersButton).click()

  // Wait for filter to apply - check for network idle
  await page.waitForLoadState('networkidle')
}

/**
 * Reset transaction filters (Admin)
 */
export async function resetTransactionFilters(page: Page): Promise<void> {
  await page.locator(SELECTORS.pointsAdmin.resetFiltersButton).click()

  // Wait for filter to reset - check for network idle
  await page.waitForLoadState('networkidle')
}

/**
 * Get user account info from table
 */
export async function getUserAccountInfo(
  page: Page,
  userId: string
): Promise<PointsWalletInfo> {
  const row = page.locator(SELECTORS.pointsAdmin.accountRow(userId))

  const [nameText, email, balanceText, status] = await Promise.all([
    row.locator('.font-medium').textContent(),
    row.locator('.text-sm.text-muted-foreground').textContent(),
    row.locator('.text-2xl.font-bold').textContent(),
    row.locator('[data-slot="badge"]').textContent(),
  ])

  return {
    userId,
    email: email || nameText || '',
    balance: parseInt(balanceText?.replace(/,/g, '') || '0'),
    status: status || '',
  }
}

/**
 * Get transaction info from table
 */
export async function getTransactionInfo(
  page: Page,
  index: number
): Promise<TransactionInfo> {
  const row = page.locator(SELECTORS.pointsAdmin.transactionRow(index))

  // Parallel fetch all text content
  const [type, amountText, balanceText, description, time] = await Promise.all([
    row.locator(SELECTORS.pointsAdmin.transactionType(index)).textContent(),
    row.locator(SELECTORS.pointsAdmin.transactionAmount(index)).textContent(),
    row.locator(SELECTORS.pointsAdmin.transactionBalance(index)).textContent(),
    row.locator(SELECTORS.pointsAdmin.transactionDescription(index)).textContent(),
    row.locator(SELECTORS.pointsAdmin.transactionTime(index)).textContent(),
  ])

  return {
    index,
    type: type || '',
    amount: parseInt(amountText?.replace(/[^0-9-]/g, '') || '0'),
    balanceAfter: parseInt(balanceText?.replace(/,/g, '') || '0'),
    description: description || '',
    time: time || '',
  }
}

// ============================================================================
// User Points Page Helpers
// ============================================================================

/**
 * Navigate to Points User page
 */
export async function navigateToPointsUser(page: Page, realmId: string): Promise<void> {
  await page.goto(`/${realmId}/user/points`)
  await expect(page.locator(SELECTORS.pointsUser.page)).toBeVisible()
}

/**
 * Generic helper to parse a number from a selector
 * @param page - Playwright Page object
 * @param selector - CSS selector for the element containing the number
 * @returns Parsed integer value
 */
async function parseNumberFromSelector(page: Page, selector: string): Promise<number> {
  const text = await page.locator(selector).textContent()
  return parseInt(text?.replace(/,/g, '') || '0')
}

/**
 * Get user balance from balance card
 */
export async function getUserBalance(page: Page): Promise<number> {
  return parseNumberFromSelector(page, SELECTORS.pointsUser.balanceAmount)
}

/**
 * Get account status from balance card
 */
export async function getWalletStatus(page: Page): Promise<string> {
  return await page.locator(SELECTORS.pointsUser.accountStatus).textContent() || ''
}

/**
 * Apply user transaction filters
 */
export async function applyUserTransactionFilters(
  page: Page,
  filters: {
    type?: TransactionType
    startTime?: string
    endTime?: string
  }
): Promise<void> {
  if (filters.type !== undefined) {
    await page.locator(SELECTORS.pointsUser.filterType).selectOption(filters.type)
  }

  if (filters.startTime !== undefined) {
    await page.locator(SELECTORS.pointsUser.filterStartTime).fill(filters.startTime)
  }

  if (filters.endTime !== undefined) {
    await page.locator(SELECTORS.pointsUser.filterEndTime).fill(filters.endTime)
  }

  // Apply filters
  await page.locator(SELECTORS.pointsUser.applyFiltersButton).click()

  // Wait for filter to apply - check for network idle
  await page.waitForLoadState('networkidle')
}

/**
 * Reset user transaction filters
 */
export async function resetUserTransactionFilters(page: Page): Promise<void> {
  await page.locator(SELECTORS.pointsUser.resetFiltersButton).click()

  // Wait for filter to reset - check for network idle
  await page.waitForLoadState('networkidle')
}

/**
 * Click view transaction detail
 */
export async function viewTransactionDetail(page: Page, index: number): Promise<void> {
  await page.locator(SELECTORS.pointsUser.transactionRow(index)).click()

  // Wait for detail dialog
  await expect(page.locator('[data-testid="transaction-detail-dialog"]')).toBeVisible()
}

/**
 * Export transaction history
 */
export async function exportTransactionHistory(page: Page): Promise<void> {
  // Setup download listener before clicking
  const downloadPromise = page.waitForEvent('download')

  await page.locator(SELECTORS.pointsUser.exportButton).click()

  // Wait for download to start
  await downloadPromise
}

// ============================================================================
// Points Configuration & Statistics Helpers
// ============================================================================

/**
 * Points-related route constants
 */
export const POINTS_ROUTES = {
  REALM_CONFIG: (realmId: string) => `/${realmId}/manage/points/realm-config`,
  FREE_STATS: (realmId: string) => `/${realmId}/manage/points/free-stats`,
  FREE_USERS: (realmId: string) => `/${realmId}/manage/points/free-users`,
  USER_POINTS: (realmId: string) => `/${realmId}/user/points`,
  REGISTER: (realmId: string) => `/${realmId}/auth/register`,
} as const

/**
 * Generate test email with timestamp for isolation
 */
export function generateTestEmail(testStartTime: number, prefix: string = 'test-user'): string {
  return `${prefix}-${testStartTime}@example.com`
}

/**
 * Update realm points configuration
 */
export async function updateRealmConfig(
  page: Page,
  selector: string,
  value: string
): Promise<void> {
  await page.locator(selector).fill(value)
  await page.locator(SELECTORS.points.saveButton).click()
  await expect(page.getByText(/配置已更新|configuration.*updated/i).first()).toBeVisible()
  await expect(page.locator(selector)).toHaveValue(value)
}

/**
 * Verify configuration validation error
 */
export async function verifyConfigValidation(
  page: Page,
  selector: string,
  invalidValue: string,
  expectedErrorPattern: RegExp
): Promise<void> {
  await page.locator(selector).clear()
  await page.locator(selector).fill(invalidValue)
  // Client-side Zod validation shows error immediately (save button is disabled)
  await expect(page.getByText(expectedErrorPattern)).toBeVisible()
  await expect(page.locator(SELECTORS.points.saveButton)).toBeDisabled()
}

/**
 * Verify chart is displayed with optional legend verification
 */
export async function verifyChartDisplayed(
  page: Page,
  chartSelector: string,
  verifyLegend?: string[]
): Promise<void> {
  const chart = page.getByTestId(chartSelector)
  await expect(chart).toBeVisible()
  await expect(chart.locator('canvas')).toBeVisible()

  if (verifyLegend) {
    for (const legend of verifyLegend) {
      await expect(page.getByText(new RegExp(legend, 'i'))).toBeVisible()
    }
  }
}

/**
 * Verify statistics are displayed
 */
export async function verifyStatisticsDisplayed(
  page: Page,
  statistics: Array<{ pattern: RegExp; description: string }>
): Promise<void> {
  for (const stat of statistics) {
    await expect(page.getByText(stat.pattern)).toBeVisible()
  }
}

/**
 * Register a new user
 */
export async function registerUser(
  page: Page,
  realmId: string,
  email: string,
  password: string = 'password123'
): Promise<void> {
  await page.goto(POINTS_ROUTES.REGISTER(realmId))

  // Wait for form to be visible
  await page.getByTestId('register-email-input').waitFor({ state: 'visible', timeout: 5000 })

  // Fill registration form
  await page.getByTestId('register-email-input').fill(email)
  await page.getByTestId('register-password-input').fill(password)
  await page.getByTestId('register-confirm-password-input').fill(password)

  // Submit form using specific testid selector
  await page.getByTestId('register-submit-button').click()

  // Wait for registration API response
  await page.waitForResponse(
    response => response.url().includes('/register') && response.request().method() === 'POST',
    { timeout: 10000 }
  )
}

/**
 * Navigate to points page and verify
 */
export async function navigateToPointsPageAndVerify(
  page: Page,
  realmId: string
): Promise<void> {
  await page.goto(POINTS_ROUTES.USER_POINTS(realmId))
  await expect(page.locator(SELECTORS.pointsUser.page)).toBeVisible()
}


