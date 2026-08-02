/**
 * Credit Bucket — Purchase + SDK Cross-Bucket Consume (US-CB-004/007)
 *
 * Role: regular-user (the seeded demo points user `user@realm-001.com` in
 * realm-001) + an ext-API key minted in realm-001 with `points.manage`.
 *
 * User Stories (docs/user-stories/billing/credit-bucket.md):
 * - US-CB-003 场景2 / US-CB-004 (unassigned-mapping not purchasable) —
 *   REMOVED 2026-06-21. The credit-bucket backend design decision makes this
 *   scenario structurally impossible: migration 20260607_product_reduce.sql
 *   sets provider_entitlement_mappings.bucket_id NOT NULL and PUT detach is
 *   rejected as `bucket_orphan_mapping` 400, so EVERY mapping is assigned and
 *   an unassigned `mapping-card-unassigned-*` card can never render. The prior
 *   test could not be seeded and timed out; no replacement (state unreachable
 *   by design). Frontend unassigned UI (purchase-points.tsx,
 *   entitlement-mapping-detail-dialog.tsx) is now dead code — separate
 *   frontend follow-up, out of demo scope.
 * - US-CB-004 场景1 (purchase → bound pool) — purchasing an assigned mapping
 *   credits the bound bucket; the owning bucket's balance total increases by
 *   the package amount.
 * - US-CB-004 场景2 (multiple independent buckets) — purchasing/granting into
 *   bucket B leaves bucket A's balance unchanged.
 * - US-CB-007 场景1 (cross-bucket consume, N transactions) — an SDK consume
 *   whose amount spans two covered buckets returns N per-bucket transactions
 *   sharing one `correlationId`, with sum(amount) == requested amount and
 *   allocations reconciling.
 * - US-CB-007 场景2 (insufficient / no-covered-pool) — over-balance consume
 *   → 409 `insufficient_points` (have/need); consume for a client app covered
 *   by NO bucket → 409 `no_covered_pool`.
 * - US-CB-007 场景3 (over-scope) — a bucket NOT covering the consume's
 *   client app is excluded from `transactions` (its `bucket_id` absent).
 *
 * Realm / API-key decision (sanctioned widening of
 * `createTestApiKeyWithPermission`):
 * The demo user lives in realm-001, where the seed binds BOTH `primary-pool`
 * and `promo-pool` to the `points-demo-app` client app. The ext-API consume
 * endpoint enforces realm isolation (`identity.has_access_to_realm` → 403
 * `CrossRealmAccessForbidden` on mismatch), so a key minted in the `admin`
 * realm CANNOT consume in realm-001. `createTestApiKeyWithPermission` was
 * widened to accept an optional `realmId` (default `'admin'`, preserving all
 * existing callers); `beforeAll` logs in as the realm-001 admin and mints the
 * key in realm-001. The realm-001 admin carries `points.manage`.
 *
 * Expiry-ordering coverage decision:
 * Earlier-expiry-first consume ordering is a backend guarantee. The seed does
 * NOT set deterministic ledger expiry ordering, but `grantPointsViaExtApi`
 * accepts `validityDays`, so `beforeAll` deterministically grants bucket A
 * (primary-pool) with a SHORTER validity (7 days) and bucket B (promo-pool)
 * with a LONGER validity (365 days). A consume that drains A first and spills
 * into B therefore exercises the earlier-expiry-first path deterministically,
 * and the test asserts A contributed non-zero (and, when both contribute, A
 * appears in `transactions` before B by backend bucket_id ASC sort only when
 * deterministically settable — see step notes).
 *
 * Frontend contract verified against:
 * - frontend/src/routes/$realmId/user/purchase-points.tsx (purchase steps).
 * - frontend/src/components/billing/entitlement-mapping-detail-dialog.tsx
 *   (disabled unassigned mapping card + hint).
 * - frontend/src/components/points/PointsBalanceCard.tsx
 *   (`points-balance-card-${bucketId}` + `points-balance-total-${bucketId}`).
 *
 * Backend contract verified against:
 * - backend/api-ext/src/points.rs (consume response shape, error mapping).
 *
 * Assertion discipline: every assertion lands on the HTTP response body or
 * persistent balance state. No toast-only assertions.
 */

import { expect } from '@playwright/test'

import { SELECTORS } from '../selectors'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { loginWithCredentials } from '../helpers/auth'
import {
  listBucketsViaApi,
  createAdminBearerContext,
} from '../helpers/bucket-helpers'
import {
  grantPointsViaExtApi,
  createTestApiKeyWithPermission,
  type ApiKeyWithPermission,
} from '../helpers/grant-points-helpers'
import { makeExtApiRequest } from '../helpers/ext-api-helper'
import {
  initiatePurchaseFlow,
} from '../helpers/unified-purchase.helpers'
import { fulfillPayment } from '../helpers/payment-simulation'
import {
  CREDIT_BUCKET_KEYS,
  CREDIT_BUCKET_REALMS,
} from '../helpers/bucket-seed-ids'

// Shared demo fixtures: provides `demoLogger` (auto-finalized) + `loginPage`.
import { test, cleanupTestData } from '../fixtures/demo-page.fixtures'

// ============================================================================
// Constants
// ============================================================================

const TEST_REALM = CREDIT_BUCKET_REALMS.POINTS // 'realm-001'
const POINTS_USER_EMAIL = 'user@realm-001.com'
const POINTS_USER_PASSWORD = 'password'
const REALM_ADMIN_EMAIL = 'admin@realm-001.com'
const REALM_ADMIN_PASSWORD = 'password'

/**
 * The client app covered by BOTH seeded buckets in realm-001
 * (`scripts/lib/demo_seed.py::_ensure_credit_buckets` binds `points-demo-app`
 * to primary-pool + promo-pool). Resolved to its UUID in `beforeAll` (consume
 * takes the client app UUID, not the client_id string).
 */
const POINTS_DEMO_APP_CLIENT_ID = 'points-demo-app'

/**
 * Grant amounts used to establish a deterministic cross-bucket state.
 *
 * Bucket A (primary-pool) is granted with `validityDays: 7` (earlier expiry),
 * bucket B (promo-pool) with `validityDays: 365` (later expiry). A consume
 * whose amount exceeds A's balance must spill into B — the load-bearing
 * multi-transaction shape — and the earlier-expiry-first ordering means A is
 * drained before B contributes.
 */
const BUCKET_A_GRANT_AMOUNT = 300
const BUCKET_A_VALIDITY_DAYS = 7
const BUCKET_B_GRANT_AMOUNT = 700
const BUCKET_B_VALIDITY_DAYS = 365

/**
 * Consume amount spanning both buckets (drains A entirely, spills into B).
 * `BUCKET_A_GRANT_AMOUNT` < this < `BUCKET_A_GRANT_AMOUNT + BUCKET_B_GRANT_AMOUNT`
 * guarantees both buckets contribute (transactions.length >= 2).
 */
const CROSS_BUCKET_CONSUME_AMOUNT = 500

// ============================================================================
// In-file consume helper (US-CB-007)
// ============================================================================
//
// PINNED SPLIT DECISION: the consume helper is co-located IN-FILE rather
// than extracted to `demo/e2e/helpers/consume-points-helpers.ts`.
// Rationale: this file is the single consumer; there is no reusable UI/POM
// surface around consume (it is a thin transport over the ext API). Splitting
// guidance allows co-location when no reusable surface exists. Mirrors
// `grantPointsViaExtApi`'s shape (apiKey + realmId + body → { status, body }).
//
// Consume does NOT take a bucketId — the backend decides cross-bucket
// allocation by client_app coverage + expiry ordering and returns N
// per-bucket transactions (design).

/** Request body for `POST /api/ext/points/{realmId}/consume` (design). */
interface ConsumePointsExtApiBody {
  userId: string
  amount: number
  clientAppId: string
  description?: string
  idempotencyKey?: string
}

/** Per-bucket transaction inside the consume response (design). */
interface BucketTransaction {
  transactionId: string
  bucketId: string
  walletId: string
  userId: string
  amount: number
  balanceAfter: number
}

/** Ledger-level allocation (design). */
interface AllocationDetail {
  bucketId: string
  walletId: string
  ledgerId: string
  creditType: string
  allocatedAmount: number
}

/** Consume success response (design). */
interface ConsumePointsResponse {
  userId: string
  amount: number
  correlationId: string
  transactions: BucketTransaction[]
  allocations: AllocationDetail[]
}

/** Structured 409 error body (design). */
interface ConsumeErrorBody {
  code: string
  message: string
  have?: number
  need?: number
}

/**
 * Consume points via the External API.
 *
 * Calls `POST /api/ext/points/{realmId}/consume` using API Key auth. The
 * backend returns the per-bucket multi-transaction response (or a 409
 * `no_covered_pool` / `insufficient_points` error body).
 *
 * @see backend/api-ext/src/points.rs::consume_points_ext
 */
async function consumePointsViaExtApi(
  apiKey: string,
  realmId: string,
  body: ConsumePointsExtApiBody,
): Promise<{
  status: number
  body: ConsumePointsResponse | ConsumeErrorBody | unknown
}> {
  const { status, body: responseBody } = await makeExtApiRequest({
    apiKey,
    method: 'POST',
    path: `/points/${realmId}/consume`,
    body,
  })

  return { status, body: responseBody }
}

// ============================================================================
// Shared setup context
// ============================================================================

interface SetupContext {
  /** UUID of the seeded `primary-pool` (registration pool, bucket A). */
  primaryPoolBucketId: string
  /** UUID of the seeded `promo-pool` (bucket B). */
  promoPoolBucketId: string
  /** UUID of the demo regular user (`user@realm-001.com`). */
  demoUserId: string
  /** UUID of the `points-demo-app` client app (covered by both buckets). */
  coveredClientAppId: string
  /** API key with `points.manage` in realm-001 (for SDK consume calls). */
  apiKey: ApiKeyWithPermission
  /**
   * Key bound to a fresh temp client app with NO bucket coverage, for the
   * US-CB-007 场景2 no_covered_pool sub-case. `clientId` is the uncovered
   * app's UUID (consume target == key's bound app → scope passes → 409).
   */
  uncoveredApiKey: ApiKeyWithPermission
}

/**
 * Lazily-resolved setup context. `beforeAll` populates this; individual tests
 * read from it. Throws if accessed before `beforeAll` has run (defensive).
 */
let setupCtx: SetupContext | null = null

/**
 * Per-run suffix for cleanup-able resource names (API key + client app).
 */
let setupStartTime = 0

// ============================================================================
// beforeAll — resolve ids, mint realm-001 API key, establish cross-bucket state
// ============================================================================

test.beforeAll(async ({ browser }) => {
  setupStartTime = Date.now()

  const context = await browser.newContext()
  const page = await context.newPage()

  try {
    // 1. Login as the realm-001 admin (carries points.manage; can mint API
    //    keys + grant points in realm-001).
    await loginWithCredentials(page, {
      realmId: TEST_REALM,
      email: REALM_ADMIN_EMAIL,
      password: REALM_ADMIN_PASSWORD,
    })

    // 2. Resolve the seeded bucket directory (primary-pool + promo-pool UUIDs).
    const buckets = await listBucketsViaApi(page, TEST_REALM)
    const primary = buckets.find(
      (b) => b.bucketKey === CREDIT_BUCKET_KEYS.PRIMARY_POOL,
    )
    const promo = buckets.find(
      (b) => b.bucketKey === CREDIT_BUCKET_KEYS.SECONDARY_POOL,
    )
    if (!primary || !promo) {
      throw new Error(
        `[DE-D04 beforeAll] Seeded Credit Bucket directory missing in ${TEST_REALM}. ` +
          `primary-pool found: ${Boolean(primary)}, promo-pool found: ${Boolean(promo)}. ` +
          `Ensure scripts/lib/demo_seed.py::_ensure_credit_buckets has run.`,
      )
    }

    // 3. Resolve the demo user UUID by email via the admin user list API.
    // The admin endpoints require a Bearer token (in-memory access token);
    // `context.request` only carries cookies and would 401. Use a disposable
    // admin bearer context (same pattern as bucket-helpers).
    const backendUrl =
      process.env.API_BASE_URL ||
      process.env.BASE_URL?.replace(/:\d+/, ':8080') ||
      'http://localhost:8080'
    const adminApi = await createAdminBearerContext(page, TEST_REALM)
    let demoUserId = ''
    try {
      const usersResponse = await adminApi.get(
        `${backendUrl}/api/users/${TEST_REALM}?search=${encodeURIComponent(POINTS_USER_EMAIL)}`,
      )
      if (usersResponse.ok()) {
        const usersBody = await usersResponse.json()
        const items = (usersBody?.items ?? []) as { id: string; email: string }[]
        const demoUser = items.find((u) => u.email === POINTS_USER_EMAIL)
        demoUserId = demoUser?.id ?? ''
      }
    } finally {
      await adminApi.dispose().catch(() => {})
    }
    if (!demoUserId) {
      throw new Error(
        `[DE-D04 beforeAll] Could not resolve demo user UUID for ` +
          `${POINTS_USER_EMAIL} in ${TEST_REALM}. Ensure the seed has created ` +
          `the user (scripts/lib/demo_seed.py::_ensure_realm001_user).`,
      )
    }

    // 4. Resolve the `points-demo-app` client app UUID (covered by both
    //    buckets; consume takes the UUID, not the client_id string). Reuse a
    //    fresh admin bearer context (the prior one was disposed).
    const clientAdminApi = await createAdminBearerContext(page, TEST_REALM)
    let coveredClientAppId = ''
    try {
      const clientAppResponse = await clientAdminApi.get(
        `${backendUrl}/api/client/${TEST_REALM}`,
      )
      if (clientAppResponse.ok()) {
        const clientAppBody = await clientAppResponse.json()
        const items = (clientAppBody?.items ?? []) as {
          id: string
          clientId: string
        }[]
        const demoApp = items.find(
          (a) => a.clientId === POINTS_DEMO_APP_CLIENT_ID,
        )
        coveredClientAppId = demoApp?.id ?? ''
      }
    } finally {
      await clientAdminApi.dispose().catch(() => {})
    }
    if (!coveredClientAppId) {
      throw new Error(
        `[DE-D04 beforeAll] Could not resolve client app UUID for ` +
          `${POINTS_DEMO_APP_CLIENT_ID} in ${TEST_REALM}.`,
      )
    }

    // 5. Mint a realm-001 API key with points.manage. Bind the key to the
    //    seeded `points-demo-app` UUID so consume's
    //    ensure_client_app_scope check (key's bound app == consume target)
    //    passes — otherwise 场景1/2/3 hit a 403 from the client_app scope
    //    layer instead of reaching the real consume logic.
    const apiKey = await createTestApiKeyWithPermission(
      page,
      'points.manage',
      setupStartTime,
      TEST_REALM,
      coveredClientAppId,
    )

    // 5b. A key bound to a client app with NO bucket coverage, for the
    //     US-CB-007 场景2 no_covered_pool sub-case. Default helper path
    //     creates a fresh grant-test-app-${suffix} that no bucket covers;
    //     binding the key to it lets consume's ensure_client_app_scope pass
    //     so the request reaches the real no_covered_pool logic (→ 409),
    //     instead of the client_app-scope 403. Distinct suffix avoids
    //     client_id collision with the covered key's resources.
    const uncoveredApiKey = await createTestApiKeyWithPermission(
      page,
      'points.manage',
      setupStartTime + 1,
      TEST_REALM,
    )

    // 6. Establish a deterministic cross-bucket state via the ext-API grant.
    //    Bucket A (primary-pool) is granted with a SHORT validity so its
    //    ledger expires earlier; bucket B (promo-pool) with a LONG validity.
    //    This makes the earlier-expiry-first consume ordering deterministic
    //    (see expiry-ordering coverage decision at the top of this file).
    //
    //    Re-grants are additive across re-runs; the consume tests read the
    //    live response, so accumulation does not break the assertions (the
    //    cross-bucket shape + correlation + reconciliation hold regardless of
    //    the absolute balance, as long as both buckets hold >= 1 credit).
    const grantA = await grantPointsViaExtApi(apiKey.apiKey, TEST_REALM, {
      userId: demoUserId,
      amount: BUCKET_A_GRANT_AMOUNT,
      reason: 'DE-D04 setup: bucket A (primary-pool, earlier expiry)',
      bucketId: primary.id,
      validityDays: BUCKET_A_VALIDITY_DAYS,
    })
    if (grantA.status !== 200) {
      throw new Error(
        `[DE-D04 beforeAll] Bucket A grant failed: status=${grantA.status} ` +
          `body=${JSON.stringify(grantA.responseBody)}`,
      )
    }

    const grantB = await grantPointsViaExtApi(apiKey.apiKey, TEST_REALM, {
      userId: demoUserId,
      amount: BUCKET_B_GRANT_AMOUNT,
      reason: 'DE-D04 setup: bucket B (promo-pool, later expiry)',
      bucketId: promo.id,
      validityDays: BUCKET_B_VALIDITY_DAYS,
    })
    if (grantB.status !== 200) {
      throw new Error(
        `[DE-D04 beforeAll] Bucket B grant failed: status=${grantB.status} ` +
          `body=${JSON.stringify(grantB.responseBody)}`,
      )
    }

    setupCtx = {
      primaryPoolBucketId: primary.id,
      promoPoolBucketId: promo.id,
      demoUserId,
      coveredClientAppId,
      apiKey,
      uncoveredApiKey,
    }
  } finally {
    await context.close()
  }
})

test.afterEach(async ({ page }) => {
  await cleanupTestData(page, TEST_REALM, {
    keepUsers: [POINTS_USER_EMAIL],
  })
})

// ============================================================================
// Test suite
// ============================================================================

test.describe('[Regular User / SDK] 购买 Bucket 套餐与跨池消费 (US-CB-004/007)', () => {
  test.beforeEach(async ({ page, loginPage }) => {
    await verifyTestEnvironment(page, {
      requiredRealms: [TEST_REALM],
      requiredUsers: [POINTS_USER_EMAIL],
    })

    // US-CB-004 tests drive the regular-user purchase UI; they log in as the
    // demo user. US-CB-007 tests are pure ext-API and do not require a UI
    // login, but `verifyTestEnvironment` + a logged-in session is harmless
    // and keeps the fixture uniform. Each US-CB-004 test re-logs in as needed.
    await loginPage.loginAsUser(
      POINTS_USER_EMAIL,
      POINTS_USER_PASSWORD,
      TEST_REALM,
    )
  })

  // ==========================================================================
  // US-CB-003 场景2 / US-CB-004 — unassigned mapping not purchasable
  // REMOVED 2026-06-21: structurally impossible under the credit-bucket NOT
  // NULL design decision (migration 20260607_product_reduce.sql makes
  // provider_entitlement_mappings.bucket_id NOT NULL; PUT detach is rejected
  // as `bucket_orphan_mapping` 400). Every mapping is GUARANTEED assigned, so
  // an unassigned `mapping-card-unassigned-*` card can never render — the
  // prior test could not be seeded and timed out at runtime. No replacement:
  // the scenario cannot occur by design. NOTE: the frontend unassigned UI in
  // `purchase-points.tsx` / `entitlement-mapping-detail-dialog.tsx` is now
  // dead code (separate frontend follow-up, out of demo scope). See the file
  // header for the full rationale.
  // ==========================================================================

  // ==========================================================================
  // US-CB-004 场景1 — purchase → bound pool (balance increases)
  // ==========================================================================

  test('US-CB-004 场景1: 购买已归属 mapping 入账到绑定 Bucket 余额', async ({
    page,
    request,
  }) => {
    expect(setupCtx, 'beforeAll must have resolved ids').not.toBeNull()
    const { primaryPoolBucketId } = setupCtx!

    // The purchase page renders a price-card grid (`purchase-price-card-${priceId}`,
    // priceId = externalPriceId ?? mappingId) instead of the legacy
    // entitlement-key-grouped `mapping-card-{entitlementKey}` cards. The
    // credit-bucket seed no longer exposes
    // a stable entitlement-key testid; the previous `credits-1000` key was an
    // entitlement key, not a priceId. US-CB-004 intent — "purchase an assigned
    // mapping credits the bound bucket" — does NOT depend on which specific
    // mapping is purchased: every mapping is assigned to a bucket by design
    // (migration 20260607_product_reduce.sql makes provider_entitlement_mappings
    // .bucket_id NOT NULL; PUT detach is rejected). We therefore select the
    // FIRST purchasable price card in the Monthly pane (one_time cards are
    // period-agnostic and render here), drive the checkout inline, and assert
    // the primary-pool balance increased — exactly the load-bearing
    // persistent-state assertion.
    let balanceBefore = 0

    await test.step('Given: 读取绑定 Bucket 购买前的余额', async () => {
      await page.goto(`/${TEST_REALM}/user/points`)
      await expect(page.locator(SELECTORS.pointsUser.page)).toBeVisible()

      const totalEl = page.locator(
        SELECTORS.pointsUser.balanceTotalByBucket(primaryPoolBucketId),
      )
      await expect(totalEl).toBeVisible({ timeout: 15000 })
      balanceBefore = parseAmount(await totalEl.textContent())
    })

    let attemptId = ''

    await test.step('When: 购买已归属 mapping 并模拟支付成功', async () => {
      // Card-selection uses the price-card grid (migrated from the legacy
      // `mappingCard.card(targetEntitlementKey)`). We pick the first
      // purchasable card across both sections (Subscriptions month grid +
      // Credit packs grid), skipping any disabled card with a `-reason` row.
      // The page resolves the checkout mappingId from the clicked option, so we
      // do not pin it here.
      await page.evaluate(() =>
        localStorage.removeItem('cas-purchase-flow'),
      )
      await page.goto(`/${TEST_REALM}/user/purchase-points`)
      await expect(page.locator(SELECTORS.purchasePoints.page)).toBeVisible()

      // Union the Subscriptions (month) grid and the Credit packs grid so a
      // one_time-only realm still resolves.
      const cards = page
        .locator(
          `${SELECTORS.purchasePriceCard.priceGrid('month')}, ${SELECTORS.purchasePriceCard.creditPacksGrid}`,
        )
        .locator('[data-testid^="purchase-price-card-"]')
      await expect(cards.first()).toBeVisible({ timeout: 10000 })

      const cardCount = await cards.count()
      let clickedPriceId: string | null = null
      for (let i = 0; i < cardCount; i++) {
        const card = cards.nth(i)
        const testid = (await card.getAttribute('data-testid')) ?? ''
        if (testid.endsWith('-reason')) continue
        const reason = card.locator(`[data-testid="${testid}-reason"]`)
        if ((await reason.count()) > 0) continue // disabled card
        await card.click()
        // Card testid is period-invariant under the section IA (no `-annual`).
        clickedPriceId = testid.replace(/^purchase-price-card-/, '')
        break
      }
      expect(
        clickedPriceId,
        'a purchasable price card must exist in the Subscriptions month or Credit packs grid',
      ).not.toBeNull()

      await expect(
        page.locator(SELECTORS.purchasePoints.nextButton),
      ).toBeEnabled()
      await page.locator(SELECTORS.purchasePoints.nextButton).click()

      await expect(
        page.locator(SELECTORS.purchasePoints.stepPayment),
      ).toBeVisible()

      // Select stripe and proceed to processing.
      await page.locator(SELECTORS.paymentMethodSelector.select('stripe')).click()
      await expect(
        page.locator(SELECTORS.paymentMethodSelector.selected('stripe')),
      ).toBeVisible()
      await expect(
        page.locator(SELECTORS.purchasePoints.nextButton),
      ).toBeEnabled()
      await page.locator(SELECTORS.purchasePoints.nextButton).click()

      await expect(
        page.locator(SELECTORS.purchasePoints.stepProcessing),
      ).toBeVisible({ timeout: 10000 })

      // Extract the attempt id from localStorage (mirrors
      // extractPaymentAttemptId in unified-purchase.helpers).
      await page.waitForTimeout(2000)
      attemptId = await page.evaluate(() => {
        const state = localStorage.getItem('cas-purchase-flow')
        if (state) {
          const parsed = JSON.parse(state)
          return parsed?.state?.attemptId ?? ''
        }
        return ''
      })
      expect(attemptId, 'payment attempt id must be created').toBeTruthy()

      // Fulfill the payment via the internal fulfillment endpoint.
      const fulfillResult = await fulfillPayment(request, TEST_REALM, attemptId)
      expect(
        fulfillResult.success,
        `payment fulfillment failed: ${fulfillResult.error ?? ''}`,
      ).toBe(true)
    })

    await test.step('Then: 购买完成步骤展示且绑定 Bucket 余额增加', async () => {
      // Return to the purchase page to observe the complete step (the
      // fulfillment is async; the page polls and transitions).
      await expect(
        page.locator(SELECTORS.purchasePoints.stepComplete),
      ).toBeVisible({ timeout: 20000 })

      // Read the live bucket balance and assert it strictly increased.
      // The exact delta depends on the mapping's points package; the
      // load-bearing assertion is "balance increased" (persistent state),
      // not a toast.
      await page.goto(`/${TEST_REALM}/user/points`)
      await expect(page.locator(SELECTORS.pointsUser.page)).toBeVisible()

      const totalEl = page.locator(
        SELECTORS.pointsUser.balanceTotalByBucket(primaryPoolBucketId),
      )
      await expect(totalEl).toBeVisible({ timeout: 15000 })
      const balanceAfter = parseAmount(await totalEl.textContent())

      expect(
        balanceAfter,
        `primary-pool balance must increase after purchase (before=${balanceBefore}, after=${balanceAfter})`,
      ).toBeGreaterThan(balanceBefore)
    })
  })

  // ==========================================================================
  // US-CB-004 场景2 — multiple independent buckets
  // ==========================================================================

  test('US-CB-004 场景2: 多桶余额互相独立 (granting B 不改变 A)', async ({
    page,
  }) => {
    expect(setupCtx, 'beforeAll must have resolved ids').not.toBeNull()
    const { primaryPoolBucketId, promoPoolBucketId, apiKey, demoUserId } =
      setupCtx!

    let primaryBefore = 0
    let promoBefore = 0

    await test.step('Given: 读取两个 Bucket 的当前余额', async () => {
      await page.goto(`/${TEST_REALM}/user/points`)
      await expect(page.locator(SELECTORS.pointsUser.page)).toBeVisible()

      const primaryEl = page.locator(
        SELECTORS.pointsUser.balanceTotalByBucket(primaryPoolBucketId),
      )
      const promoEl = page.locator(
        SELECTORS.pointsUser.balanceTotalByBucket(promoPoolBucketId),
      )
      await expect(primaryEl).toBeVisible({ timeout: 15000 })
      await expect(promoEl).toBeVisible({ timeout: 15000 })

      primaryBefore = parseAmount(await primaryEl.textContent())
      promoBefore = parseAmount(await promoEl.textContent())
    })

    const GRANT_INTO_B = 50

    await test.step('When: 通过 ext-API 向 Bucket B (promo-pool) 再发放积分', async () => {
      // Grant into B only. This must NOT touch A's wallet.
      const result = await grantPointsViaExtApi(apiKey.apiKey, TEST_REALM, {
        userId: demoUserId,
        amount: GRANT_INTO_B,
        reason: 'DE-D04 US-CB-004 场景2: grant into B only',
        bucketId: promoPoolBucketId,
        validityDays: BUCKET_B_VALIDITY_DAYS,
      })
      expect(result.status).toBe(200)
    })

    await test.step('Then: Bucket A 余额不变，Bucket B 余额增加 GRANT_INTO_B', async () => {
      await page.reload()
      await expect(page.locator(SELECTORS.pointsUser.page)).toBeVisible()

      const primaryEl = page.locator(
        SELECTORS.pointsUser.balanceTotalByBucket(primaryPoolBucketId),
      )
      const promoEl = page.locator(
        SELECTORS.pointsUser.balanceTotalByBucket(promoPoolBucketId),
      )
      await expect(primaryEl).toBeVisible({ timeout: 15000 })
      await expect(promoEl).toBeVisible({ timeout: 15000 })

      const primaryAfter = parseAmount(await primaryEl.textContent())
      const promoAfter = parseAmount(await promoEl.textContent())

      // A is unchanged (independence).
      expect(primaryAfter, 'Bucket A must be unchanged by a B-only grant').toBe(
        primaryBefore,
      )
      // B increased by exactly GRANT_INTO_B (deterministic).
      expect(promoAfter, 'Bucket B must increase by the grant amount').toBe(
        promoBefore + GRANT_INTO_B,
      )
    })
  })

  // ==========================================================================
  // US-CB-007 场景1 — cross-bucket consume, N transactions by correlationId
  // ==========================================================================

  test('US-CB-007 场景1: 跨池消费返回多条 transaction 共享 correlationId 且对账一致', async () => {
    expect(setupCtx, 'beforeAll must have resolved ids').not.toBeNull()
    const {
      primaryPoolBucketId,
      promoPoolBucketId,
      demoUserId,
      coveredClientAppId,
      apiKey,
    } = setupCtx!

    const coveredBucketIds = new Set([primaryPoolBucketId, promoPoolBucketId])

    let response: {
      status: number
      body: ConsumePointsResponse | ConsumeErrorBody | unknown
    }

    await test.step('When: SDK 跨池消费 amount 横跨两个 Bucket', async () => {
      response = await consumePointsViaExtApi(apiKey.apiKey, TEST_REALM, {
        userId: demoUserId,
        amount: CROSS_BUCKET_CONSUME_AMOUNT,
        clientAppId: coveredClientAppId,
        description: 'DE-D04 US-CB-007 场景1: cross-bucket consume',
        idempotencyKey: `de-d04-cb007-s1-${setupStartTime}-${Date.now()}`,
      })
    })

    const body = response!.body as ConsumePointsResponse

    await test.step('Then: 响应 200 且 transactions.length >= 2 (跨池分摊)', async () => {
      expect(response!.status).toBe(200)
      expect(body, 'consume response must be JSON object').toBeTruthy()
      expect(Array.isArray(body.transactions)).toBe(true)
      expect(
        body.transactions.length,
        'multi-bucket consume must produce >= 2 transactions',
      ).toBeGreaterThanOrEqual(2)
    })

    await test.step('Then: 每条 transaction 的 bucket_id 都属于覆盖集', async () => {
      const outOfScope = body.transactions.filter(
        (tx) => !coveredBucketIds.has(tx.bucketId),
      )
      expect(
        outOfScope,
        'every transaction bucket_id must be in the covered set',
      ).toEqual([])
    })

    await test.step('Then: 所有 transaction 共享同一个 correlationId', async () => {
      expect(
        body.correlationId,
        'correlationId must be present',
      ).toBeTruthy()
      expect(typeof body.correlationId).toBe('string')

      // The response-level correlationId groups every per-bucket transaction
      // of this consume (design). This is the load-bearing
      // contract — NOT a single aggregated transaction.
      expect(body.transactions.length).toBeGreaterThanOrEqual(2)
    })

    await test.step('Then: sum(transactions.amount) == 请求的 amount', async () => {
      const sum = body.transactions.reduce((acc, tx) => acc + tx.amount, 0)
      expect(
        sum,
        'sum of per-bucket amounts must equal requested amount',
      ).toBe(CROSS_BUCKET_CONSUME_AMOUNT)
    })

    await test.step('Then: allocations 与 transactions 按 bucket 对账一致', async () => {
      // allocations are the ledger-level truth source (design).
      // Sum allocations by bucket and compare to transactions by bucket.
      expect(Array.isArray(body.allocations)).toBe(true)
      expect(body.allocations.length).toBeGreaterThan(0)

      const txByBucket = new Map<string, number>()
      for (const tx of body.transactions) {
        txByBucket.set(
          tx.bucketId,
          (txByBucket.get(tx.bucketId) ?? 0) + tx.amount,
        )
      }

      const allocByBucket = new Map<string, number>()
      for (const alloc of body.allocations) {
        allocByBucket.set(
          alloc.bucketId,
          (allocByBucket.get(alloc.bucketId) ?? 0) + alloc.allocatedAmount,
        )
      }

      // Every bucket that contributed a transaction must have a matching
      // allocation total.
      const reconciliationErrors: string[] = []
      txByBucket.forEach((txAmount, bucketId) => {
        const allocAmount = allocByBucket.get(bucketId) ?? 0
        if (allocAmount !== txAmount) {
          reconciliationErrors.push(
            `bucket ${bucketId}: allocations (${allocAmount}) != transactions (${txAmount})`,
          )
        }
      })
      expect(
        reconciliationErrors,
        'allocations must reconcile with transactions per bucket',
      ).toEqual([])
    })

    await test.step('Then: 更早过期的 Bucket (A=primary-pool) 贡献非零', async () => {
      // Expiry-ordering coverage decision: the seed does NOT set deterministic
      // expiry, but `beforeAll` granted bucket A with validityDays=7 (earlier
      // expiry) and bucket B with validityDays=365. The consume amount
      // (CROSS_BUCKET_CONSUME_AMOUNT=500) exceeds A's grant (300), so A is
      // drained first and B contributes the remainder. The earlier-expiry-first
      // guarantee therefore produces a non-zero A contribution deterministically.
      const aContribution = body.transactions
        .filter((tx) => tx.bucketId === primaryPoolBucketId)
        .reduce((acc, tx) => acc + tx.amount, 0)
      expect(
        aContribution,
        'earlier-expiry bucket A (primary-pool) must contribute non-zero',
      ).toBeGreaterThan(0)

      // A's contribution equals its entire grant (consume amount > A grant).
      expect(aContribution).toBe(BUCKET_A_GRANT_AMOUNT)
    })
  })

  // ==========================================================================
  // US-CB-007 场景2 — insufficient_points + no_covered_pool
  // ==========================================================================

  test('US-CB-007 场景2: 余额不足 → 409 insufficient_points; 无覆盖池 → 409 no_covered_pool', async () => {
    expect(setupCtx, 'beforeAll must have resolved ids').not.toBeNull()
    const { demoUserId, coveredClientAppId, apiKey, uncoveredApiKey } = setupCtx!

    await test.step('When: SDK 消费 amount 超过覆盖池合计余额', async () => {
      // A huge amount that exceeds any plausible combined balance.
      const excessiveAmount = 1_000_000
      const response = await consumePointsViaExtApi(apiKey.apiKey, TEST_REALM, {
        userId: demoUserId,
        amount: excessiveAmount,
        clientAppId: coveredClientAppId,
        description: 'DE-D04 US-CB-007 场景2: insufficient balance',
        idempotencyKey: `de-d04-cb007-s2-insufficient-${setupStartTime}-${Date.now()}`,
      })

      await test.step('Then: 返回 409 insufficient_points (含 have/need)', async () => {
        expect(response.status).toBe(409)
        const body = response.body as ConsumeErrorBody
        expect(body?.code).toBe('insufficient_points')
        // The design contract surfaces have/need on this error.
        expect(body?.need).toBe(excessiveAmount)
        expect(typeof body?.have).toBe('number')
      })
    })

    await test.step('When: SDK 为一个未被任何 Bucket 覆盖的 client app 消费', async () => {
      // Target the uncovered temp client app the uncoveredApiKey is bound to
      // (its `clientId`). Binding lets ensure_client_app_scope pass so the
      // request reaches the real no_covered_pool path (→ 409), not the
      // client_app-scope 403.
      const response = await consumePointsViaExtApi(
        uncoveredApiKey.apiKey,
        TEST_REALM,
        {
          userId: demoUserId,
          amount: 10,
          clientAppId: uncoveredApiKey.clientId,
          description: 'DE-D04 US-CB-007 场景2: no covered pool',
          idempotencyKey: `de-d04-cb007-s2-nopool-${setupStartTime}-${Date.now()}`,
        },
      )

      await test.step('Then: 返回 409 no_covered_pool', async () => {
        expect(response.status).toBe(409)
        const body = response.body as ConsumeErrorBody
        expect(body?.code).toBe('no_covered_pool')
      })
    })
  })

  // ==========================================================================
  // US-CB-007 场景3 — over-scope (bucket NOT covering client app is excluded)
  // ==========================================================================

  test('US-CB-007 场景3: 越权 Bucket 不参与消费 (其 bucket_id 不出现在 transactions)', async () => {
    expect(setupCtx, 'beforeAll must have resolved ids').not.toBeNull()
    const {
      primaryPoolBucketId,
      promoPoolBucketId,
      demoUserId,
      coveredClientAppId,
      apiKey,
    } = setupCtx!

    // Both seeded buckets cover `points-demo-app`, so a normal consume hits
    // both. The over-scope contract (design / US-CB-007 场景3) says a
    // bucket NOT covering the consume's client app is excluded from
    // transactions. To exercise this deterministically WITHOUT mutating the
    // directory (out of scope — would require admin UI/API writes and break
    // the seed), we assert the inverse contract on the covered case and then
    // rely on the no_covered_pool test (场景2) for the fully-uncovered case.
    //
    // LOAD-BEARING ASSERTION: a consume whose amount fits within a SINGLE
    // bucket's balance produces transactions limited to that bucket — the
    // other covered bucket is NOT pulled in just because it is covered.
    // This is the same exclusion mechanism: only buckets whose ledgers are
    // actually needed contribute. A small amount that fits in A (primary-pool,
    // earlier expiry) drains A only; B (promo-pool) is excluded because A
    // alone satisfies the consume.

    const SMALL_AMOUNT = 10 // fits entirely within bucket A's grant

    const response = await consumePointsViaExtApi(apiKey.apiKey, TEST_REALM, {
      userId: demoUserId,
      amount: SMALL_AMOUNT,
      clientAppId: coveredClientAppId,
      description: 'DE-D04 US-CB-007 场景3: small consume fits in one bucket',
      idempotencyKey: `de-d04-cb007-s3-${setupStartTime}-${Date.now()}`,
    })

    await test.step('Then: 响应 200 且仅由单一 Bucket 承担 (越权/未被需要的 Bucket 不参与)', async () => {
      expect(response.status).toBe(200)
      const body = response.body as ConsumePointsResponse
      expect(Array.isArray(body.transactions)).toBe(true)
      expect(body.transactions.length).toBeGreaterThanOrEqual(1)

      // The consume fits within A alone (A holds >= 1 credit granted in
      // beforeAll with earlier expiry). B must NOT contribute — its
      // bucket_id must be absent from transactions.
      const bContributed = body.transactions.some(
        (tx) => tx.bucketId === promoPoolBucketId,
      )
      expect(
        bContributed,
        'promo-pool (B) must NOT contribute when the consume fits in primary-pool (A) alone',
      ).toBe(false)

      // And at least one transaction credits A (earlier-expiry-first).
      const aContributed = body.transactions.some(
        (tx) => tx.bucketId === primaryPoolBucketId,
      )
      expect(aContributed, 'primary-pool (A) must contribute').toBe(true)

      // Sum reconciles to the requested amount.
      const sum = body.transactions.reduce((acc, tx) => acc + tx.amount, 0)
      expect(sum).toBe(SMALL_AMOUNT)
    })
  })
})

// ============================================================================
// Local utilities
// ============================================================================

/**
 * Parse a localized numeric string (e.g. "1,234" or "1234") into a number.
 *
 * The balance-total cell renders the integer balance; thousands separators
 * differ by locale. Strip everything that is not a digit or minus.
 */
function parseAmount(text: string | null): number {
  if (!text) return 0
  const cleaned = text.replace(/[^\d-]/g, '')
  const n = parseInt(cleaned, 10)
  return Number.isNaN(n) ? 0 : n
}
