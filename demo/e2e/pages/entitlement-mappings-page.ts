import { Page, Locator, expect } from '@playwright/test'
import { SELECTORS } from '../selectors'
import { BasePage } from './base-page'
import type { UnifiedLogger } from '../helpers/unified-logger'

/**
 * Page Object for the Entitlement Mappings page.
 *
 * Route: /{realmId}/manage/billing/entitlement-mappings
 *
 * User stories:
 * - US-EM-001: View provider entitlement mappings
 * - US-EM-002: Trigger provider product sync
 * - US-EM-004: Entitlement-based points policy configuration
 *
 * @see docs/user-stories/billing/entitlement-mapping.md
 */
export class EntitlementMappingsPage extends BasePage {
  // Page-level locators
  readonly container: Locator
  readonly heading: Locator
  readonly table: Locator
  readonly providerFilterSelect: Locator
  readonly pagination: Locator

  // Provider sync controls
  readonly providerSyncButton: Locator
  readonly syncProviderSelect: Locator
  readonly syncButton: Locator

  // Detail dialog locators
  readonly detailDialog: Locator
  readonly entitlementKeyInput: Locator
  readonly pointsPerPeriodInput: Locator
  readonly grantPeriodTypeSelect: Locator
  readonly validityDaysInput: Locator
  readonly grantOnSubscribeSwitch: Locator
  readonly maxPeriodsInput: Locator
  readonly mappingEnabledSwitch: Locator
  readonly saveMappingButton: Locator
  readonly providerProductInfoCard: Locator

  // Empty state
  readonly emptyState: Locator

  constructor(page: Page, logger?: UnifiedLogger) {
    super(page, logger)
    this.container = page.locator(SELECTORS.entitlementMappings.page)
    this.heading = page.locator(SELECTORS.entitlementMappings.heading)
    this.table = page.locator(SELECTORS.entitlementMappings.table)
    this.providerFilterSelect = page.locator(SELECTORS.entitlementMappings.providerFilterSelect)
    this.pagination = page.locator(SELECTORS.entitlementMappings.pagination)

    this.providerSyncButton = page.locator(SELECTORS.entitlementMappings.providerSyncButton)
    this.syncProviderSelect = page.locator(SELECTORS.entitlementMappings.syncProviderSelect)
    this.syncButton = page.locator(SELECTORS.entitlementMappings.syncButton)

    this.detailDialog = page.locator(SELECTORS.entitlementMappings.detailDialog)
    this.entitlementKeyInput = page.locator(SELECTORS.entitlementMappings.entitlementKeyInput)
    this.pointsPerPeriodInput = page.locator(SELECTORS.entitlementMappings.pointsPerPeriodInput)
    this.grantPeriodTypeSelect = page.locator(SELECTORS.entitlementMappings.grantPeriodTypeSelect)
    this.validityDaysInput = page.locator(SELECTORS.entitlementMappings.validityDaysInput)
    this.grantOnSubscribeSwitch = page.locator(SELECTORS.entitlementMappings.grantOnSubscribeSwitch)
    this.maxPeriodsInput = page.locator(SELECTORS.entitlementMappings.maxPeriodsInput)
    this.mappingEnabledSwitch = page.locator(SELECTORS.entitlementMappings.mappingEnabledSwitch)
    this.saveMappingButton = page.locator(SELECTORS.entitlementMappings.saveMappingButton)
    this.providerProductInfoCard = page.locator(SELECTORS.entitlementMappings.providerProductInfoCard)

    this.emptyState = page.locator(SELECTORS.entitlementMappings.emptyState)
  }

  /**
   * Navigate to the entitlement mappings page for a given realm.
   */
  async goto(realmId: string = 'admin'): Promise<void> {
    await super.goto(`/${realmId}/manage/billing/entitlement-mappings`)
    await this.waitForReady()
  }

  /**
   * Wait for the page container and heading to be visible.
   */
  async waitForReady(): Promise<void> {
    await expect(this.container).toBeVisible()
    await expect(this.heading).toBeVisible()
  }

  /**
   * Wait for data to finish loading (table or empty state becomes visible).
   *
   * The frontend renders a loading skeleton while the API call is in flight.
   * Neither the table nor the empty state has its data-testid set during loading,
   * so tests must wait for data to settle before checking table/empty state.
   */
  async waitForDataLoaded(timeout: number = 10000): Promise<void> {
    await this.page.locator(
      `${SELECTORS.entitlementMappings.table}, ${SELECTORS.entitlementMappings.emptyState}`
    ).first().waitFor({ state: 'visible', timeout })
  }

  /**
   * Get the number of visible mapping rows in the table.
   */
  async getMappingRowCount(): Promise<number> {
    await expect(this.table).toBeVisible()
    const rows = this.table.locator('tbody tr')
    return await rows.count()
  }

  /**
   * Get text content of all cells in a mapping row by index.
   */
  async getMappingRowTexts(rowIndex: number): Promise<string[]> {
    const row = this.table.locator('tbody tr').nth(rowIndex)
    await expect(row).toBeVisible()
    const cells = row.locator('td')
    const count = await cells.count()
    const texts: string[] = []
    for (let i = 0; i < count; i++) {
      texts.push((await cells.nth(i).textContent()) || '')
    }
    return texts
  }

  /**
   * Click a mapping row by index to open the detail dialog.
   */
  async clickMappingRow(rowIndex: number): Promise<void> {
    const row = this.table.locator('tbody tr').nth(rowIndex)
    await expect(row).toBeVisible()
    await row.click()
  }

  /**
   * Filter mappings by payment provider using the Radix Select dropdown.
   */
  async filterByProvider(provider: string): Promise<void> {
    await this.selectRadixOption(this.providerFilterSelect, provider)
    // Wait for table to settle after filter change
    await this.page.waitForLoadState('domcontentloaded')
  }

  /**
   * Check if the empty state card is visible (no mappings).
   */
  async isTableEmpty(): Promise<boolean> {
    return await this.isVisible(this.emptyState)
  }

  /**
   * Get the empty state message text.
   */
  async getEmptyStateText(): Promise<string> {
    await expect(this.emptyState).toBeVisible()
    return (await this.emptyState.textContent()) || ''
  }

  /**
   * Open the detail dialog by clicking a mapping row.
   */
  async openDetailDialog(rowIndex: number): Promise<void> {
    await this.clickMappingRow(rowIndex)
    await expect(this.detailDialog).toBeVisible({ timeout: 5000 })
  }

  /**
   * Check if the detail dialog is currently open.
   */
  async isDetailDialogOpen(): Promise<boolean> {
    return await this.isVisible(this.detailDialog)
  }

  /**
   * Close the detail dialog using the Escape key.
   */
  async closeDetailDialog(): Promise<void> {
    await this.page.keyboard.press('Escape')
    await expect(this.detailDialog).toBeHidden({ timeout: 3000 })
  }

  /**
   * Toggle the enabled switch on a specific mapping row.
   */
  async toggleMappingEnabled(mappingId: string): Promise<void> {
    const toggle = this.page.locator(SELECTORS.entitlementMappings.mappingEnabledToggle(mappingId))
    await this.smartClick(toggle)
  }

  /**
   * Check if pagination controls are visible.
   */
  async isPaginationVisible(): Promise<boolean> {
    return await this.isVisible(this.pagination)
  }

  /**
   * Get all table header text content.
   */
  async getTableHeaders(): Promise<string[]> {
    const headers = this.table.locator('thead th')
    const count = await headers.count()
    const texts: string[] = []
    for (let i = 0; i < count; i++) {
      texts.push((await headers.nth(i).textContent()) || '')
    }
    return texts
  }
}
