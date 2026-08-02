/**
 * Mixed Consumption Quota Demo Tests (DE-D05)
 *
 * Role: regular-user
 * Route: /{realmId}/user/points (with external API consume)
 *
 * User Story:
 * - US-PU-010 (docs/user-stories/billing/points-user.md) — 滚动窗口额度与充值余额的可用性体验
 *
 * Design contract:
 * - `.ai/design/points-grant-redesign.md` §4.1 / §5.3 / §6.1
 * - `.ai/design-ui/points-grant-redesign/ui-spec.md` §4
 */

import { expect, type Page } from '@playwright/test'

import { test, cleanupTestData } from '../fixtures/demo-page.fixtures'
import { SELECTORS } from '../selectors'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { createBearerApiContext, loginWithCredentials } from '../helpers/auth'
import { listBucketsViaApi } from '../helpers/bucket-helpers'
import {
  createTestApiKeyWithPermission,
  grantPointsViaExtApi,
  type ApiKeyWithPermission,
} from '../helpers/grant-points-helpers'
import { fulfillPayment } from '../helpers/payment-simulation'
import {
  purchaseSubscriptionToGetQuota,
  consumePointsViaExtApi,
  getWindowRemaining,
  getSpendableNow,
} from '../helpers/points-quota-helpers'
import {
  QUOTA_DEMO_REALM,
  QUOTA_DEMO_USER_EMAIL,
  QUOTA_DEMO_PASSWORD,
  QUOTA_DEMO_ADMIN_EMAIL,
} from '../fixtures/points-quota.fixtures'
import { secrets, hasStripePayment } from '../secrets/env'
import { ensureMultiPriceCatalog } from '../helpers/resolve-mappings'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'

// ============================================================================
// Constants
// ============================================================================

const TEST_REALM = QUOTA_DEMO_REALM
const USER_EMAIL = QUOTA_DEMO_USER_EMAIL
const USER_PASSWORD = QUOTA_DEMO_PASSWORD
const ADMIN_EMAIL = QUOTA_DEMO_ADMIN_EMAIL
const ADMIN_PASSWORD = QUOTA_DEMO_PASSWORD
const PROVIDER_HINT = 'stripe'
const TOPUP_GRANT_AMOUNT = 1_000

const SMALLEST_WINDOW_KEY = '5h'

// ============================================================================
// Shared setup context
// ============================================================================

interface SetupContext {
  bucketId: string
  userId: string
  clientAppId: string
  apiKey: ApiKeyWithPermission
}

let setupCtx: SetupContext | null = null
let setupStartTime = 0

// ============================================================================
// Helpers
// ============================================================================

function assertSetup(): SetupContext {
  expect(setupCtx, 'beforeAll must have resolved setup context').not.toBeNull()
  return setupCtx!
}

async function purchaseAndFulfillQuota(page: Page): Promise<void> {
  const attemptId = await purchaseSubscriptionToGetQuota(
    page,
    TEST_REALM,
    USER_EMAIL,
    USER_PASSWORD,
    PROVIDER_HINT,
  )

  const fulfillResult = await fulfillPayment(page.context().request, TEST_REALM, attemptId)
  expect(
    fulfillResult.success,
    `subscription fulfillment failed: ${fulfillResult.error ?? ''}`,
  ).toBeTruthy()
}

async function consumeForTest(amount: number): Promise<{
  status: number
  body: unknown
}> {
  const { apiKey, userId, clientAppId } = assertSetup()
  return consumePointsViaExtApi(apiKey.apiKey, TEST_REALM, {
    userId,
    amount,
    clientAppId,
    description: 'DE-D05 mixed consume',
    idempotencyKey: `de-d05-${setupStartTime}-${Date.now()}`,
  })
}

async function readTopUpBalance(page: Page): Promise<number> {
  const { bucketId } = assertSetup()
  const spendable = await getSpendableNow(page)
  const windowRemaining = await getWindowRemaining(page, bucketId, SMALLEST_WINDOW_KEY)
  return Math.max(0, spendable - windowRemaining)
}

// ============================================================================
// beforeAll — configure mapping, resolve ids, mint API key, seed top-up balance
// ============================================================================

test.beforeAll(async ({ browser }) => {
  // Skip gracefully when Stripe credentials are absent (live dependency).
  test.skip(!hasStripePayment(), 'Stripe credentials required (live test)')
  setupStartTime = Date.now()

  const context = await browser.newContext()
  const page = await context.newPage()

  try {
    const { LoginPage } = await import('../pages/login-page')
    const loginPage = new LoginPage(page)
    await loginPage.loginAsAdmin(ADMIN_EMAIL, ADMIN_PASSWORD, TEST_REALM)

    // Sync the real multi-price catalog into Herald so the purchase flow has a
    // real Stripe subscription product to check out.
    const apiContext = await createBearerApiContext(loginPage.getAccessToken())
    await ensureMultiPriceCatalog(apiContext, {
      baseUrl: BASE_URL,
      realmId: TEST_REALM,
      stripeSecretKey: secrets.stripe.secretKey!,
      stripePublishableKey: secrets.stripe.publishableKey!,
      stripeWebhookSecret: secrets.stripe.webhookSecret!,
    }).finally(() => apiContext.dispose())

    await page.evaluate(() => localStorage.removeItem('cas-purchase-flow'))

    const buckets = await listBucketsViaApi(page, TEST_REALM)
    const primary = buckets.find((b) => b.bucketKey === 'primary-pool')
    if (!primary) {
      throw new Error(`[DE-D05 beforeAll] primary-pool bucket not found in ${TEST_REALM}`)
    }

    const backendUrl =
      process.env.API_BASE_URL ||
      process.env.BASE_URL?.replace(/:\d+/, ':8080') ||
      'http://localhost:8080'
    // `context.request` carries only cookies, not the in-memory Bearer access
    // token (auth-rewrite); admin user/client GETs 401 without the Bearer
    // header. Reuse the same Bearer token the catalog lookup used.
    const adminApi = await createBearerApiContext(loginPage.getAccessToken())
    try {
      const usersResponse = await adminApi.get(
        `${backendUrl}/api/users/${TEST_REALM}?search=${encodeURIComponent(USER_EMAIL)}`,
      )
      let userId = ''
      if (usersResponse.ok()) {
        const usersBody = await usersResponse.json()
        const items = (usersBody?.items ?? []) as { id: string; email: string }[]
        const demoUser = items.find((u) => u.email === USER_EMAIL)
        userId = demoUser?.id ?? ''
      }
      if (!userId) {
        throw new Error(`[DE-D05 beforeAll] Could not resolve demo user UUID for ${USER_EMAIL}`)
      }

      const clientAppResponse = await adminApi.get(
        `${backendUrl}/api/client/${TEST_REALM}`,
      )
      let clientAppId = ''
      if (clientAppResponse.ok()) {
        const clientAppBody = await clientAppResponse.json()
        const items = (clientAppBody?.items ?? []) as { id: string; clientId: string }[]
        const demoApp = items.find((a) => a.clientId === 'points-demo-app')
        clientAppId = demoApp?.id ?? ''
      }
      if (!clientAppId) {
        throw new Error('[DE-D05 beforeAll] Could not resolve client app UUID for points-demo-app')
      }

      // The API-key creation endpoints are also Bearer-only, so route them
      // through the same admin Bearer context instead of the cookie-only default.
      const apiKey = await createTestApiKeyWithPermission(
        page,
        'points.manage',
        setupStartTime,
        TEST_REALM,
        clientAppId,
        adminApi,
      )

      const grant = await grantPointsViaExtApi(apiKey.apiKey, TEST_REALM, {
        userId,
        amount: TOPUP_GRANT_AMOUNT,
        bucketId: primary.id,
        reason: 'DE-D05 setup: deterministic top-up balance for mixed consume',
        validityDays: 365,
      })
      if (grant.status !== 200) {
        throw new Error(
          `[DE-D05 beforeAll] Top-up grant failed: status=${grant.status} body=${JSON.stringify(grant.responseBody)}`,
        )
      }

      setupCtx = {
        bucketId: primary.id,
        userId,
        clientAppId,
        apiKey,
      }
    } finally {
      await adminApi.dispose().catch(() => {})
    }
  } finally {
    await context.close()
  }
})

test.afterEach(async ({ page }) => {
  await cleanupTestData(page, TEST_REALM, {
    keepUsers: [USER_EMAIL],
  })
})

// ============================================================================
// Test suite
// ============================================================================

test.describe('[Regular User] 混合消费协调 (US-PU-010)', () => {
  test.beforeEach(async ({ page, loginPage }) => {
    await verifyTestEnvironment(page, {
      requiredRealms: [TEST_REALM],
      requiredUsers: [USER_EMAIL],
    })

    await loginPage.loginAsUser(USER_EMAIL, USER_PASSWORD, TEST_REALM)
    await purchaseAndFulfillQuota(page)

    await page.goto(`/${TEST_REALM}/user/points`)
    await expect(
      page.locator(SELECTORS.pointsUsageDashboard.page),
    ).toBeVisible({ timeout: 15000 })
  })

  test('US-PU-010 场景3: 窗口额度优先，充值余额不变', async ({ page }) => {
    const { bucketId } = assertSetup()

    const windowBefore = await getWindowRemaining(page, bucketId, SMALLEST_WINDOW_KEY)
    const topUpBefore = await readTopUpBalance(page)

    const consumeAmount = Math.min(50, windowBefore - 1)
    const result = await consumeForTest(consumeAmount)
    expect(result.status).toBe(200)

    await page.reload()
    await expect(
      page.locator(SELECTORS.pointsUsageDashboard.page),
    ).toBeVisible({ timeout: 15000 })

    const windowAfter = await getWindowRemaining(page, bucketId, SMALLEST_WINDOW_KEY)
    const topUpAfter = await readTopUpBalance(page)

    expect(windowAfter).toBe(windowBefore - consumeAmount)
    expect(topUpAfter).toBe(topUpBefore)
  })

  test('US-PU-010 场景3: 超额部分自动转充值余额', async ({ page }) => {
    const { bucketId } = assertSetup()

    const windowBefore = await getWindowRemaining(page, bucketId, SMALLEST_WINDOW_KEY)
    const topUpBefore = await readTopUpBalance(page)
    const consumeAmount = windowBefore + 200

    const result = await consumeForTest(consumeAmount)
    expect(result.status).toBe(200)

    await page.reload()
    await expect(
      page.locator(SELECTORS.pointsUsageDashboard.page),
    ).toBeVisible({ timeout: 15000 })

    const windowAfter = await getWindowRemaining(page, bucketId, SMALLEST_WINDOW_KEY)
    const topUpAfter = await readTopUpBalance(page)

    expect(windowAfter).toBe(0)
    expect(topUpAfter).toBe(topUpBefore - (consumeAmount - windowBefore))

    await expect(
      page.locator(SELECTORS.pointsUsageDashboard.overspendTopupAlert),
    ).toBeVisible()
  })

  test('US-PU-010 场景4: 窗口+充值合计不足时整体拒绝', async ({ page }) => {
    const { bucketId } = assertSetup()

    const windowBefore = await getWindowRemaining(page, bucketId, SMALLEST_WINDOW_KEY)
    const topUpBefore = await readTopUpBalance(page)
    const consumeAmount = windowBefore + topUpBefore + 1_000

    const result = await consumeForTest(consumeAmount)
    expect([400, 402, 409]).toContain(result.status)

    await page.reload()
    await expect(
      page.locator(SELECTORS.pointsUsageDashboard.page),
    ).toBeVisible({ timeout: 15000 })

    const windowAfter = await getWindowRemaining(page, bucketId, SMALLEST_WINDOW_KEY)
    const topUpAfter = await readTopUpBalance(page)

    expect(windowAfter).toBe(windowBefore)
    expect(topUpAfter).toBe(topUpBefore)

    await expect(
      page.locator(SELECTORS.pointsUsageDashboard.insufficientAlert),
    ).toBeVisible()
  })

  test('US-PU-010 场景4: 并发消费不会超额', async ({ page }) => {
    const { bucketId } = assertSetup()

    const windowBefore = await getWindowRemaining(page, bucketId, SMALLEST_WINDOW_KEY)
    const topUpBefore = await readTopUpBalance(page)
    const totalAvailable = windowBefore + topUpBefore

    // Two concurrent requests whose sum exceeds total available.
    const amount1 = Math.ceil(totalAvailable * 0.6)
    const amount2 = Math.ceil(totalAvailable * 0.6)

    const [result1, result2] = await Promise.all([
      consumeForTest(amount1),
      consumeForTest(amount2),
    ])

    await page.reload()
    await expect(
      page.locator(SELECTORS.pointsUsageDashboard.page),
    ).toBeVisible({ timeout: 15000 })

    const windowAfter = await getWindowRemaining(page, bucketId, SMALLEST_WINDOW_KEY)
    const topUpAfter = await readTopUpBalance(page)
    const totalConsumed = windowBefore - windowAfter + (topUpBefore - topUpAfter)

    // At least one request should have been rejected or the total consumed
    // must not exceed the original available balance.
    expect(totalConsumed).toBeLessThanOrEqual(totalAvailable)
    if (result1.status === 200 && result2.status === 200) {
      expect(totalConsumed).toBeLessThan(amount1 + amount2)
    }
  })
})
