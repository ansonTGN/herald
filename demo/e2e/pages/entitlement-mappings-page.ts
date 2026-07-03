import { Page, Locator, expect } from '@playwright/test'
import { SELECTORS } from '../selectors'
import { BasePage } from './base-page'
import type { UnifiedLogger } from '../helpers/unified-logger'

/**
 * Page Object for the master-detail Entitlement Mappings page.
 *
 * Route: /{realmId}/manage/billing/entitlement-mappings
 *
 * Frontend source:
 * frontend/src/components/billing/entitlement-mappings-page.tsx
 * + entitlement-mapping-detail-dialog.tsx (ProtectedPriceConfirmDialog — Cancel-only)
 * + provider-sync-button.tsx (wrapper `<div data-testid="provider-sync-button">`).
 *
 * User stories:
 * - US-EM-001: View provider entitlement mappings (list-pane view)
 * - US-EM-002: Trigger provider product sync
 * - US-EM-007: Multi-price master-detail configuration (shared key, per-price policy)
 *
 * LOUD NOTE — priceKey suffix:
 * `price-edit-row-${externalPriceId ?? mappingId}` and the toggle share the same
 * suffix. For Stripe rows (non-NULL external_price_id) the suffix is the price id;
 * for Creem rows (NULL external_price_id — price-less provider) the
 * suffix falls back to the mapping id. Callers MUST pass the correct key for the
 * provider under test.
 *
 * LOUD NOTE — ProtectedPriceConfirmDialog:
 * The 409 dialog renders ONLY `protected-price-active-subs` +
 * `protected-price-confirm-cancel`. There is NO proceed button: the active-
 * subscription lock is enforced authoritatively by the backend 409 (batch rolls
 * back); the client offers no force path. Tests assert the dialog surfaces the
 * active-sub count, then dismiss it.
 */
export class EntitlementMappingsPage extends BasePage {
  // Page shell
  readonly container: Locator
  readonly heading: Locator

  // Banner regions
  readonly readonlyPermBanner: Locator
  readonly webhookPriceUnresolvedBanner: Locator
  readonly emptyState: Locator

  // Toolbar filters
  readonly providerFilterSelect: Locator
  readonly productFilterSelect: Locator
  readonly entitlementKeyFilterSelect: Locator

  // Master list (left pane)
  readonly mappingProductList: Locator

  // Detail panel (right pane)
  readonly mappingDetailPanel: Locator
  readonly detailHead: Locator
  readonly saveMappingButton: Locator

  // Provider sync controls (wrapper div + inner Button)
  readonly providerSyncButton: Locator
  readonly syncProviderSelect: Locator
  readonly syncButton: Locator
  readonly syncResultProducts: Locator
  readonly syncResultPrices: Locator

  // Protected-price 409 dialog (Cancel-only)
  readonly protectedPriceConfirmDialog: Locator
  readonly protectedPriceActiveSubs: Locator
  readonly protectedPriceConfirmCancel: Locator

  constructor(page: Page, logger?: UnifiedLogger) {
    super(page, logger)
    this.container = page.locator(SELECTORS.multiPriceMapping.page)
    this.heading = page.locator('[data-testid="entitlement-mappings-heading"]')

    this.readonlyPermBanner = page.locator(SELECTORS.multiPriceMapping.readonlyPermBanner)
    this.webhookPriceUnresolvedBanner = page.locator(
      SELECTORS.multiPriceMapping.webhookPriceUnresolvedBanner,
    )
    this.emptyState = page.locator(SELECTORS.multiPriceMapping.emptyState)

    this.providerFilterSelect = page.locator(SELECTORS.multiPriceMapping.providerFilterSelect)
    this.productFilterSelect = page.locator(SELECTORS.multiPriceMapping.productFilterSelect)
    this.entitlementKeyFilterSelect = page.locator(
      SELECTORS.multiPriceMapping.entitlementKeyFilterSelect,
    )

    this.mappingProductList = page.locator(SELECTORS.multiPriceMapping.mappingProductList)

    this.mappingDetailPanel = page.locator(SELECTORS.multiPriceMapping.mappingDetailPanel)
    this.detailHead = page.locator(SELECTORS.multiPriceMapping.detailHead)
    this.saveMappingButton = page.locator(SELECTORS.multiPriceMapping.saveMappingButton)

    // `provider-sync-button` is a wrapper `<div>`; the actionable controls live
    // inside it. Resolve via the wrapper scope so multiple sync buttons (if any)
    // never collide.
    this.providerSyncButton = page.locator(SELECTORS.multiPriceMapping.providerSyncButton)
    // The sync controls live DIRECTLY inside the wrapper `<div data-testid="provider-sync-button">`.
    // Do NOT re-scope by the wrapper testid (that would require the wrapper to
    // contain itself and resolve to 0 elements).
    this.syncProviderSelect = this.providerSyncButton.locator('[data-testid="sync-provider-select"]')
    this.syncButton = this.providerSyncButton.locator(SELECTORS.multiPriceMapping.syncButton)
    this.syncResultProducts = this.providerSyncButton.locator(
      SELECTORS.multiPriceMapping.syncResultProducts,
    )
    this.syncResultPrices = this.providerSyncButton.locator(
      SELECTORS.multiPriceMapping.syncResultPrices,
    )

    this.protectedPriceConfirmDialog = page.locator(
      SELECTORS.multiPriceMapping.protectedPriceConfirmDialog,
    )
    this.protectedPriceActiveSubs = page.locator(
      SELECTORS.multiPriceMapping.protectedPriceActiveSubs,
    )
    this.protectedPriceConfirmCancel = page.locator(
      SELECTORS.multiPriceMapping.protectedPriceConfirmCancel,
    )
  }

  /**
   * Navigate to the entitlement mappings page for a given realm by route.
   *
   * The sidebar entry testid is i18n-derived and must NOT be relied on; always
   * navigate by route.
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
   * Wait for data to finish loading (master list or empty state becomes visible).
   *
   * The frontend renders a loading skeleton while the API call is in flight;
   * neither the product list nor the empty state has its testid during loading.
   */
  async waitForDataLoaded(timeout: number = 10000): Promise<void> {
    await this.page
      .locator(
        `${SELECTORS.multiPriceMapping.mappingProductList}, ${SELECTORS.multiPriceMapping.emptyState}`,
      )
      .first()
      .waitFor({ state: 'visible', timeout })
  }

  /**
   * Check if the empty state card is visible (no mappings).
   */
  async isListEmpty(): Promise<boolean> {
    return await this.isVisible(this.emptyState)
  }

  // ==================== Master list ====================

  /**
   * Select a product in the master list (left pane) by its external product id.
   * The product row testid is `mapping-product-row-${externalProductId}`.
   */
  async selectProduct(productId: string): Promise<void> {
    const row = this.page.locator(SELECTORS.multiPriceMapping.mappingProductRow(productId))
    await this.smartClick(row)
    // The detail panel mounts/remounts on selection change.
    await expect(this.mappingDetailPanel).toBeVisible({ timeout: 5000 })
  }

  /**
   * Click the first product row (helper for tests that don't know the seeded id).
   */
  async selectFirstProduct(): Promise<void> {
    const row = this.page.locator(SELECTORS.multiPriceMapping.firstMappingProductRow()).first()
    await this.smartClick(row)
    await expect(this.mappingDetailPanel).toBeVisible({ timeout: 5000 })
  }

  /**
   * Check if a product row is rendered as selected (aria-current="true").
   */
  async isProductSelected(productId: string): Promise<boolean> {
    const row = this.page.locator(SELECTORS.multiPriceMapping.mappingProductRow(productId))
    const current = await row.getAttribute('aria-current')
    return current === 'true'
  }

  /**
   * Filter mappings by payment provider using the Radix Select dropdown.
   */
  async filterByProvider(provider: string): Promise<void> {
    await this.selectRadixOption(this.providerFilterSelect, provider)
    await this.page.waitForLoadState('domcontentloaded')
  }

  // ==================== Detail panel ====================

  /**
   * Get the price-edit-row locator for a single price.
   *
   * `priceKey` is `externalPriceId` for Stripe rows and `mappingId` for Creem
   * (NULL price) rows — see the loud note on the class.
   */
  getPriceEditRow(priceKey: string): Locator {
    return this.mappingDetailPanel.locator(SELECTORS.multiPriceMapping.priceEditRow(priceKey))
  }

  getMetadataBlock(priceKey: string): Locator {
    return this.mappingDetailPanel.locator(
      SELECTORS.multiPriceMapping.priceMetadataBlock(priceKey),
    )
  }

  getMetadataEntry(scope: 'product' | 'price', key: string): Locator {
    return this.mappingDetailPanel.locator(
      SELECTORS.multiPriceMapping.metadataEntry(scope, key),
    )
  }

  async getMetadataEntryValue(scope: 'product' | 'price', key: string): Promise<string> {
    return (await this.getMetadataEntry(scope, key).textContent())?.trim() ?? ''
  }

  async getProductRowLabel(productId: string): Promise<string> {
    return (
      await this.page
        .locator(SELECTORS.multiPriceMapping.mappingProductRow(productId))
        .textContent()
    )?.trim() ?? ''
  }

  async getDetailHeadLabel(): Promise<string> {
    return (await this.detailHead.textContent())?.trim() ?? ''
  }

  getBillingTypeInput(priceKey: string): Locator {
    return this.getPriceEditRow(priceKey).locator(
      SELECTORS.multiPriceMapping.priceBillingType(priceKey),
    )
  }

  getBillingPeriodInput(priceKey: string): Locator {
    return this.getReadonlyFieldInput(priceKey, 'Period')
  }

  async getPriceDisplayValue(priceKey: string): Promise<string> {
    return this.getReadonlyFieldValue(priceKey, 'Price')
  }

  async getBillingPeriodValue(priceKey: string): Promise<string> {
    return this.getReadonlyFieldValue(priceKey, 'Period')
  }

  async getProductFilterOptionLabels(): Promise<string[]> {
    await this.smartClick(this.productFilterSelect)
    const options = this.page.getByRole('option')
    await expect(options.first()).toBeVisible({ timeout: 3000 })
    const labels = await options
      .evaluateAll((nodes) => nodes.map((node) => node.textContent?.trim() ?? ''))
    await this.page.keyboard.press('Escape')
    await expect(options.first()).toBeHidden({ timeout: 3000 })
    return labels.filter(Boolean)
  }

  /**
   * Get the enabled-toggle locator for a single price.
   */
  getPriceEnabledToggle(priceKey: string): Locator {
    return this.mappingDetailPanel.locator(
      SELECTORS.multiPriceMapping.priceEnabledToggle(priceKey),
    )
  }

  /**
   * Get the shared-key chip locator for an entitlement key (renders once per
   * shared key inside the detail panel).
   */
  getSharedKeyChip(entitlementKey: string): Locator {
    return this.mappingDetailPanel.locator(
      SELECTORS.multiPriceMapping.sharedKeyChip(entitlementKey),
    )
  }

  /**
   * Fill configurable fields on a single price row. Only the supplied fields
   * are touched. The entitlement-key input is matched by the "Entitlement Key"
   * Field label position within the row (no dedicated testid on the input).
   *
   * Options:
   * - entitlementKey: free-text input under "Entitlement Key" label
   * - pointsPerPeriod: numeric input under "Points per period" label
   * - billingPeriod: free-text input under "Period" label
   *
   * billingType / grantPeriodType / validityDays / maxPeriods / grantOnSubscribe
   * live under the "Advanced" collapsible — callers needing them should open
   * it first. This helper covers the common top-level fields only.
   */
  async fillPriceRow(
    priceKey: string,
    fields: {
      entitlementKey?: string
      pointsPerPeriod?: number
      billingPeriod?: string
    },
  ): Promise<void> {
    const row = this.getPriceEditRow(priceKey)
    await expect(row).toBeVisible()

    if (fields.entitlementKey !== undefined) {
      // The "Entitlement Key" Field is `<div class="space-y-1"><Label>…</Label>
      // <input/></div>`. Scope to the Field wrapper via its Label, then to the
      // input that is a SIBLING of the Label — NOT a descendant of an ancestor
      // div (which would match the price-id Field's readOnly input first).
      // `locator('div', {hasText})` matches ancestors too, so we anchor on the
      // Label element and go up to its immediate Field wrapper.
      const keyField = row
        .locator('label', { hasText: 'Entitlement Key' })
        .locator('xpath=ancestor::div[starts-with(@class,"space-y-1")][1]')
      const keyInput = keyField.locator('input').first()
      await this.fillField(keyInput, fields.entitlementKey)
    }
    if (fields.pointsPerPeriod !== undefined) {
      const pointsField = row
        .locator('label', { hasText: 'Points per period' })
        .locator('xpath=ancestor::div[starts-with(@class,"space-y-1")][1]')
      const pointsInput = pointsField.locator('input[type="number"]').first()
      await this.fillField(pointsInput, String(fields.pointsPerPeriod))
    }
    if (fields.billingPeriod !== undefined) {
      // `hasText:'Period'` is case-insensitive substring, so it ALSO matches the
      // "Points per period" Field label. Match a <label> whose text is EXACTLY
      // "Period" (frontend i18n key billing.field_period → "Period") to avoid
      // landing on the points input.
      const periodField = row
        .locator("xpath=./label[normalize-space()='Period']")
        .locator('xpath=ancestor::div[starts-with(@class,"space-y-1")][1]')
      const periodInput = periodField.locator('input').first()
      await this.fillField(periodInput, fields.billingPeriod)
    }
  }

  private async getReadonlyFieldValue(priceKey: string, label: string): Promise<string> {
    const input = this.getReadonlyFieldInput(priceKey, label)
    await expect(input).toBeVisible()
    return await input.inputValue()
  }

  private getReadonlyFieldInput(priceKey: string, label: string): Locator {
    const row = this.getPriceEditRow(priceKey)
    const field = row
      .locator(`xpath=./div[1]//label[normalize-space()='${label}']`)
      .locator('xpath=ancestor::div[starts-with(@class,"space-y-1")][1]')
    return field.locator('input').first()
  }

  /**
   * Toggle the enabled switch on a single price row.
   *
   * NOTE: When the price protects active subscriptions, the backend rejects the
   * disable with a 409 AFTER save (the toggle itself is not pre-disabled on the
   * client). Callers expecting the 409 path should call saveChanges() next and
   * then expectProtectedPriceDialog().
   */
  async togglePriceEnabled(priceKey: string): Promise<void> {
    const toggle = this.getPriceEnabledToggle(priceKey)
    await this.smartClick(toggle)
  }

  /**
   * Click the Save Changes button (batch PUT). Does not wait for the response —
   * callers that need to assert the result should follow with the appropriate
   * expect* call (banner / dialog / panel re-render).
   */
  async saveChanges(): Promise<void> {
    await expect(this.saveMappingButton).toBeVisible()
    await this.saveMappingButton.click()
  }

  // ==================== Protected-price 409 dialog ====================

  /**
   * Assert the protected-price 409 dialog is visible (surfaces after a save that
   * the backend rejected because the toggled price protects active subscriptions).
   */
  async expectProtectedPriceDialog(): Promise<void> {
    await expect(this.protectedPriceConfirmDialog).toBeVisible({ timeout: 5000 })
    await expect(this.protectedPriceActiveSubs).toBeVisible()
  }

  /**
   * Read the active-subscription count surfaced by the 409 dialog.
   */
  async getProtectedPriceActiveSubs(): Promise<number> {
    await expect(this.protectedPriceActiveSubs).toBeVisible()
    const text = (await this.protectedPriceActiveSubs.textContent()) || ''
    const match = text.match(/\d+/)
    return match ? Number(match[0]) : 0
  }

  /**
   * Dismiss the protected-price dialog via its Cancel button (the only action;
   * there is NO proceed button — the lock is backend-enforced).
   */
  async cancelProtectedPrice(): Promise<void> {
    await expect(this.protectedPriceConfirmCancel).toBeVisible()
    await this.protectedPriceConfirmCancel.click()
    await expect(this.protectedPriceConfirmDialog).toBeHidden({ timeout: 3000 })
  }

  // ==================== Webhook-unresolved banner ====================

  /**
   * Assert the webhook-price-unresolved banner is visible (rendered when at
   * least one loaded mapping has an unresolved webhook price).
   */
  async expectWebhookUnresolvedBanner(): Promise<void> {
    await expect(this.webhookPriceUnresolvedBanner).toBeVisible()
  }

  // ==================== Provider sync ====================

  /**
   * Trigger a provider product sync via the toolbar sync button.
   *
   * Selects the provider in the sync-provider dropdown, clicks Sync, and waits
   * for the result spans to surface. Returns the parsed {productsSynced,
   * pricesSynced} counts from the result spans.
   *
   * @param provider 'stripe' | 'creem'
   */
  async sync(
    provider: 'stripe' | 'creem',
  ): Promise<{ productsSynced: number; pricesSynced: number }> {
    await expect(this.providerSyncButton).toBeVisible()
    await this.selectRadixOption(this.syncProviderSelect, provider)

    // Click the inner sync button, then wait for either the result spans or a
    // toast (sync may fail with test credentials). Resolve counts if present.
    await this.smartClick(this.syncButton)

    // Best-effort: wait for result spans (completed/partial sync renders them).
    const resultVisible = await this.syncResultProducts
      .waitFor({ state: 'visible', timeout: 10000 })
      .then(() => true)
      .catch(() => false)

    if (!resultVisible) {
      return { productsSynced: 0, pricesSynced: 0 }
    }

    const productsText = (await this.syncResultProducts.textContent()) || ''
    const pricesText = (await this.syncResultPrices.textContent()) || ''
    const productsMatch = productsText.match(/\d+/)
    const pricesMatch = pricesText.match(/\d+/)
    return {
      productsSynced: productsMatch ? Number(productsMatch[0]) : 0,
      pricesSynced: pricesMatch ? Number(pricesMatch[0]) : 0,
    }
  }
}
