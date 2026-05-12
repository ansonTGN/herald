/**
 * Subscription History Page Helpers
 *
 * Shared helper functions for subscription history tests
 */

import { Page, expect } from '@playwright/test'
import { SELECTORS } from '../../selectors'

/**
 * Event type constants for subscription history
 */
export const EVENT_TYPES = {
  created: 'created',
  upgraded: 'upgraded',
  downgraded: 'downgraded',
  canceled: 'canceled',
  renewed: 'renewed',
  reactivated: 'reactivated',
  expired: 'expired',
} as const

/**
 * Event display type constants (for UI display)
 */
export const EVENT_DISPLAY_TYPES = {
  created: 'Created',
  upgraded: 'Upgraded',
  downgraded: 'Downgraded',
  canceled: 'Canceled',
  renewed: 'Renewed',
  reactivated: 'Reactivated',
  expired: 'Expired',
} as const

/**
 * Helper function to navigate to subscription history page
 */
export async function navigateToSubscriptionHistory(page: Page, realmId: string): Promise<void> {
  await page.goto(`/${realmId}/manage/subscription-history`)
  // Wait for the page to load and the loading state to finish
  await page.waitForLoadState('networkidle')
  // Wait for the loading state to finish if it's present
  await page.waitForSelector('[data-testid="page-loading"]', { state: 'detached', timeout: 5000 }).catch(() => {
    // If the loading state was never present or already finished, that's fine
  })
  // Wait for the page heading to be visible (more reliable than data-testid)
  await expect(page.getByRole('heading', { name: 'Subscription History' })).toBeVisible({ timeout: 10000 })
  // Wait for the "User ID" label to be visible (filter section is rendered)
  await expect(page.getByText('User ID')).toBeVisible({ timeout: 10000 })
  // Wait for the "History Events" heading to be visible (table is rendered)
  await expect(page.getByRole('heading', { name: 'History Events' })).toBeVisible({ timeout: 10000 })
}

/**
 * Helper function to navigate to subscription detail history page
 */
export async function navigateToSubscriptionDetailHistory(page: Page, realmId: string, subscriptionId: string): Promise<void> {
  await page.goto(`/${realmId}/subscription/${subscriptionId}/history`)
  await expect(page.getByTestId(SELECTORS.subscriptionDetailHistory.page)).toBeVisible()
}

/**
 * Helper function to set event type filter
 */
export async function setEventTypeFilter(page: Page, eventType: string): Promise<void> {
  // Use label-based selector instead of data-testid
  await page.getByLabel('Event Type').click()
  // Wait for the dropdown to be visible (check first option)
  await expect(page.getByRole('option').first()).toBeVisible()
  await page.getByRole('option', { name: eventType }).click()
}

/**
 * Helper function to set plan filter
 */
export async function setPlanFilter(page: Page, planId: string): Promise<void> {
  // Use label-based selector instead of data-testid
  await page.getByTestId('subscription-history-filter').getByLabel('Plan').click()
  // Wait for the dropdown to be visible (check first option)
  await expect(page.getByRole('option').first()).toBeVisible()
  await page.getByRole('option', { name: planId }).click()
}

/**
 * Helper function to set user ID filter
 */
export async function setUserIdFilter(page: Page, userId: string): Promise<void> {
  const filterInput = page.locator(SELECTORS.subscriptionHistory.filterUserId)
  await filterInput.clear()
  await filterInput.fill(userId)
}

/**
 * Helper function to set date range filter
 */
export async function setDateRangeFilter(page: Page, fromDate: string, toDate: string): Promise<void> {
  // Use label-based selector instead of data-testid
  const fromInput = page.getByLabel('From Date')
  await fromInput.clear()
  await fromInput.fill(fromDate)

  const toInput = page.getByLabel('To Date')
  await toInput.clear()
  await toInput.fill(toDate)
}

/**
 * Helper function to apply filters
 */
export async function applyFilters(page: Page): Promise<void> {
  // Use text-based selector instead of data-testid
  await page.getByRole('button', { name: 'Apply Filters' }).click()
  // Wait for filters to apply (wait for network to settle)
  await page.waitForLoadState('networkidle')
}

/**
 * Helper function to reset filters
 */
export async function resetFilters(page: Page): Promise<void> {
  // Use text-based selector instead of data-testid
  await page.getByRole('button', { name: 'Reset' }).click()
}

/**
 * Helper function to view event details
 */
export async function viewEventDetails(page: Page, eventIndex: number): Promise<void> {
  await page.getByTestId(SELECTORS.subscriptionHistory.viewDetailsButton(eventIndex)).click()
}

/**
 * Helper function to click timeline event details
 */
export async function viewTimelineEventDetails(page: Page, eventIndex: number): Promise<void> {
  await page.getByTestId(SELECTORS.subscriptionDetailHistory.viewEventDetailsButton(eventIndex)).click()
}

/**
 * Helper function to close event detail dialog
 */
export async function closeEventDetailDialog(page: Page): Promise<void> {
  await page.getByRole('button', { name: 'Close' }).click()
  await expect(page.getByTestId(SELECTORS.subscriptionDetailHistory.eventDetailDialog)).not.toBeVisible()
}

/**
 * Helper function to verify event badge is visible
 */
export async function verifyEventBadge(page: Page, eventType: keyof typeof EVENT_TYPES): Promise<void> {
  const badgeSelector = SELECTORS.subscriptionDetailHistory.eventBadge(eventType)
  await expect(page.locator(badgeSelector)).toBeVisible()
}

/**
 * Helper function to check if timeline is empty
 */
export async function isTimelineEmpty(page: Page): Promise<boolean> {
  const isEmpty = await page.getByTestId(SELECTORS.subscriptionDetailHistory.timelineEmpty).isVisible().catch(() => false)
  return isEmpty
}

/**
 * Helper function to check if timeline is loading
 */
export async function isTimelineLoading(page: Page): Promise<boolean> {
  const isLoading = await page.getByTestId(SELECTORS.subscriptionDetailHistory.timelineLoading).isVisible().catch(() => false)
  return isLoading
}

/**
 * Helper function to wait for timeline to load
 */
export async function waitForTimelineToLoad(page: Page): Promise<void> {
  // Wait for loading indicator to disappear using Playwright's built-in waiting
  await page.waitForFunction(
    async () => {
      // Check if loading indicator is not visible or not present
      const loadingIndicator = document.querySelector('[data-testid="timeline-loading"]')
      return !loadingIndicator || loadingIndicator === null
    },
    { timeout: 10000 }
  )
}

/**
 * Helper function to click back button
 */
export async function clickBackButton(page: Page): Promise<void> {
  // Use text-based selector instead of data-testid
  await page.getByRole('button', { name: 'Back' }).click()
}
