import { Page, expect } from '@playwright/test'

/**
 * Clicks a row action menu item for a table row identified by text content.
 * Menu dropdowns are rendered via React Portal so must be located globally on the page.
 * This function handles pagination and will search through multiple pages to find the row.
 */
export async function clickRowMenuItem(
  page: Page,
  rowText: string,
  menuItemName: string
): Promise<void> {
  const maxPages = 10 // Safety limit to prevent infinite loops
  let currentPage = 0

  while (currentPage < maxPages) {
    // Check if the row exists on the current page
    const row = page.locator(`tr:has-text("${rowText}")`)
    const rowCount = await row.count()

    if (rowCount > 0) {
      // Found the row — scroll into view if needed, then click the menu
      await row.first().scrollIntoViewIfNeeded()
      await expect(row.first()).toBeVisible()
      const menuTrigger = row.first().getByRole('button', { name: 'Open menu' })
      await expect(menuTrigger).toBeVisible()
      await menuTrigger.click()

      await expect(page.getByRole('menu')).toBeVisible()
      await page.getByRole('menuitem', { name: menuItemName }).click()
      return
    }

    // Row not found, check if there's a next page button
    const nextButton = page.getByRole('button', { name: /next|»|>/i })
    const nextButtonExists = await nextButton.count() > 0

    if (!nextButtonExists) {
      // No more pages, row not found
      throw new Error(`Row with text "${rowText}" not found in table after checking ${currentPage + 1} page(s)`)
    }

    // Check if next button is disabled
    const isDisabled = await nextButton.isDisabled().catch(() => false)
    if (isDisabled) {
      throw new Error(`Row with text "${rowText}" not found in table (reached last page)`)
    }

    // Navigate to next page
    await nextButton.click()
    await page.waitForLoadState('networkidle')
    await page.waitForTimeout(500) // Wait for table to update
    currentPage++
  }

  throw new Error(`Row with text "${rowText}" not found in table after searching ${maxPages} pages`)
}
