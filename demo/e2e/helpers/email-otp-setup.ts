/**
 * Email-OTP Realm Setup Helpers for Demo Tests
 *
 * Convenience wrappers that flip the Email-OTP feature on/off for a realm via
 * the admin Settings UI. These are the "Given Realm has OTP on / off" setup
 * steps shared by the US-EO-001/002 user-flow demos and the US-EO-003
 * admin-config demo (including the degradation assertion that needs OTP off).
 *
 * OTP is enabled purely through the admin UI at test setup — these helpers do
 * NOT edit seed data or SQL.
 *
 * @see ../pages/settings-page.ts (SettingsPage Email-OTP methods)
 * @see ./auth.ts (`loginAsAdmin`)
 */

import { Page } from '@playwright/test'
import type { UnifiedLogger } from './unified-logger'
import { loginAsAdmin, clearSessionData } from './auth'
import { SettingsPage } from '../pages/settings-page'
import { SELECTORS } from '../selectors'

/**
 * Wait for the admin sidebar's Settings entry to be present.
 *
 * After `loginAsAdmin`, the admin lands on the session-scoped `/manage` route
 * (custom-domain dual-mode routing — `realmPath()` returns the bare path for
 * session-scoped segments). The sidebar items are permission-filtered
 * (`filterByPermission` in frontend/src/components/admin/sidebar.tsx) and
 * permissions are hydrated asynchronously post-login, so the Settings entry
 * (which requires PERMISSION.SETTINGS_VIEW) is not yet rendered the instant
 * login returns. This gate lets that settle before `SettingsPage.goto()` clicks
 * it, avoiding a flaky "sidebar-menu-settings not visible" right after login.
 */
async function waitForAdminSidebar(
  page: Page,
  demoLogger: UnifiedLogger,
  realmId: string
): Promise<void> {
  const settingsMenu = page.locator(SELECTORS.sidebar.menuSettings)
  await demoLogger.testCode.log(
    `[EmailOtp Setup] Waiting for admin sidebar (realm "${realmId}")...`
  )
  // NOTE: this wait currently times out in the demo env. After admin login the
  // app redirects to the session-scoped `/manage` route, but the sidebar's
  // Settings entry (permission-filtered, requires SETTINGS_VIEW) does not
  // render. This is a GLOBAL regression affecting every admin demo
  // (confirmed: the unrelated realm-admin-email-config-demo fails identically),
  // introduced outside email-otp-login (custom-domain dual-mode routing e1ec3a98
  // / browser Bearer session f3b8d48a). Not an email-otp defect.
  await settingsMenu.waitFor({ state: 'visible', timeout: 15000 })
}

/**
 * Enable Email-OTP login for a realm (the "Given Realm has OTP on" step).
 *
 * Logs in as the realm admin, opens Settings, switches to the Email-OTP tab,
 * enables OTP, optionally enables auto-register, and saves. Idempotent:
 * `setCheckbox` only clicks when the desired state differs from the current
 * state, so calling this when OTP is already on is a no-op apart from the
 * save (which re-writes the same values).
 *
 * @param page        Playwright Page.
 * @param demoLogger  UnifiedLogger from the test fixture.
 * @param realmId     Target realm id.
 * @param options     `autoRegister` — when true, also toggles the
 *                    auto-register switch on before saving.
 */
export async function enableEmailOtpForRealm(
  page: Page,
  demoLogger: UnifiedLogger,
  realmId: string,
  options: { autoRegister?: boolean } = {}
): Promise<void> {
  const { autoRegister = false } = options

  demoLogger.testCode.log(`[EmailOtp Setup] Enabling Email-OTP for realm "${realmId}" (autoRegister=${autoRegister})`)

  await loginAsAdmin(page, { realmId })

  await waitForAdminSidebar(page, demoLogger, realmId)

  const settingsPage = new SettingsPage(page, demoLogger, realmId)
  await settingsPage.goto()
  await settingsPage.waitForReady()
  await settingsPage.switchToEmailOtpTab()

  await settingsPage.enableEmailOtp()
  if (autoRegister) {
    await settingsPage.enableAutoRegister()
  }
  await settingsPage.saveEmailOtpConfig()

  // Leave a clean unauthenticated state: the caller's next step is typically
  // `goto /realm/auth/login`, which the root loader redirects to /manage while
  // the admin session is still alive (login card never renders).
  await clearSessionData(page)

  demoLogger.testCode.log(`[EmailOtp Setup] Email-OTP enabled for realm "${realmId}"`)
}

/**
 * Disable Email-OTP login for a realm (best-effort teardown / degradation
 * setup).
 *
 * Logs in as the realm admin, opens Settings, switches to the Email-OTP tab,
 * disables OTP and auto-register, and saves. Wrapped in try/catch so a
 * teardown failure never hard-fails the run — it logs and continues.
 *
 * @param page        Playwright Page.
 * @param demoLogger  UnifiedLogger from the test fixture.
 * @param realmId     Target realm id.
 */
export async function disableEmailOtpForRealm(
  page: Page,
  demoLogger: UnifiedLogger,
  realmId: string
): Promise<void> {
  try {
    demoLogger.testCode.log(`[EmailOtp Setup] Disabling Email-OTP for realm "${realmId}"`)

    await loginAsAdmin(page, { realmId })

    await waitForAdminSidebar(page, demoLogger, realmId)

    const settingsPage = new SettingsPage(page, demoLogger, realmId)
    await settingsPage.goto()
    await settingsPage.waitForReady()
    await settingsPage.switchToEmailOtpTab()

    await settingsPage.disableEmailOtp()
    await settingsPage.disableAutoRegister()
    await settingsPage.saveEmailOtpConfig()

    // Match enableEmailOtpForRealm: clear the admin session so teardown leaves
    // a clean state for the next test.
    await clearSessionData(page)

    demoLogger.testCode.log(`[EmailOtp Setup] Email-OTP disabled for realm "${realmId}"`)
  } catch (error) {
    // Teardown / degradation setup must never hard-fail the test run.
    console.warn(`[EmailOtp Setup] Failed to disable Email-OTP for realm "${realmId}":`, error)
  }
}
