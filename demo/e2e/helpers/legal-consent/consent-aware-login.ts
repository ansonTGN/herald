import { Page } from '@playwright/test'
import { loginAsAdmin } from '../auth'
import { SELECTORS } from '../../selectors'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'

/**
 * Login as the realm admin and transparently agree to any login-time
 * re-consent prompt. This keeps cleanup / admin scenarios stable after a
 * scenario test publishes or reverts a legal agreement version.
 */
export async function loginAsAdminWithConsent(
  page: Page,
  realmId: string,
  options?: { forceRelogin?: boolean }
): Promise<void> {
  console.log(`[ConsentAwareLogin] Logging in as admin for realm: ${realmId}`)

  await loginAsAdmin(page, {
    realmId,
    forceRelogin: options?.forceRelogin ?? true,
    waitNavigation: false,
  })

  const reconsentView = page.locator(SELECTORS.legalConsent.loginReconsentView)
  const needsReconsent = await reconsentView.isVisible({ timeout: 3000 }).catch(() => false)

  if (needsReconsent) {
    console.log('[ConsentAwareLogin] Login-time re-consent view detected; agreeing...')
    const agreeButton = page.locator(SELECTORS.legalConsent.loginAgreeAndContinueButton)
    await Promise.all([
      page.waitForURL(`**/${realmId}/manage**`, { timeout: 15000 }),
      agreeButton.click(),
    ])
    console.log('[ConsentAwareLogin] Re-consent accepted; navigation completed')
  } else {
    console.log('[ConsentAwareLogin] No re-consent gate; already past consent')

    // Ensure we end up on the admin dashboard for the realm when there is no
    // re-consent view. The caller may rely on being inside the admin console.
    const currentUrl = page.url()
    if (!currentUrl.includes(`/${realmId}/manage`)) {
      await page.goto(`${BASE_URL}/${realmId}/manage`, { waitUntil: 'domcontentloaded' })
    }
  }
}
