/**
 * WeChat Pay Provider Configuration Demo (US-WP-001)
 *
 * User Stories (published baselines this WeChat scenario extends):
 * - docs/user-stories/billing/payment-provider.md:
 *   - US-PV-001: Configure payment provider credentials
 *   - US-PV-002: View payment provider configuration
 * WeChat-specific acceptance (US-WP-001: 商户凭据配置、敏感字段脱敏、编辑留空保留)
 * is draft-stage and tracked in the feature workflow until PRD publish.
 *
 * Coverage:
 *   - S1: create WeChat Pay config → provider row listed; reopening the edit
 *     form shows the secret fields EMPTY (masked non-echo) — the user-visible
 *     form of "敏感字段以脱敏方式展示".
 *   - S2: edit with secret fields left empty → save succeeds and the provider
 *     row persists (leave-empty-to-keep retention proof).
 *
 * NOT covered (declared gap):
 *   - US-WP-001 scenario 3 (delete protection under active subscriptions). The
 *     demo admin realm has no active WeChat subscriptions, so the 409 branch
 *     cannot be exercised via the UI here; it is covered by the generic
 *     provider deletion protection and backend tests. Do NOT mask this gap.
 *
 * Assertion discipline:
 *   - Assertions land on PERSISTENT state: provider row testids, the edit
 *     button, and the edit-form input values. Toasts (sonner) are NEVER a
 *     primary assertion (auto-dismissed, locale/volatile).
 *
 * Fixture discipline:
 *   - Unified fixture (`test` / `expect` / `cleanupTestData` from
 *     demo-page.fixtures). `demoLogger` is auto-finalized inside the fixture —
 *     tests MUST NOT call `logger.finalize()` manually.
 *   - `afterEach` runs `cleanupTestData(page, DEMO_ADMIN.realmId, ...)`.
 *
 * Runner:
 *   uv run scripts/demo-test-runner.py \
 *     "demo/e2e/billing-admin/wechat-provider-config-demo.e2e.ts" \
 *     --run-id <RUN_ID> --no-ngrok
 */

import { test, cleanupTestData, expect } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { loginAsAdmin, DEMO_ADMIN } from '../helpers/auth'
import { PaymentProvidersPage } from '../pages/payment-providers-page'
import { SELECTORS } from '../selectors'
import { generateWechatDemoKeyMaterial } from '../helpers/wechat-webhook-simulation'

test.describe('US-WP-001 — WeChat Pay provider configuration', () => {
  let testStartTime: number
  let paymentProvidersPage: PaymentProvidersPage

  test.beforeEach(async ({ page, demoLogger }) => {
    testStartTime = Date.now()
    await verifyTestEnvironment(page, {
      requiredRealms: [DEMO_ADMIN.realmId],
      requiredUsers: [DEMO_ADMIN.email],
    })
    await loginAsAdmin(page, { realmId: DEMO_ADMIN.realmId })
    paymentProvidersPage = new PaymentProvidersPage(page, demoLogger)
    await demoLogger.testCode.log('Admin logged in; environment verified')
  })

  test.afterEach(async ({ page, demoLogger }) => {
    await cleanupTestData(page, DEMO_ADMIN.realmId, {
      keepUsers: [DEMO_ADMIN.email],
      timestamp: testStartTime,
    })
    await demoLogger.testCode.log('Test data cleaned up')
  })

  test('[US-WP-001 S1] create WeChat config, secrets masked on reopen', async ({
    page,
    demoLogger,
  }) => {
    // Realistic key material: the backend parses the merchant key when a
    // client is constructed, and the APIv3 key must be exactly 32 bytes.
    const material = generateWechatDemoKeyMaterial()

    await test.step('Given payment-providers page', async () => {
      await paymentProvidersPage.goto(DEMO_ADMIN.realmId)
      await expect(page.getByTestId('payment-providers-page')).toBeVisible()
      await demoLogger.testCode.log('On payment-providers page')
    })

    await test.step('When create WeChat Pay config', async () => {
      await paymentProvidersPage.configureWechatProvider({
        appId: 'wx-demo-appid-001',
        mchId: '1900000109',
        serialNo: 'DEMO-MCH-SERIAL-0001',
        privateKeyPem: material.merchantPrivateKeyPem,
        apiV3Key: material.apiV3Key,
        notifyUrl: `https://demo.invalid/api/third/pay/${DEMO_ADMIN.realmId}/wechat/webhooks`,
      })
      await demoLogger.testCode.log('WeChat provider created')
    })

    await test.step('Then WeChat provider is listed as configured', async () => {
      await expect(paymentProvidersPage.wechatProviderRow).toBeVisible()
      await expect(paymentProvidersPage.editWechatButton).toBeVisible()
      await demoLogger.testCode.log('WeChat provider row present')
    })

    await test.step('And reopening the edit form shows secrets masked (empty, never echoed)', async () => {
      await paymentProvidersPage.editWechatButton.click()
      await expect(page.locator(SELECTORS.wechatConfigForm.page)).toBeVisible()
      // is_secret values are never echoed back: the private key textarea and
      // the APIv3 key field render EMPTY on edit (脱敏展示).
      await expect(page.locator(SELECTORS.wechatConfigForm.privateKeyInput)).toHaveValue('')
      await expect(page.locator(SELECTORS.wechatConfigForm.v3KeyInput)).toHaveValue('')
      // Non-secret fields DO echo back.
      await expect(page.locator(SELECTORS.wechatConfigForm.appIdInput)).toHaveValue(
        'wx-demo-appid-001',
      )
      await demoLogger.testCode.log('Secrets masked; non-secret fields echoed')
    })
  })

  test('[US-WP-001 S2] edit keeps secrets when left empty', async ({
    page,
    demoLogger,
  }) => {
    const material = generateWechatDemoKeyMaterial()
    const createNotifyUrl = `https://demo.invalid/api/third/pay/${DEMO_ADMIN.realmId}/wechat/webhooks`
    const editedNotifyUrl = `https://demo-edited.invalid/api/third/pay/${DEMO_ADMIN.realmId}/wechat/webhooks`

    await test.step('Given a configured WeChat provider', async () => {
      await paymentProvidersPage.goto(DEMO_ADMIN.realmId)
      await paymentProvidersPage.configureWechatProvider({
        appId: 'wx-demo-appid-001',
        mchId: '1900000109',
        serialNo: 'DEMO-MCH-SERIAL-0001',
        privateKeyPem: material.merchantPrivateKeyPem,
        apiV3Key: material.apiV3Key,
        notifyUrl: createNotifyUrl,
      })
      await expect(paymentProvidersPage.editWechatButton).toBeVisible()
      await demoLogger.testCode.log('WeChat provider created')
    })

    await test.step('When edit leaves secret fields empty and changes notify URL', async () => {
      // editSensitiveLeaveEmpty: private key + APIv3 key are NOT filled. The
      // successful save back to the list is the retention proof (backend keeps
      // the prior secrets when none is sent — is_empty_secret_to_preserve).
      await paymentProvidersPage.configureWechatProvider(
        {
          appId: 'wx-demo-appid-001',
          mchId: '1900000109',
          serialNo: 'DEMO-MCH-SERIAL-0002',
          privateKeyPem: material.merchantPrivateKeyPem, // ignored: skipSensitive
          apiV3Key: material.apiV3Key, // ignored: skipSensitive
          notifyUrl: editedNotifyUrl,
        },
        { editSensitiveLeaveEmpty: true },
      )
      await demoLogger.testCode.log('WeChat provider edited (secrets left empty to keep)')
    })

    await test.step('Then provider row persists (secrets retained)', async () => {
      await expect(paymentProvidersPage.wechatProviderRow).toBeVisible()
      await expect(paymentProvidersPage.editWechatButton).toBeVisible()
      await demoLogger.testCode.log('Edit persisted; provider still configured')
    })

    await test.step('And the non-secret edit is echoed back on reopen', async () => {
      await paymentProvidersPage.editWechatButton.click()
      await expect(page.locator(SELECTORS.wechatConfigForm.page)).toBeVisible()
      await expect(page.locator(SELECTORS.wechatConfigForm.notifyUrlInput)).toHaveValue(
        editedNotifyUrl,
      )
      await demoLogger.testCode.log('Non-secret field updated to new value')
    })
  })
})
