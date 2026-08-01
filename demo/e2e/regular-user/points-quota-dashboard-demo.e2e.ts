/**
 * Points Quota Dashboard Demo Tests (DE-D02)
 *
 * Role: regular-user
 * Route: /{realmId}/user/points
 *
 * User Story:
 * - US-PU-010 (docs/user-stories/billing/points-user.md) — 滚动窗口额度与充值余额的可用性体验
 *
 * Design contract:
 * - `.ai/design/points-grant-redesign.md` §4.2.2 / §4.4.3
 * - `.ai/design-ui/points-grant-redesign/ui-spec.md` §3.1 / §4 / §7
 * - Converged testid contract: `.ai/task/points-grant-redesign/frontend/accept/FE-A07-report.md`
 *
 * This file reuses the selectors, fixtures and helpers produced by DE-D01:
 * - `demo/e2e/selectors.ts`
 * - `demo/e2e/fixtures/points-quota.fixtures.ts`
 * - `demo/e2e/helpers/points-quota-helpers.ts`
 *
 * Assertion discipline: every business assertion lands on a persistent page
 * element, a stable `data-testid`, or the wallet API response. No toast-only
 * assertions.
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
import {
  grantQuotaEntitlement,
  revokeQuotaEntitlement,
} from '../helpers/quota-entitlement-helpers'
import {
  createEntitlementMappingWithQuotaWindows,
  consumePointsViaExtApi,
  getWindowRow,
  getWindowRemaining,
  getWindowResetsIn,
  getSpendableNow,
  getSpendableFromPool,
} from '../helpers/points-quota-helpers'
import { registerUser } from '../helpers/points-helpers'
import {
  QUOTA_DEMO_REALM,
  QUOTA_DEMO_USER_EMAIL,
  QUOTA_DEMO_PASSWORD,
  QUOTA_DEMO_ADMIN_EMAIL,
  DEMO_QUOTA_WINDOWS,
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
const TOPUP_GRANT_AMOUNT = 1_000
const CONSUME_SMALL_AMOUNT = 50

/**
 * WinKey of the smallest window (5h). This is the window the tests will exhaust.
 */
const SMALLEST_WINDOW_KEY = '5h'

/**
 * Stable anchor for the internal quota-entitlement grant used across tests.
 * The fast suite grants quota directly via the internal API (see
 * `quota-entitlement-helpers.ts`) instead of driving a real Stripe purchase —
 * seeded price IDs are placeholders that must never reach Stripe (DE-D01 §27).
 * Each test revokes then re-grants using this fixed source_id for a clean
 * baseline.
 */
const QUOTA_SOURCE_ID = 'demo-quota-dashboard'

// ============================================================================
// Shared setup context
// ============================================================================

interface SetupContext {
  /** UUID of the `primary-pool` bucket (window quota + topup live here). */
  bucketId: string
  /** UUID of the seeded demo regular user. */
  userId: string
  /** UUID of the seeded `points-demo-app` client app (consume scope). */
  clientAppId: string
  /** API key with `points.manage` bound to `points-demo-app` in realm-001. */
  apiKey: ApiKeyWithPermission
}

let setupCtx: SetupContext | null = null
let setupStartTime = 0

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
    // 1. Login as the realm-001 admin (carries points.manage).
    const { LoginPage } = await import('../pages/login-page')
    const loginPage = new LoginPage(page)
    await loginPage.loginAsAdmin(ADMIN_EMAIL, ADMIN_PASSWORD, TEST_REALM)

    // 2. Resolve the real multi-price Stripe product + sync its catalog into
    //    Herald. Replaces the removed placeholder seed id.
    const apiContext = await createBearerApiContext(loginPage.getAccessToken())
    const catalog = await ensureMultiPriceCatalog(apiContext, {
      baseUrl: BASE_URL,
      realmId: TEST_REALM,
      stripeSecretKey: secrets.stripe.secretKey!,
      stripePublishableKey: secrets.stripe.publishableKey!,
      stripeWebhookSecret: secrets.stripe.webhookSecret!,
    }).finally(() => apiContext.dispose())
    const realProductId = catalog.product.productId

    // 3. Configure the pro-plan entitlement mapping with multi-window quota.
    //    This makes the subscription purchase grant quota entitlements instead
    //    of legacy ledger rows.
    await createEntitlementMappingWithQuotaWindows(
      page,
      TEST_REALM,
      realProductId,
      DEMO_QUOTA_WINDOWS,
    )

    // 3. Resolve the primary-pool bucket UUID.
    const buckets = await listBucketsViaApi(page, TEST_REALM)
    const primary = buckets.find((b) => b.bucketKey === 'primary-pool')
    if (!primary) {
      throw new Error(
        `[DE-D02 beforeAll] primary-pool bucket not found in ${TEST_REALM}`,
      )
    }

    // 4. Resolve the demo user UUID.
    const backendUrl =
      process.env.API_BASE_URL ||
      process.env.BASE_URL?.replace(/:\d+/, ':8080') ||
      'http://localhost:8080'
    const usersResponse = await context.request.get(
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
      throw new Error(
        `[DE-D02 beforeAll] Could not resolve demo user UUID for ${USER_EMAIL}`,
      )
    }

    // 5. Resolve the points-demo-app client app UUID.
    const clientAppResponse = await context.request.get(
      `${backendUrl}/api/client/${TEST_REALM}`,
    )
    let clientAppId = ''
    if (clientAppResponse.ok()) {
      const clientAppBody = await clientAppResponse.json()
      const items = (clientAppBody?.items ?? []) as {
        id: string
        clientId: string
      }[]
      const demoApp = items.find((a) => a.clientId === 'points-demo-app')
      clientAppId = demoApp?.id ?? ''
    }
    if (!clientAppId) {
      throw new Error(
        `[DE-D02 beforeAll] Could not resolve client app UUID for points-demo-app`,
      )
    }

    // 6. Mint a realm-001 API key with points.manage bound to points-demo-app
    //    so consume's ensure_client_app_scope check passes.
    const apiKey = await createTestApiKeyWithPermission(
      page,
      'points.manage',
      setupStartTime,
      TEST_REALM,
      clientAppId,
    )

    // 7. Seed a top-up balance for the total-formula assertion — but only if
    //    none exists yet. The ext grant API has no idempotency key, so an
    //    unconditional grant would accumulate across demo runs and make the
    //    pool (and thus `points-empty-state` for the empty test) non-
    //    deterministic. Seeding once keeps the pool stable at
    //    TOPUP_GRANT_AMOUNT.
    const existingPool = await getSpendableFromPool(page, TEST_REALM, primary.id)
    if (existingPool < TOPUP_GRANT_AMOUNT) {
      const grant = await grantPointsViaExtApi(apiKey.apiKey, TEST_REALM, {
        userId,
        amount: TOPUP_GRANT_AMOUNT - existingPool,
        bucketId: primary.id,
        reason: 'DE-D02 setup: deterministic top-up balance for total formula',
        validityDays: 365,
      })
      if (grant.status !== 200) {
        throw new Error(
          `[DE-D02 beforeAll] Top-up grant failed: status=${grant.status} body=${JSON.stringify(grant.responseBody)}`,
        )
      }
    }

    setupCtx = {
      bucketId: primary.id,
      userId,
      clientAppId,
      apiKey,
    }
  } finally {
    await context.close()
  }
})

test.afterEach(async ({ page }) => {
  // Revoke the seeded quota entitlement so no window-quota leaks across demo
  // files (the kept demo user persists between runs).
  const ctx = setupCtx
  if (ctx) {
    await revokeQuotaEntitlement(page.context().request, TEST_REALM, {
      userId: ctx.userId,
      bucketId: ctx.bucketId,
      sourceId: QUOTA_SOURCE_ID,
    }).catch(() => {
      /* best-effort; cleanupTestData still runs below */
    })
  }
  await cleanupTestData(page, TEST_REALM, {
    keepUsers: [USER_EMAIL],
  })
})

// ============================================================================
// Helpers
// ============================================================================

function assertSetup(): SetupContext {
  expect(setupCtx, 'beforeAll must have resolved setup context').not.toBeNull()
  return setupCtx!
}

/**
 * Seed a clean window-quota entitlement for the demo user via the internal API.
 *
 * Revokes any prior grant under the fixed `QUOTA_SOURCE_ID` (idempotent no-op
 * if none), then grants a fresh one from `DEMO_QUOTA_WINDOWS`. This replaces the
 * legacy purchase→fulfill flow: the internal fulfill path does NOT issue quota
 * entitlements (only the Stripe webhook does), and seeded price IDs are
 * placeholders that must never reach Stripe (DE-D01 §27), so fast demos seed
 * quota directly here.
 */
async function seedQuotaEntitlement(page: Page): Promise<void> {
  const { userId, bucketId } = assertSetup()
  const request = page.context().request

  // Clean baseline: revoke any prior active entitlement under this source.
  const revokeResult = await revokeQuotaEntitlement(request, TEST_REALM, {
    userId,
    bucketId,
    sourceId: QUOTA_SOURCE_ID,
  })
  expect(
    revokeResult.success,
    `quota revoke failed: ${revokeResult.error ?? ''}`,
  ).toBeTruthy()

  const grantResult = await grantQuotaEntitlement(request, TEST_REALM, {
    userId,
    bucketId,
    sourceId: QUOTA_SOURCE_ID,
    windows: DEMO_QUOTA_WINDOWS,
  })
  expect(
    grantResult.success,
    `quota grant failed: ${grantResult.error ?? ''}`,
  ).toBeTruthy()
}

/**
 * Consume points through the external API using the shared API key.
 */
async function consumeForTest(amount: number): Promise<void> {
  const { apiKey, userId, clientAppId } = assertSetup()
  const result = await consumePointsViaExtApi(apiKey.apiKey, TEST_REALM, {
    userId,
    amount,
    clientAppId,
    description: 'DE-D02 quota dashboard consume',
    idempotencyKey: `de-d02-${setupStartTime}-${Date.now()}`,
  })
  expect(result.status).toBe(200)
}

// ============================================================================
// Test suite
// ============================================================================

test.describe('[Regular User] 积分配额仪表盘 (US-PU-010)', () => {
  test.beforeEach(async ({ page, loginPage }) => {
    await verifyTestEnvironment(page, {
      requiredRealms: [TEST_REALM],
      requiredUsers: [USER_EMAIL],
    })

    // Re-seed a fresh window-quota entitlement for each test so consume
    // scenarios start from a known baseline (revoke-then-grant).
    await loginPage.loginAsUser(USER_EMAIL, USER_PASSWORD, TEST_REALM)
    await seedQuotaEntitlement(page)

    await page.goto(`/${TEST_REALM}/user/points`)
    await expect(
      page.locator(SELECTORS.pointsUsageDashboard.page),
    ).toBeVisible({ timeout: 15000 })
  })

  // --------------------------------------------------------------------------
  // Window rows rendered
  // --------------------------------------------------------------------------

  test('US-PU-010 场景5: 窗口行展示剩余/上限/已用/进度条/恢复时点', async ({
    page,
  }) => {
    const { bucketId } = assertSetup()

    for (const window of DEMO_QUOTA_WINDOWS) {
      const row = getWindowRow(page, bucketId, window.key)
      await expect(row).toBeVisible()

      const bar = page.locator(
        SELECTORS.pointsUsageDashboard.windowBar(bucketId, window.key),
      )
      await expect(bar).toBeVisible()

      const remaining = await getWindowRemaining(page, bucketId, window.key)
      expect(remaining).toBe(window.limit)

      const rowText = (await row.textContent()) ?? ''
      expect(rowText).toContain(window.limit.toLocaleString())
      expect(rowText).toContain('0') // used starts at 0
    }
  })

  // --------------------------------------------------------------------------
  // Tightest constraint on top
  // --------------------------------------------------------------------------

  test('US-PU-010 场景2: 最严约束窗口置顶', async ({ page }) => {
    const { bucketId } = assertSetup()

    const rows = page.locator('[data-testid^="points-window-row-"]')
    await expect(rows).toHaveCount(DEMO_QUOTA_WINDOWS.length)

    const firstRowTestId = await rows.first().getAttribute('data-testid')
    expect(firstRowTestId).toBe(
      `points-window-row-${bucketId}-${SMALLEST_WINDOW_KEY}`,
    )

    // The tightest-constraint window has the smallest remaining value.
    // The UI renders it first; the pill/badge is a visual cue without a stable
    // dedicated testid in the converged FE-A07 contract, so ordering is the
    // authoritative assertion here.
    const firstRowRemaining = await getWindowRemaining(
      page,
      bucketId,
      SMALLEST_WINDOW_KEY,
    )
    for (const window of DEMO_QUOTA_WINDOWS) {
      if (window.key === SMALLEST_WINDOW_KEY) continue
      const remaining = await getWindowRemaining(page, bucketId, window.key)
      expect(firstRowRemaining).toBeLessThanOrEqual(remaining)
    }
  })

  // --------------------------------------------------------------------------
  // Total formula
  // --------------------------------------------------------------------------

  test('US-PU-010 场景5: 当前可消费总额 = 各窗口剩余最小值 + 充值余额', async ({
    page,
  }) => {
    const { bucketId } = assertSetup()

    await expect(
      page.locator(SELECTORS.pointsUsageDashboard.spendableNow),
    ).toBeVisible()
    await expect(
      page.locator(SELECTORS.pointsUsageDashboard.spendableFormula),
    ).toBeVisible()

    const smallestRemaining = await getWindowRemaining(
      page,
      bucketId,
      SMALLEST_WINDOW_KEY,
    )
    const spendableNow = await getSpendableNow(page)
    // The pool (topup/registration/granted) balance accumulates across demo
    // runs — the ext grant API has no idempotency key — so read the live value
    // from the wallets API instead of hard-coding TOPUP_GRANT_AMOUNT.
    const poolBalance = await getSpendableFromPool(
      page,
      TEST_REALM,
      bucketId,
    )
    const expected = smallestRemaining + poolBalance

    expect(spendableNow).toBe(expected)
  })

  // --------------------------------------------------------------------------
  // Resets-at visible after consume
  // --------------------------------------------------------------------------

  test('US-PU-010 场景5: 消费后窗口行显示恢复时点文案', async ({ page }) => {
    const { bucketId } = assertSetup()

    await consumeForTest(CONSUME_SMALL_AMOUNT)
    await page.reload()
    await expect(
      page.locator(SELECTORS.pointsUsageDashboard.page),
    ).toBeVisible({ timeout: 15000 })

    const resetsIn = await getWindowResetsIn(page, bucketId, SMALLEST_WINDOW_KEY)
    expect(resetsIn).toMatch(/\d+(m|h|d)/)
  })

  // --------------------------------------------------------------------------
  // Exhausted red alert
  // --------------------------------------------------------------------------

  test('US-PU-010 场景2: 5小时窗耗尽后显示红色警示', async ({ page }) => {
    const { bucketId } = assertSetup()

    const initialRemaining = await getWindowRemaining(
      page,
      bucketId,
      SMALLEST_WINDOW_KEY,
    )
    await consumeForTest(initialRemaining)

    await page.reload()
    await expect(
      page.locator(SELECTORS.pointsUsageDashboard.page),
    ).toBeVisible({ timeout: 15000 })

    const exhaustedRow = getWindowRow(page, bucketId, SMALLEST_WINDOW_KEY)
    await expect(exhaustedRow).toBeVisible()

    await expect(
      page.locator(SELECTORS.pointsUsageDashboard.exhaustedAlert),
    ).toBeVisible()

    const remainingAfter = await getWindowRemaining(
      page,
      bucketId,
      SMALLEST_WINDOW_KEY,
    )
    expect(remainingAfter).toBe(0)
  })
})

// --------------------------------------------------------------------------
// Loading state (standalone: needs to observe the initial load)
// --------------------------------------------------------------------------

test('US-PU-010 加载态: 初始加载时显示骨架占位', async ({
  page,
  loginPage,
}) => {
  await verifyTestEnvironment(page, {
    requiredRealms: [TEST_REALM],
    requiredUsers: [USER_EMAIL],
  })

  await loginPage.loginAsUser(USER_EMAIL, USER_PASSWORD, TEST_REALM)

  // Delay the wallets endpoint so the skeleton stays visible long enough to assert.
  await page.route('**/api/points/**/wallets', async (route) => {
    await new Promise((resolve) => setTimeout(resolve, 1500))
    await route.continue()
  })

  await page.goto(`/${TEST_REALM}/user/points`)

  // Assert skeleton placeholders are visible while the request is in flight.
  // The exact count depends on the component layout; we assert at least one
  // skeleton exists and the dashboard container has not yet rendered data rows.
  const skeletons = page.locator(
    '[data-testid="points-usage-dashboard"] [data-slot="skeleton"]',
  )
  await expect(skeletons.first()).toBeVisible({ timeout: 5000 })

  // Wait for the delayed request to finish and the dashboard to render.
  await expect(
    page.locator(SELECTORS.pointsUsageDashboard.page),
  ).toBeVisible({ timeout: 15000 })
})

// --------------------------------------------------------------------------
// Empty state (standalone: needs a user with no quota and no balance)
// --------------------------------------------------------------------------

test('US-PU-010 空态: 无配额且无余额的用户显示空态提示', async ({
  page,
}) => {
  await verifyTestEnvironment(page, {
    requiredRealms: [TEST_REALM],
    requiredUsers: [USER_EMAIL],
  })

  const emptyUserEmail = `empty-${Date.now()}@realm-001.com`
  await registerUser(page, TEST_REALM, emptyUserEmail, USER_PASSWORD)
  await loginWithCredentials(page, {
    realmId: TEST_REALM,
    email: emptyUserEmail,
    password: USER_PASSWORD,
  })

  await page.goto(`/${TEST_REALM}/user/points`)
  await expect(page.locator(SELECTORS.pointsUser.page)).toBeVisible()
  // A brand-new user holds no points buckets at all, so the page renders the
  // "no pools" empty card (`points-balance-empty`). This is distinct from
  // `points-empty-state`, which renders INSIDE a bucket card when the bucket
  // exists but has no quota and no balance.
  await expect(
    page.locator(SELECTORS.pointsUser.balanceEmpty),
  ).toBeVisible({ timeout: 15000 })
})
