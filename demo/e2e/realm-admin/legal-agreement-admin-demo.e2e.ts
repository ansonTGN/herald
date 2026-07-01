/**
 * Realm Admin Legal Agreement Management Demo Tests
 *
 * User Story: .ai/user-stories/core/legal-consent-account-deletion.md [US-RA-019]
 * Design Doc: .ai/design/legal-consent-account-deletion.md §4.4.1, §4.4.2, §4.2.2, §5.1
 *
 * Scenarios:
 * - US-RA-019 Scenario 1: Realm Admin opens Settings > Legal and views current
 *   agreements, source badges (default/custom), version metadata, and history.
 * - US-RA-019 Scenario 2/6: Publishing a custom agreement creates a new version,
 *   switches the source badge to custom, and triggers re-consent for a regular
 *   user on their next login.
 * - US-RA-019 Scenario 3: Reverting to the default template snapshot creates
 *   another new custom version and also triggers re-consent.
 *
 * Compliance: spec/demo/e2e-testing.md
 * - All operations go through the UI (no direct API calls).
 * - Shared selectors are used for all DOM assertions.
 * - Test users are cleaned up after each test while preserving the realm admin.
 */

import { test, expect, cleanupTestData } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { loginAsAdminWithConsent } from '../helpers/legal-consent/consent-aware-login'
import { LoginPage } from '../pages/login-page'
import { SELECTORS } from '../selectors'
import type { Page } from '@playwright/test'

const realmId = 'realm-001'

test.describe('[Realm Admin] Legal Agreement Management Demo Tests', () => {
  let testStartTime: number
  const createdEmails: string[] = []
  let registrationCounter = 0

  test.beforeEach(async ({ page, testStartTime: startTime }) => {
    testStartTime = startTime
    createdEmails.length = 0
    registrationCounter = 0

    await verifyTestEnvironment(page, {
      requiredRealms: ['realm-001'],
      requiredUsers: ['admin@realm-001.com'],
    })
  })

  test.afterEach(async ({ page }) => {
    await cleanupTestData(page, realmId, {
      keepUsers: ['admin@realm-001.com'],
      timestamp: testStartTime,
      testUserEmails: [...createdEmails],
    })
  })

  /**
   * Clear the browser session so the next actor can log in cleanly.
   */
  async function clearUserSession(page: Page): Promise<void> {
    await page.context().clearCookies()
    await page.evaluate(() => {
      localStorage.clear()
      sessionStorage.clear()
    }).catch(() => {
      // Storage may not be accessible if the page is in a transient state.
    })
  }

  /**
   * Determine whether the currently rendered source badge for an agreement is
   * default or custom.
   */
  async function getAgreementSource(
    page: Page,
    agreementType: 'terms_of_service' | 'privacy_policy'
  ): Promise<'default' | 'custom'> {
    const card = page.locator(SELECTORS.legalConsent.legalAgreementCard(agreementType))
    const customBadge = card.locator(SELECTORS.legalConsent.sourceBadge('custom'))
    const defaultBadge = card.locator(SELECTORS.legalConsent.sourceBadge('default'))

    if (await customBadge.isVisible({ timeout: 2000 }).catch(() => false)) {
      return 'custom'
    }
    if (await defaultBadge.isVisible({ timeout: 2000 }).catch(() => false)) {
      return 'default'
    }

    throw new Error(`Could not determine source badge for ${agreementType}`)
  }

  /**
   * Register a fresh regular user through the UI and log them in.
   * Registration records consent to the currently effective agreement versions.
   */
  async function registerAndLoginUser(
    page: Page,
    loginPage: LoginPage
  ): Promise<{ email: string; password: string }> {
    const suffix = `${testStartTime}-${registrationCounter++}`
    const email = `lc-admin-legal-${suffix}@example.com`
    const nickname = `lc-admin-legal-${suffix}`
    const password = 'Password123!'

    await test.step('Register test user', async () => {
      await page.goto(`/${realmId}/auth/register`, { waitUntil: 'domcontentloaded' })
      await expect(page.locator(SELECTORS.registration.card)).toBeVisible()

      await page.locator(SELECTORS.registration.emailInput).fill(email)
      await page.locator(SELECTORS.registration.nicknameInput).fill(nickname)
      await page.locator(SELECTORS.registration.passwordInput).fill(password)
      await page.locator(SELECTORS.registration.confirmPasswordInput).fill(password)
      await page.locator(SELECTORS.legalConsent.registerConsentCheckbox).check()
      await expect(
        page.locator(SELECTORS.legalConsent.registerConsentCheckbox)
      ).toBeChecked()

      const responsePromise = page.waitForResponse(
        response =>
          response.url().includes(`/api/auth/${realmId}/register`) &&
          response.request().method() === 'POST',
        { timeout: 10000 }
      )
      await page.locator(SELECTORS.registration.registerButton).click()
      const response = await responsePromise
      expect(response.ok()).toBe(true)
    })

    await test.step('Log in as the test user', async () => {
      await loginPage.loginAsUser(email, password, realmId)
    })

    return { email, password }
  }

  test('Scenario 1: Realm Admin can view the Legal tab and current agreements', async ({
    page,
  }) => {
    await test.step('Login as realm admin and open Settings > Legal', async () => {
      await loginAsAdminWithConsent(page, realmId)
      await page.goto(`/${realmId}/manage/settings`, { waitUntil: 'domcontentloaded' })
      await page.locator(SELECTORS.legalConsent.legalTab).click()
      await expect(page.locator(SELECTORS.legalConsent.legalAgreementsTab)).toBeVisible({
        timeout: 10000,
      })
    })

    await test.step('Assert both agreement cards are visible', async () => {
      await expect(
        page.locator(SELECTORS.legalConsent.legalAgreementCard('terms_of_service'))
      ).toBeVisible()
      await expect(
        page.locator(SELECTORS.legalConsent.legalAgreementCard('privacy_policy'))
      ).toBeVisible()
    })

    await test.step('Assert each card has a source badge and version metadata', async () => {
      for (const agreementType of ['terms_of_service', 'privacy_policy'] as const) {
        const card = page.locator(SELECTORS.legalConsent.legalAgreementCard(agreementType))
        const badge = card
          .locator(SELECTORS.legalConsent.sourceBadge('default'))
          .or(card.locator(SELECTORS.legalConsent.sourceBadge('custom')))
        await expect(badge).toBeVisible()

        const meta = page.locator(SELECTORS.legalConsent.legalAgreementMeta(agreementType))
        await expect(meta).toBeVisible()
        const metaText = await meta.textContent()
        expect(metaText).toMatch(/Version:\s*\d+/)
      }
    })

    await test.step('Assert each card has a history table or an empty placeholder', async () => {
      for (const agreementType of ['terms_of_service', 'privacy_policy'] as const) {
        const historyTable = page.locator(
          SELECTORS.legalConsent.legalHistoryTable(agreementType)
        )
        const historyEmpty = page.locator(
          `[data-testid="legal-history-empty-${agreementType}"]`
        )
        await expect(historyTable.or(historyEmpty)).toBeVisible()
      }
    })
  })

  test('Scenario 2: Publishing a custom agreement triggers re-consent', async ({
    page,
    loginPage,
    adminLegalHelper,
  }) => {
    const { email, password } = await test.step(
      'Register and login a test user to establish consent',
      async () => {
        const user = await registerAndLoginUser(page, loginPage)
        createdEmails.push(user.email)
        return user
      }
    )

    await test.step('Clear session and switch to realm admin', async () => {
      await clearUserSession(page)
    })

    const { beforeVersion, afterVersion } = await test.step(
      'Publish a custom Terms of Service version',
      async () => {
        // loginAsAdminWithConsent ensures a stable admin session at the start.
        await adminLegalHelper.gotoLegalTab(realmId)

        const beforeVersion = await adminLegalHelper.getCurrentVersion('terms_of_service')
        await adminLegalHelper.publishCustomAgreement(
          'terms_of_service',
          'Custom Terms of Service for admin legal demo (EN).',
          '管理员法律协议演示自定义服务条款（中文）。',
          'admin-legal-demo-publish'
        )
        await adminLegalHelper.expectSourceBadge('terms_of_service', 'custom')
        const afterVersion = await adminLegalHelper.getCurrentVersion('terms_of_service')

        return { beforeVersion, afterVersion }
      }
    )

    expect(afterVersion).toBeGreaterThan(beforeVersion)

    await test.step('Clear session and login the test user again', async () => {
      await clearUserSession(page)
      await loginPage.goto(realmId)
      await loginPage.login({ email, password })
    })

    await test.step('Expect login-time re-consent view and agree to continue', async () => {
      await expect(
        page.locator(SELECTORS.legalConsent.loginReconsentView)
      ).toBeVisible({ timeout: 10000 })

      await Promise.all([
        page.waitForURL(
          new RegExp(`^http://localhost:3000/${realmId}/(?!auth/login)`),
          { timeout: 15000 }
        ),
        page.locator(SELECTORS.legalConsent.loginAgreeAndContinueButton).click(),
      ])

      await expect(
        page.locator(SELECTORS.header.userAvatar).or(page.locator(SELECTORS.profile.container))
      ).toBeVisible({ timeout: 10000 })
    })
  })

  test('Scenario 3: Reverting to default snapshot triggers re-consent', async ({
    page,
    loginPage,
    adminLegalHelper,
  }) => {
    await test.step('Ensure ToS is in a custom state', async () => {
      await adminLegalHelper.gotoLegalTab(realmId)
      const source = await getAgreementSource(page, 'terms_of_service')
      if (source === 'default') {
        await adminLegalHelper.publishCustomAgreement(
          'terms_of_service',
          'Pre-seed custom Terms of Service for revert demo (EN).',
          '回退演示用预置自定义服务条款（中文）。',
          'admin-legal-demo-preseed'
        )
      }
      await clearUserSession(page)
    })

    const { email, password } = await test.step(
      'Register and login another test user to establish consent on the custom version',
      async () => {
        const user = await registerAndLoginUser(page, loginPage)
        createdEmails.push(user.email)
        return user
      }
    )

    await test.step('Clear session and switch back to realm admin', async () => {
      await clearUserSession(page)
    })

    const { beforeVersion, afterVersion } = await test.step(
      'Revert ToS to the default template snapshot',
      async () => {
        await adminLegalHelper.gotoLegalTab(realmId)
        const beforeVersion = await adminLegalHelper.getCurrentVersion('terms_of_service')
        await adminLegalHelper.revertToDefault('terms_of_service')
        const afterVersion = await adminLegalHelper.getCurrentVersion('terms_of_service')

        const historyTable = page.locator(
          SELECTORS.legalConsent.legalHistoryTable('terms_of_service')
        )
        await expect(historyTable).toBeVisible()

        return { beforeVersion, afterVersion }
      }
    )

    expect(afterVersion).toBeGreaterThan(beforeVersion)

    await test.step('Clear session and login the test user again', async () => {
      await clearUserSession(page)
      await loginPage.goto(realmId)
      await loginPage.login({ email, password })
    })

    await test.step('Expect login-time re-consent view and agree to continue', async () => {
      await expect(
        page.locator(SELECTORS.legalConsent.loginReconsentView)
      ).toBeVisible({ timeout: 10000 })

      await Promise.all([
        page.waitForURL(
          new RegExp(`^http://localhost:3000/${realmId}/(?!auth/login)`),
          { timeout: 15000 }
        ),
        page.locator(SELECTORS.legalConsent.loginAgreeAndContinueButton).click(),
      ])

      await expect(
        page.locator(SELECTORS.header.userAvatar).or(page.locator(SELECTORS.profile.container))
      ).toBeVisible({ timeout: 10000 })
    })
  })
})
