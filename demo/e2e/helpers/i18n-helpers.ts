/**
 * i18n Helper Functions
 *
 * Provides locale switching, storage access, and sidebar text assertion
 * helpers for E2E demo tests.
 *
 * All selectors are data-testid-based via the centralized SELECTORS object.
 */

import { Page, expect } from '@playwright/test'
import { SELECTORS } from '../selectors'

/**
 * Switch the UI to the target locale by clicking the corresponding
 * language-switcher button.
 *
 * Primary verification: wait until localStorage key `herald-locale` reflects
 * the target locale value.
 *
 * @param page  Playwright Page object
 * @param locale Target locale — `'en'` or `'zh-CN'`
 */
export async function switchToLocale(
  page: Page,
  locale: 'en' | 'zh-CN',
): Promise<void> {
  const buttonSelector =
    locale === 'en'
      ? SELECTORS.languageSwitcher.enButton
      : SELECTORS.languageSwitcher.zhButton

  await page.locator(buttonSelector).click()

  // Wait until the locale is persisted in localStorage
  await page.waitForFunction(
    (lang: string) => localStorage.getItem('herald-locale') === lang,
    locale,
  )
}

/**
 * Read the current locale from localStorage.
 *
 * @param page Playwright Page object
 * @returns The raw `herald-locale` value, or `null` if unset
 */
export async function getLocaleFromStorage(
  page: Page,
): Promise<string | null> {
  return page.evaluate(() => localStorage.getItem('herald-locale'))
}

/**
 * Set the locale directly in localStorage (without UI interaction).
 *
 * Typed as `string` (not union) to allow unsupported locale values for
 * fallback/edge-case testing.
 *
 * @param page   Playwright Page object
 * @param locale Locale string to store
 */
export async function setLocaleInStorage(
  page: Page,
  locale: string,
): Promise<void> {
  await page.evaluate(
    (lang: string) => localStorage.setItem('herald-locale', lang),
    locale,
  )
}

/**
 * Get the visible text of a sidebar menu item.
 *
 * The testid targets a parent element (Link / div) that contains an inner
 * `<span>` holding the translated label. This helper locates the first
 * matching span and returns its trimmed text content.
 *
 * @param page        Playwright Page object
 * @param menuTestId  A data-testid selector string (e.g. `SELECTORS.sidebar.menuDashboard`)
 * @returns Trimmed text content of the menu label span
 */
export async function getSidebarMenuText(
  page: Page,
  menuTestId: string,
): Promise<string> {
  const menuItem = page.locator(menuTestId)
  await expect(menuItem).toBeVisible()

  const text = await menuItem.locator('span[class*="truncate"]').first().textContent()
  return (text ?? '').trim()
}

/**
 * Assert that the Dashboard sidebar menu item displays its English label.
 */
export async function assertSidebarInEnglish(page: Page): Promise<void> {
  const text = await getSidebarMenuText(page, SELECTORS.sidebar.menuDashboard)
  expect(text).toBe('Dashboard')
}

/**
 * Assert that the Dashboard sidebar menu item displays its Chinese label.
 */
export async function assertSidebarInChinese(page: Page): Promise<void> {
  const text = await getSidebarMenuText(page, SELECTORS.sidebar.menuDashboard)
  expect(text).toBe('仪表盘') // "仪表盘"
}
