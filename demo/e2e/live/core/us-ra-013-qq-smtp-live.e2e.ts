/**
 * Live QQ SMTP Email Test (Manual-Assisted)
 *
 * Related User Stories: US-RA-013 (配置 Realm 邮件服务), US-RA-014 (发送测试邮件)
 * Coverage: partial live smoke; covers US-RA-013 场景2 (SMTP 配置) and
 *   US-RA-014 场景1 (测试邮件发送成功). User story source:
 *   docs/user-stories/core/realm-admin.md (US-RA-013 at L528, US-RA-014 at L585).
 * Not Covered: Resend provider, UI form interactions, missing-field validation
 *   (US-RA-013 场景3), provider switch clearing (US-RA-013 场景4), unconfigured
 *   rejection path (US-RA-014 场景2), failure error UI (US-RA-014 场景3).
 * Live Dependency: real QQ Mail SMTP server reachable + correct 16-char
 *   authorization code (授权码, NOT the login password).
 * Manual Step: none for the API-driven flow; recipient receives the test email
 *   out-of-band (no inbox check is performed).
 * Run Command:
 *   cd demo
 *   npx playwright test e2e/live/core/us-ra-013-qq-smtp-live.e2e.ts --project=demo-fast
 * Skip/Fail Policy:
 *   Skips (test.skip) when QQ SMTP credentials are absent — so the rest of the
 *   demo suite still runs. When credentials ARE present, fails loud on any
 *   backend error (network failures, wrong authorization code, QQ SMTP rate
 *   limiting, etc.) and prints the backend error message — it does NOT silently
 *   pass or skip.
 *
 * Validates the live QQ SMTP path end-to-end via API (stable, no UI form
 * typing required). Mirrors the existing live demo approach (GitHub OAuth demo
 * also seeds real third-party credentials through API helpers and asserts on
 * API responses). The frontend SMTP form testids exist
 * (email-provider-smtp / email-provider-resend / email-config-status-badge /
 * email-status-error in frontend/src/components/realm-config/email-config-form.tsx)
 * but are intentionally not driven here — API seeding is more stable and is the
 * convention used by sibling live demos.
 *
 * =============================================================================
 * HOW TO RUN
 * =============================================================================
 *
 * 1. Start backend and frontend
 *
 *    # Terminal 1: backend (port 8080)
 *    cd backend && cargo run
 *
 *    # Terminal 2: frontend (port 3000)
 *    cd frontend && npm run dev
 *
 * 2. Get a QQ Mail SMTP authorization code:
 *
 *    a) Log in to QQ Mail at https://mail.qq.com
 *    b) Settings (设置) → Account (账户)
 *    c) Under "POP3/IMAP/SMTP/Exchange/CardDAV/CalDAV Service", enable
 *       "IMAP/SMTP Service"
 *    d) Click "Generate Authorization Code" (生成授权码) — a 16-char string
 *       is returned. This is NOT your QQ login password.
 *
 * 3. Set in demo/.env.demo:
 *
 *    QQ_SMTP_HOST=smtp.qq.com
 *    QQ_SMTP_PORT=465
 *    QQ_SMTP_USERNAME=your-qq@qq.com      # full mailbox
 *    QQ_SMTP_PASSWORD=<16-char authorization code>
 *    QQ_SMTP_ENCRYPTION=ssl               # or "starttls" with port 587
 *    QQ_FROM_ADDRESS=your-qq@qq.com       # optional, defaults to QQ_SMTP_USERNAME
 *    QQ_TEST_RECIPIENT=someone@example.com # optional, defaults to QQ_SMTP_USERNAME
 *
 * 4. Run:
 *
 *    cd demo
 *    npx playwright test e2e/live/core/us-ra-013-qq-smtp-live.e2e.ts --project=demo-fast
 *
 * =============================================================================
 */

import { test, expect } from '../../fixtures/demo-auth.fixtures'
import { secrets, hasQqSmtp } from '../../secrets/env'
import { seedEmailSmtpConfig } from '../../secrets/realm-seed'
import { loginAsAdmin } from '../../helpers/auth'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'
const REALM_ID = 'admin'

// QQ SMTP defaults (applied only when explicit env overrides are absent).
const DEFAULT_QQ_HOST = 'smtp.qq.com'
const DEFAULT_QQ_PORT = '465'
const DEFAULT_QQ_ENCRYPTION = 'ssl'

// Config keys seeded by seedEmailSmtpConfig; used for cleanup DELETE calls.
// Mirrors herald_core::third::email::read_email_config key expectations.
const EMAIL_CONFIG_KEYS = [
  'provider',
  'from_address',
  'smtp_host',
  'smtp_port',
  'smtp_username',
  'smtp_password',
  'smtp_encryption',
] as const

/** Resolve the QQ SMTP seed input from env, applying QQ defaults where unset. */
function resolveQqSmtpInput() {
  const smtpUsername = secrets.qq.smtpUsername!
  return {
    smtpHost: secrets.qq.smtpHost || DEFAULT_QQ_HOST,
    smtpPort: secrets.qq.smtpPort || DEFAULT_QQ_PORT,
    smtpUsername,
    smtpPassword: secrets.qq.smtpPassword!,
    smtpEncryption: secrets.qq.smtpEncryption || DEFAULT_QQ_ENCRYPTION,
    fromAddress: secrets.qq.fromAddress || smtpUsername,
  }
}

/** Best-effort cleanup: delete every seeded email config_key for the realm. */
async function cleanupEmailConfig(request: import('@playwright/test').APIRequestContext): Promise<void> {
  for (const key of EMAIL_CONFIG_KEYS) {
    try {
      const response = await request.delete(
        `${BASE_URL}/api/configs/${REALM_ID}/email/${key}`,
      )
      // 204 = deleted, 404 = nothing to delete — both fine.
      if (!response.ok() && response.status() !== 404) {
        console.log(`[cleanup] email/${key} delete returned ${response.status()}`)
      }
    } catch (error) {
      // Cleanup is best-effort; never fail the run on cleanup errors.
      console.error(`[cleanup] Failed to delete email/${key}:`, error)
    }
  }
  console.log('[cleanup] email config cleanup complete')
}

test.describe('[Live][Core Email] US-RA-013/014: QQ SMTP config and test email', () => {
  test.beforeEach(async ({ page, demoLogger }) => {
    test.skip(!hasQqSmtp(), 'QQ SMTP credentials not configured in demo/.env.demo')

    await test.step('Setup: seed QQ SMTP email config', async () => {
      await loginAsAdmin(page, { realmId: REALM_ID })
      await seedEmailSmtpConfig(page.request, REALM_ID, resolveQqSmtpInput())
      demoLogger.testCode.log('[Test] ✓ QQ SMTP email config seeded')
    })
  })

  test.afterEach(async ({ page, demoLogger }) => {
    try {
      // page may already be authenticated from beforeEach; only DELETE if so.
      const cookies = await page.context().cookies()
      const hasAuth = cookies.some((c) => c.name === 'X-Auth' || c.name === 'auth_token')
      if (!hasAuth) {
        await loginAsAdmin(page, { realmId: REALM_ID })
      }
      await cleanupEmailConfig(page.request)
      demoLogger.testCode.log('[Test] ✓ email config cleanup complete')
    } catch (error) {
      demoLogger.testCode.log(`[Test] ✗ cleanup failed during email config cleanup: ${error}`)
      console.error('[cleanup] Failed during email config cleanup:', error)
    }
  })

  test('US-RA-013 Scenario 2: QQ SMTP config saved and status shows configured', async ({
    page,
    demoLogger,
  }) => {
    // Given the QQ SMTP config was seeded in beforeEach, the status endpoint
    // must report the realm as configured.
    await test.step('When querying email status', async () => {
      const response = await page.request.get(
        `${BASE_URL}/api/configs/${REALM_ID}/email/status`,
      )
      expect(
        response.ok(),
        `email/status returned ${response.status()}: ${await response.text()}`,
      ).toBeTruthy()

      const status = await response.json()
      demoLogger.testCode.log(`[Test] ✓ email status: ${JSON.stringify(status)}`)

      // Then the realm is reported configured with provider=smtp and a from_address
      expect(status.configured).toBe(true)
      expect(status.provider).toBe('smtp')
      expect(typeof status.from_address).toBe('string')
      expect(status.from_address.length).toBeGreaterThan(0)
      expect(Array.isArray(status.missing_fields)).toBe(true)
      expect(status.missing_fields).toEqual([])
      demoLogger.testCode.log('[Test] ✓ status reports configured / smtp / from_address present')
    })
  })

  test('US-RA-014 Scenario 1: QQ SMTP test email sends successfully', async ({
    page,
    demoLogger,
  }) => {
    const recipient =
      secrets.qq.testRecipient || secrets.qq.smtpUsername!

    // Given the realm is configured (beforeEach seed), the test-email endpoint
    // must accept a real recipient and report success.
    await test.step('When sending a test email through the QQ SMTP server', async () => {
      const response = await page.request.post(
        `${BASE_URL}/api/configs/${REALM_ID}/email/test`,
        {
          data: { recipient },
          // Real SMTP send to QQ's server: TLS handshake + delivery can take
          // time; give a generous upper bound (default Playwright timeout is
          // 30s which is too tight on a cold connection).
          timeout: 60_000,
        },
      )

      // Fail loud on non-2xx (rate-limit 429, auth 535, etc.) — print the
      // backend message so the operator can diagnose the SMTP failure.
      if (!response.ok()) {
        const body = await response.text()
        demoLogger.testCode.log(`[Test] ✗ email/test returned ${response.status()} for recipient ${recipient}: ${body}`)
        throw new Error(
          `email/test returned ${response.status()} for recipient ${recipient}: ${body}`,
        )
      }

      const result = await response.json()
      demoLogger.testCode.log(`[Test] ✓ email/test result: ${JSON.stringify(result)}`)

      // Then the backend reports success. A false-y success here means SMTP
      // send failed server-side — fail loud with the backend's message.
      if (result.success !== true) {
        demoLogger.testCode.log(`[Test] ✗ test email failed to send via QQ SMTP. Backend message: ${result.message ?? '(none)'}`)
        throw new Error(
          `Test email failed to send via QQ SMTP. Backend message: ${result.message ?? '(none)'}`,
        )
      }
      expect(result.success).toBe(true)
      expect(typeof result.message).toBe('string')
      demoLogger.testCode.log('[Test] ✓ test email sent successfully via QQ SMTP')
    })
  })
})
