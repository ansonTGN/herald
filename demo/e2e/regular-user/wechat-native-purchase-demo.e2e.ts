/**
 * WeChat Pay Native (PC QR) Purchase + Callback Pipeline Demo
 *
 * User Stories (published baselines this WeChat scenario extends):
 * - docs/user-stories/billing/payment-attempt.md:
 *   - US-PA-001..004: payment attempt lifecycle (create/poll/fulfil/close)
 * WeChat-specific acceptance (US-WP-002 PC 扫码 Native 支付; US-WP-004 回调
 * 验签/解密/幂等履约) is draft-stage and tracked in the feature workflow
 * until PRD publish.
 *
 * Coverage:
 *   - US-WP-002 S1: native order → QR + countdown → (simulated WeChat
 *     callback through the REAL webhook endpoint) → succeeded UI.
 *   - US-WP-002 S3: payment failure → failed UI + repay entry.
 *   - US-WP-004 S1: legally signed + encrypted callback fulfils the attempt.
 *   - US-WP-004 S2: invalid signature → 422 FAIL, attempt stays Pending.
 *   - US-WP-004 S3: amount mismatch → 422 FAIL, attempt stays Pending.
 *   - US-WP-004 S4: duplicate callback → idempotent 200, no state regression.
 *
 * Demo mechanism (no real WeChat merchant needed):
 *   - The unified order goes to a local v3 mock (`base_url` realm_config
 *     override seeded via `setupWechatPayRealm`); the mock records the order
 *     (out_trade_no + amount) for the forged callback.
 *   - Callbacks are forged per the WeChat v3 protocol: RSA-SHA256 signature
 *     over `{timestamp}\n{nonce}\n{body}\n` with the platform keypair whose
 *     public half is seeded as the `platform_public_key` override, and an
 *     AES-256-GCM encrypted resource keyed by the APIv3 key — the backend's
 *     real verify/decrypt/idempotency/amount code paths all execute.
 *
 * NOT covered (declared gaps):
 *   - US-WP-002 S2 (QR expiry): the real QR lifetime is hours (≤2h by design);
 *     expiry UI is covered by frontend unit tests. Demo asserts the countdown
 *     renders.
 *   - US-WP-002 S3 literal `Failed` status: no backend path produces Failed
 *     for wechat (the callback only fulfils on trade_state=SUCCESS; failure
 *     discovery relies on WeChat retry + compensation). The
 *     demo drives the REAL abandonment flow instead — user cancel → Cancelled
 *     status, no fulfilment, repurchase available. The failed-status BRANCH
 *     rendering is frontend-owned UI (payment-status-failed testid).
 *   - Points balance visibility: fulfilment correctness is asserted via the
 *     succeeded status (driven by complete_succeeded_payment_attempt); grant
 *     accounting is owned by backend scenario tests.
 *
 * Assertion discipline:
 *   - Persistent state only: QR/status regions, webhook HTTP status + body,
 *     mock-recorded order bodies. No toast assertions.
 *
 * Runner:
 *   uv run scripts/demo-test-runner.py \
 *     "demo/e2e/regular-user/wechat-native-purchase-demo.e2e.ts" \
 *     --run-id <RUN_ID> --no-ngrok
 */

import { test, cleanupTestData, expect } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { loginWithCredentials } from '../helpers/auth'
import { SELECTORS } from '../selectors'
import { TEST_DATA, initiatePurchaseFlow } from '../helpers/unified-purchase.helpers'
import {
  setupWechatPayRealm,
  WECHAT_DEMO,
  type WechatPayDemoEnv,
} from '../helpers/wechat-demo-setup'
import {
  deliverWechatCallback,
  forgeWechatCallback,
  generateWechatRsaKeyPair,
} from '../helpers/wechat-webhook-simulation'
import type { Page } from '@playwright/test'

const REALM_ID = TEST_DATA.REALMS.REALM_001
const USER_EMAIL = TEST_DATA.USERS.USER_REALM_001

/**
 * Lazily-initialized shared env: the mock server and seeded config (base_url
 * port + platform override) are created once for the whole file and reused by
 * every test — the mock must stay alive for the entire file lifetime.
 */
let env: WechatPayDemoEnv | null = null

test.describe('[Regular User] wechat-support — Native QR purchase + WeChat callback pipeline', () => {
  let testStartTime: number

  test.beforeEach(async ({ page, loginPage, demoLogger }) => {
    testStartTime = Date.now()
    await verifyTestEnvironment(page, {
      requiredRealms: [REALM_ID],
      requiredUsers: [USER_EMAIL],
    })

    if (!env) {
      env = await setupWechatPayRealm(page, loginPage, REALM_ID)
      await demoLogger.testCode.log(`WeChat demo env seeded (mock ${env.mock.url})`)
    }

    await loginWithCredentials(page, {
      realmId: REALM_ID,
      email: USER_EMAIL,
      password: TEST_DATA.CREDENTIALS.DEFAULT_PASSWORD,
    })
    // Regular users land on the session-scoped profile route post-login.
    await page.waitForURL('**/user/profile**')
    await demoLogger.testCode.log('Regular user logged in')
  })

  test.afterEach(async ({ page, demoLogger }) => {
    await cleanupTestData(page, REALM_ID, {
      keepUsers: [USER_EMAIL],
      timestamp: testStartTime,
    })
    await demoLogger.testCode.log('Test data cleaned up')
  })

  test.afterAll(async () => {
    await env?.mock.close()
    env = null
  })

  // ==========================================================================
  // US-WP-002 S1 + US-WP-004 S1: happy path — QR then legally signed callback
  // ==========================================================================

  test('[US-WP-002 S1 / US-WP-004 S1] native order renders QR; signed callback fulfils it', async ({
    page,
    demoLogger,
  }) => {
    const attemptId = await startWechatNativeAttempt(page)
    const material = env!.material

    await test.step('Then the WeChat QR with countdown is rendered (no redirect)', async () => {
      await expect(page.locator(SELECTORS.wechatQrPayment.container)).toBeVisible()
      await expect(page.locator(SELECTORS.wechatQrPayment.code)).toBeVisible()
      await expect(page.locator(SELECTORS.wechatQrPayment.countdown)).toBeVisible()
      // Native flow must NOT redirect away (stays on the processing step).
      await expect(page.locator(SELECTORS.purchasePoints.stepProcessing)).toBeVisible()
      await demoLogger.testCode.log(`QR rendered for attempt ${attemptId}`)
    })

    const order = env!.mock.nativeOrders[env!.mock.nativeOrders.length - 1]
    await test.step('And the unified order hit the v3 mock with the mapped price', async () => {
      expect(order).toBeDefined()
      // provider_product_info.price of the seeded demo mapping, in fen.
      expect(order!.amountTotal).toBe(WECHAT_DEMO.AMOUNT_FEN)
      // out_trade_no is the attempt's provider_reference — the callback key.
      expect(order!.outTradeNo).toMatch(/^CAS_/)
      await demoLogger.testCode.log(`Mock recorded native order ${order!.outTradeNo}`)
    })

    await test.step('When a legally signed WeChat callback arrives (US-WP-004 S1)', async () => {
      const response = await deliverWechatCallback(
        page.request,
        REALM_ID,
        forgeWechatCallback({
          eventId: `evt-demo-${attemptId}`,
          outTradeNo: order!.outTradeNo,
          transactionId: `demo-txn-${attemptId}`,
          tradeState: 'SUCCESS',
          amountTotal: WECHAT_DEMO.AMOUNT_FEN,
          apiV3Key: material.apiV3Key,
          platformPrivateKeyPem: material.platform.privateKeyPem,
        }),
      )
      expect(response.status()).toBe(200)
      await expect(response.json()).resolves.toMatchObject({ code: 'SUCCESS' })
      await demoLogger.testCode.log('Signed callback accepted (200 SUCCESS)')
    })

    await test.step('Then the page flips to the purchase-complete step', async () => {
      // On success the purchase page transitions from processing to the
      // complete step (`purchase-step-complete`) and stops polling — that is
      // the user-visible success state (payment-status-succeeded only renders
      // while still on the processing step).
      await expect(page.locator(SELECTORS.purchasePoints.stepComplete)).toBeVisible({
        timeout: 15000,
      })
      await demoLogger.testCode.log('Attempt flipped to succeeded (complete step)')
    })
  })

  // ==========================================================================
  // US-WP-004 S2: signature verification failure
  // ==========================================================================

  test('[US-WP-004 S2] callback with invalid signature is rejected, attempt stays pending', async ({
    page,
    demoLogger,
  }) => {
    const attemptId = await startWechatNativeAttempt(page)
    const material = env!.material
    const order = env!.mock.nativeOrders[env!.mock.nativeOrders.length - 1]

    await test.step('When a callback signed by a ROGUE platform key arrives', async () => {
      const rogue = generateWechatRsaKeyPair()
      const response = await deliverWechatCallback(
        page.request,
        REALM_ID,
        forgeWechatCallback({
          eventId: `evt-rogue-${attemptId}`,
          outTradeNo: order.outTradeNo,
          transactionId: `demo-txn-rogue-${attemptId}`,
          tradeState: 'SUCCESS',
          amountTotal: WECHAT_DEMO.AMOUNT_FEN,
          apiV3Key: material.apiV3Key,
          platformPrivateKeyPem: material.platform.privateKeyPem,
          signingPrivateKeyPem: rogue.privateKeyPem,
        }),
      )
      // Security rejection: 422 FAIL, no state mutated.
      expect(response.status()).toBe(422)
      await expect(response.json()).resolves.toMatchObject({ code: 'FAIL' })
      await demoLogger.testCode.log('Rogue-signed callback rejected (422 FAIL)')
    })

    await test.step('Then the QR pending UI is unchanged (attempt never mutated)', async () => {
      await expect(page.locator(SELECTORS.wechatQrPayment.container)).toBeVisible()
      await expect(page.locator(SELECTORS.paymentStatus.succeeded)).toHaveCount(0)
      await demoLogger.testCode.log('Attempt still pending after rejected callback')
    })
  })

  // ==========================================================================
  // US-WP-004 S3: amount mismatch
  // ==========================================================================

  test('[US-WP-004 S3] callback amount mismatch is rejected, attempt stays pending', async ({
    page,
    demoLogger,
  }) => {
    const attemptId = await startWechatNativeAttempt(page)
    const material = env!.material
    const order = env!.mock.nativeOrders[env!.mock.nativeOrders.length - 1]

    await test.step('When a correctly signed callback carries the WRONG amount', async () => {
      const response = await deliverWechatCallback(
        page.request,
        REALM_ID,
        forgeWechatCallback({
          eventId: `evt-amount-${attemptId}`,
          outTradeNo: order.outTradeNo,
          transactionId: `demo-txn-amount-${attemptId}`,
          tradeState: 'SUCCESS',
          amountTotal: WECHAT_DEMO.AMOUNT_FEN + 1, // mismatch by 1 fen
          apiV3Key: material.apiV3Key,
          platformPrivateKeyPem: material.platform.privateKeyPem,
        }),
      )
      expect(response.status()).toBe(422)
      await expect(response.json()).resolves.toMatchObject({ code: 'FAIL' })
      await demoLogger.testCode.log('Amount-mismatch callback rejected (422 FAIL)')
    })

    await test.step('Then the QR pending UI is unchanged', async () => {
      await expect(page.locator(SELECTORS.wechatQrPayment.container)).toBeVisible()
      await expect(page.locator(SELECTORS.paymentStatus.succeeded)).toHaveCount(0)
      await demoLogger.testCode.log('Attempt still pending after amount mismatch')
    })
  })

  // ==========================================================================
  // US-WP-004 S4: duplicate callback idempotency
  // ==========================================================================

  test('[US-WP-004 S4] duplicate callback is idempotent (no double fulfilment)', async ({
    page,
    demoLogger,
  }) => {
    const attemptId = await startWechatNativeAttempt(page)
    const material = env!.material
    const order = env!.mock.nativeOrders[env!.mock.nativeOrders.length - 1]

    const forged = forgeWechatCallback({
      eventId: `evt-dup-${attemptId}`,
      outTradeNo: order.outTradeNo,
      transactionId: `demo-txn-dup-${attemptId}`,
      tradeState: 'SUCCESS',
      amountTotal: WECHAT_DEMO.AMOUNT_FEN,
      apiV3Key: material.apiV3Key,
      platformPrivateKeyPem: material.platform.privateKeyPem,
    })

    await test.step('Given the first callback fulfils the attempt', async () => {
      const first = await deliverWechatCallback(page.request, REALM_ID, forged)
      expect(first.status()).toBe(200)
      await expect(page.locator(SELECTORS.purchasePoints.stepComplete)).toBeVisible({
        timeout: 15000,
      })
      await demoLogger.testCode.log('First delivery fulfilled the attempt')
    })

    await test.step('When WeChat re-delivers the SAME event (same id + same bytes)', async () => {
      const second = await deliverWechatCallback(page.request, REALM_ID, forged)
      // Idempotent ack: 200 SUCCESS, recognised as already processed.
      expect(second.status()).toBe(200)
      await expect(second.json()).resolves.toMatchObject({ code: 'SUCCESS' })
      await demoLogger.testCode.log('Duplicate delivery acked idempotently')
    })

    await test.step('Then the completed state is stable (no double fulfilment)', async () => {
      await expect(page.locator(SELECTORS.purchasePoints.stepComplete)).toBeVisible()
      await expect(page.locator(SELECTORS.purchasePoints.stepProcessing)).toHaveCount(0)
      await demoLogger.testCode.log('Completed state stable after duplicate callback')
    })
  })

  // ==========================================================================
  // US-WP-002 S4: non-renewing membership — fixed period, repeatable
  // ==========================================================================

  test('[US-WP-002 S4] non-renewing membership purchase is repeatable (no auto-renewal)', async ({
    page,
    demoLogger,
  }) => {
    await test.step('When the user buys the 30-day membership via WeChat', async () => {
      await completeWechatMembershipPurchase(page)
      await demoLogger.testCode.log('First membership purchase completed')
    })

    await test.step('Then the SAME membership can be purchased again (repeatable, not gated)', async () => {
      // Non-renewing fulfilment creates a fixed-period subscription with no
      // auto-renewal; re-purchase is the renewal path. The second run must
      // offer the same card purchasable and complete again.
      await completeWechatMembershipPurchase(page)
      await demoLogger.testCode.log('Second membership purchase completed (repeatable)')
    })
  })

  // ==========================================================================
  // US-WP-002 S3: abandonment feedback (user cancel — see header note on why
  // the literal Failed status is not backend-reachable for wechat)
  // ==========================================================================

  test('[US-WP-002 S3] cancelled payment shows terminal state, no fulfilment, repurchase available', async ({
    page,
    demoLogger,
  }) => {
    await startWechatNativeAttempt(page)

    await test.step('Given the QR is rendered', async () => {
      await expect(page.locator(SELECTORS.wechatQrPayment.container)).toBeVisible()
    })

    await test.step('When the user cancels the payment from the QR page', async () => {
      await page.locator(SELECTORS.paymentStatus.cancelButton).click()
      await demoLogger.testCode.log('Cancel clicked on QR page')
    })

    await test.step('Then the terminal (payment) step returns and no fulfilment happened', async () => {
      // Cancel transitions the page back to the payment step and clears the
      // purchase state (the cancelled branch never renders the processing
      // status region on this page).
      await expect(page.locator(SELECTORS.purchasePoints.stepPayment)).toBeVisible({
        timeout: 15000,
      })
      await expect(page.locator(SELECTORS.purchasePoints.stepProcessing)).toHaveCount(0)
      await expect(page.locator(SELECTORS.purchasePoints.stepComplete)).toHaveCount(0)
      await demoLogger.testCode.log('Cancelled; back on payment step; not fulfilled')
    })

    await test.step('And the user can start a new purchase (repurchase entry)', async () => {
      // The payment step carries the Next button that re-initiates the order —
      // the repurchase entry point after an abandoned payment.
      await expect(page.locator(SELECTORS.purchasePoints.nextButton)).toBeVisible()
      await demoLogger.testCode.log('Repurchase entry available')
    })
  })
})

// ---------------------------------------------------------------------------
// Shared flow helper
// ---------------------------------------------------------------------------

/**
 * Drive the UI to a pending WeChat Native attempt and return its id.
 *
 * Delegates to `initiatePurchaseFlow` pinned to a seeded WeChat demo price
 * card instead of "first card" (realm-001 also has Stripe cards), then waits
 * for the QR pending state specific to the Native scene.
 */
async function startWechatNativeAttempt(
  page: Page,
  priceId: string = WECHAT_DEMO.PRICE_ID,
): Promise<string> {
  const attemptId = await initiatePurchaseFlow(page, 'wechat', REALM_ID, { priceId })
  await expect(page.locator(SELECTORS.wechatQrPayment.container)).toBeVisible({
    timeout: TEST_DATA.TIMEOUTS.ELEMENT_VISIBLE,
  })
  return attemptId
}

/**
 * Full membership round-trip: order the seeded non-renewing membership card,
 * fulfil it via a forged signed callback, and wait for the complete step.
 * Throws loudly on any protocol step failing (assertions inside).
 */
async function completeWechatMembershipPurchase(page: Page): Promise<void> {
  const attemptId = await startWechatNativeAttempt(page, WECHAT_DEMO.MEMBERSHIP_PRICE_ID)
  const material = env!.material
  const order =
    env!.mock.nativeOrders[env!.mock.nativeOrders.length - 1]
  expect(order.amountTotal).toBe(WECHAT_DEMO.MEMBERSHIP_AMOUNT_FEN)

  const response = await deliverWechatCallback(
    page.request,
    REALM_ID,
    forgeWechatCallback({
      eventId: `evt-membership-${attemptId}`,
      outTradeNo: order.outTradeNo,
      transactionId: `demo-txn-membership-${attemptId}`,
      tradeState: 'SUCCESS',
      amountTotal: WECHAT_DEMO.MEMBERSHIP_AMOUNT_FEN,
      apiV3Key: material.apiV3Key,
      platformPrivateKeyPem: material.platform.privateKeyPem,
    }),
  )
  expect(response.status()).toBe(200)
  await expect(page.locator(SELECTORS.purchasePoints.stepComplete)).toBeVisible({
    timeout: 15000,
  })
}
