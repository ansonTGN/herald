import { expect, type Page } from '@playwright/test'
import { BasePage } from '../../pages/base-page'
import { SELECTORS } from '../../selectors'
import type { UnifiedLogger } from '../unified-logger'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'

/**
 * Helper for interacting with public legal agreement pages and the post-login
 * re-consent dialog.
 */
export class LegalConsentHelper extends BasePage {
  constructor(page: Page, logger?: UnifiedLogger) {
    super(page, logger)
  }

  /**
   * Navigate to a public legal agreement page.
   */
  async gotoAgreementPage(realmId: string, agreementType: string): Promise<void> {
    await this.page.goto(`${BASE_URL}/${realmId}/legal/${agreementType}`, {
      waitUntil: 'domcontentloaded',
    })
  }

  /**
   * Assert that the agreement page is rendered for the requested type.
   */
  async expectAgreementPage(agreementType: string): Promise<void> {
    // Keep the argument in the public signature for caller clarity.
    void agreementType
    await expect(this.page.locator(SELECTORS.legalConsent.agreementCard)).toBeVisible()
    await expect(this.page.locator(SELECTORS.legalConsent.agreementTitle)).toBeVisible()
    await expect(this.page.locator(SELECTORS.legalConsent.agreementVersion)).toBeVisible()
    await expect(this.page.locator(SELECTORS.legalConsent.agreementEffectiveDate)).toBeVisible()
    await expect(this.page.locator(SELECTORS.legalConsent.agreementBody)).toBeVisible()
  }

  /**
   * Assert that the post-login re-consent dialog is visible.
   */
  async expectReconsentDialogVisible(): Promise<void> {
    await expect(this.page.locator(SELECTORS.legalConsent.reconsentDialogTitle)).toBeVisible({
      timeout: 5000,
    })
    await expect(this.page.locator(SELECTORS.legalConsent.reconsentDialogDescription)).toBeVisible()
  }

  /**
   * Click the "Agree" button in the post-login re-consent dialog.
   */
  async agreeAndContinue(): Promise<void> {
    const button = this.page.locator(SELECTORS.legalConsent.reconsentAgreeButton)
    await this.smartClick(button)
  }

  /**
   * Click the "Logout" button in the post-login re-consent dialog.
   */
  async logoutFromDialog(): Promise<void> {
    const button = this.page.locator(SELECTORS.legalConsent.reconsentLogoutButton)
    await this.smartClick(button)
  }
}
