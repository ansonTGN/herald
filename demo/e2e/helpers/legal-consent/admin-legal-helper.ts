import { expect, type Locator, type Page } from '@playwright/test'
import { BasePage } from '../../pages/base-page'
import { SELECTORS } from '../../selectors'
import { loginAsAdminWithConsent } from './consent-aware-login'
import type { UnifiedLogger } from '../unified-logger'

export type AgreementType = 'terms_of_service' | 'privacy_policy'
export type SourceType = 'default' | 'custom'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'

/**
 * Helper for managing legal agreements from the Realm Admin Settings > Legal tab.
 */
export class AdminLegalHelper extends BasePage {
  constructor(page: Page, logger?: UnifiedLogger) {
    super(page, logger)
  }

  /**
   * Log in as the realm admin (consent-aware) and open Settings > Legal.
   */
  async gotoLegalTab(realmId: string): Promise<void> {
    await loginAsAdminWithConsent(this.page, realmId)
    await this.page.goto(`${BASE_URL}/${realmId}/manage/settings`, {
      waitUntil: 'domcontentloaded',
    })

    const legalTab = this.page.locator(SELECTORS.legalConsent.legalTab)
    await this.smartClick(legalTab)

    await expect(this.page.locator(SELECTORS.legalConsent.legalAgreementsTab)).toBeVisible({
      timeout: 10000,
    })
  }

  /**
   * Read the current version number for the given agreement type.
   */
  async getCurrentVersion(agreementType: AgreementType): Promise<number> {
    const meta = this.page.locator(SELECTORS.legalConsent.legalAgreementMeta(agreementType))
    await expect(meta).toBeVisible()
    const text = await meta.textContent()
    const match = text?.match(/Version:\s*(\d+)/)
    if (!match) {
      throw new Error(`Could not parse version number from agreement meta: ${text}`)
    }
    return parseInt(match[1], 10)
  }

  /**
   * Publish a custom agreement version and wait for the version number to increase.
   */
  async publishCustomAgreement(
    agreementType: AgreementType,
    contentEn: string,
    versionLabel?: string
  ): Promise<void> {
    const card = this.agreementCard(agreementType)
    await expect(card).toBeVisible()

    if (versionLabel !== undefined) {
      await this.fillField(card.locator(SELECTORS.legalConsent.legalVersionLabelInput(agreementType)), versionLabel)
    }
    await this.fillField(card.locator(SELECTORS.legalConsent.legalContentEnInput(agreementType)), contentEn)

    const publishResponsePromise = this.page.waitForResponse(
      response =>
        response.url().includes(`/api/legal/admin/`) &&
        response.url().includes(`/agreements/${agreementType}`) &&
        response.request().method() === 'PUT',
      { timeout: 10000 }
    )
    const publishButton = card.locator(SELECTORS.legalConsent.legalPublishButton(agreementType))
    await this.smartClick(publishButton)
    const publishResponse = await publishResponsePromise
    expect(publishResponse.ok()).toBe(true)

    await expect(
      card.locator(SELECTORS.legalConsent.sourceBadge('custom')).first()
    ).toBeVisible({ timeout: 15000 })
  }

  /**
   * Revert the agreement to the platform default template snapshot and wait for
   * the version number to increase.
   */
  async revertToDefault(agreementType: AgreementType): Promise<void> {
    const beforeVersion = await this.getCurrentVersion(agreementType)

    const card = this.agreementCard(agreementType)
    const revertButton = card.locator(SELECTORS.legalConsent.legalRevertButton(agreementType))
    await this.smartClick(revertButton)

    const confirmButton = this.page.locator(
      SELECTORS.legalConsent.legalRevertConfirmButton(agreementType)
    )
    await this.smartClick(confirmButton)

    await expect(async () => {
      const afterVersion = await this.getCurrentVersion(agreementType)
      expect(afterVersion).toBeGreaterThan(beforeVersion)
    }).toPass({ timeout: 15000 })
  }

  /**
   * Assert that the source badge for the agreement matches the expected source.
   */
  async expectSourceBadge(agreementType: AgreementType, source: SourceType): Promise<void> {
    const card = this.agreementCard(agreementType)
    const badge = card.locator(SELECTORS.legalConsent.sourceBadge(source)).first()
    await expect(badge).toBeVisible()
  }

  private agreementCard(agreementType: AgreementType): Locator {
    return this.page.locator(SELECTORS.legalConsent.legalAgreementCard(agreementType))
  }
}
