/**
 * US-EM-001 — Provider-filter accuracy + empty-state toolbar visibility E2E
 *
 * Restores two scenarios that lived on the deleted
 * `entitlement-mapping-demo.e2e.ts` and remain relevant to the rewritten
 * master-detail UI:
 *   - #5 Provider-filter accuracy: `provider-filter-select` narrows the
 *     product LIST to the selected provider; 'All' restores the union.
 *   - #6 Empty-state toolbar visibility: when the list is empty, the toolbar
 *     (`provider-filter-select` + `provider-sync-button`) stays visible.
 *
 * Consumed infra (AUTHORITATIVE — do not duplicate surface here):
 * - POM: `demo/e2e/pages/entitlement-mappings-page.ts`
 *     goto / waitForReady / waitForDataLoaded / filterByProvider / isListEmpty
 *     + exposed locator fields (providerFilterSelect, providerSyncButton,
 *       mappingProductList, emptyState).
 * - selectors: `SELECTORS.multiPriceMapping.*`.
 *
 * Frontend contract (verified against
 * `frontend/src/components/billing/entitlement-mappings-page.tsx`):
 * - `provider-filter-select` is a Radix Select whose `onValueChange` updates
 *   `providerFilter` state. `providerFilter` is forwarded to the query as
 *   `paymentProvider` (server-side filter, lines 69-86); selecting a provider
 *   triggers a fresh `useQuery` fetch, not a client-side filter of cached data.
 *   Selecting `all` issues `paymentProvider: undefined` (full list).
 * - The product list renders one `<button data-testid="mapping-product-row-{$id}">`
 *   per product GROUP, carrying a `<Badge variant="secondary">` whose text is
 *   `formatProviderName(g.paymentProvider)` → `Stripe` / `Creem` (capitalized).
 *   This Badge is the per-row provider marker the filter test asserts against.
 * - The empty-state branch is `allMappings.length === 0` (line 237) — i.e. the
 *   server returned zero mappings for the active filter. The toolbar (lines
 *   188-233) renders UNCONDITIONALLY, outside the data-state branch, so it
 *   stays visible in the empty state.
 *
 * Realm choice (load-bearing):
 * - Scenario #5 requires BOTH providers in one realm to prove narrowing + union
 *   restoration. `scripts/lib/demo_seed.py::_ensure_points_package_payment_demo_data`
 *   seeds Stripe AND Creem one-time mappings into `realm-001` (POINTS_REALM_ID).
 *   The admin realm only has Stripe, so #5 MUST run against realm-001.
 *   Login uses `REALM_ADMINS['realm-001']`.
 * - Scenario #6 needs a DETERMINISTIC empty state without mutating the shared
 *   DB. The admin realm has NO Creem mappings → filtering by `creem` yields
 *   zero rows → `emptyState` renders with the toolbar still visible. This
 *   reaches a true empty state via the provider filter (no DB teardown needed).
 *
 * Assertion discipline: every assertion lands on persistent DOM structure —
 * per-row Badge text, list row count, empty-state testid, toolbar testids. No
 * assertion is toast-only.
 */

import { test, cleanupTestData, expect } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import type { UnifiedLogger } from '../helpers/unified-logger'
import { DEMO_ADMIN, REALM_ADMINS } from '../helpers/auth'
import { SELECTORS } from '../selectors'

/**
 * realm-001 hosts BOTH Stripe and Creem one-time mappings (see
 * `_ensure_points_package_payment_demo_data`); required for #5 narrowing.
 */
const POINTS_REALM_ID = 'realm-001'

/**
 * `[Billing Admin] Entitlement 映射 Provider 过滤与空状态工具栏 (US-EM-001)` —
 * the describe title the runner targets via `--grep`.
 */
test.describe('[Billing Admin] Entitlement 映射 Provider 过滤与空状态工具栏 (US-EM-001)', () => {
  let testStartTime: number

  test.beforeEach(async ({ page, demoLogger }) => {
    testStartTime = Date.now()
    await verifyTestEnvironment(page, {
      requiredRealms: [DEMO_ADMIN.realmId, POINTS_REALM_ID],
      requiredUsers: [DEMO_ADMIN.email, REALM_ADMINS[POINTS_REALM_ID].email],
    })
    await demoLogger.testCode.log('Environment verified (admin personas for billing.manage)')
  })

  test.afterEach(async ({ page, demoLogger }) => {
    await cleanupTestData(page, DEMO_ADMIN.realmId, {
      keepUsers: [DEMO_ADMIN.email],
      timestamp: testStartTime,
    })
    await cleanupTestData(page, POINTS_REALM_ID, {
      keepUsers: [REALM_ADMINS[POINTS_REALM_ID].email],
      timestamp: testStartTime,
    })
    await demoLogger.testCode.log('Test data cleaned up')
  })

  // ==========================================================================
  // #5 — Provider-filter accuracy
  // Selecting a provider narrows the product LIST to that provider; selecting
  // 'All' restores the union. Seed (realm-001) has BOTH Stripe and Creem.
  // US-EM-001 (provider-filter narrowing is the admin-side facet of the view
  // story).
  // ==========================================================================
  test('#5 Provider 过滤精确性：Stripe/Creem 各自收敛，All 还原并集', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    const mappingsPage = await setupRealmMappingsPage(
      page,
      loginPage,
      demoLogger,
      POINTS_REALM_ID,
    )

    // Baseline: 'All' must surface rows from BOTH providers. If the seed ever
    // drops one provider this branch fails loudly instead of silently passing.
    let stripeCountAll = 0
    let creemCountAll = 0

    await test.step('Given: 默认 All 视图同时包含 Stripe 与 Creem 行', async () => {
      const rows = mappingsPage.mappingProductList.locator(
        SELECTORS.multiPriceMapping.firstMappingProductRow(),
      )
      await expect(rows.first()).toBeVisible()
      const totalCount = await rows.count()
      expect(totalCount, 'realm-001 seed must surface ≥ 1 product row under All').toBeGreaterThanOrEqual(1)

      // Count rows whose Badge text matches each provider.
      stripeCountAll = await rows.filter({ hasText: 'Stripe' }).count()
      creemCountAll = await rows.filter({ hasText: 'Creem' }).count()
      expect(stripeCountAll, 'realm-001 seed must contain ≥ 1 Stripe product').toBeGreaterThanOrEqual(1)
      expect(creemCountAll, 'realm-001 seed must contain ≥ 1 Creem product').toBeGreaterThanOrEqual(1)
      await demoLogger.testCode.log(
        `All view: ${totalCount} rows (Stripe=${stripeCountAll}, Creem=${creemCountAll})`,
      )
    })

    await test.step('When: 过滤 provider=stripe', async () => {
      await mappingsPage.filterByProvider('stripe')
      await mappingsPage.waitForDataLoaded()
    })

    await test.step('Then: 列表仅保留 Stripe 行（每行 Badge 均为 Stripe）', async () => {
      const rows = mappingsPage.mappingProductList.locator(
        SELECTORS.multiPriceMapping.firstMappingProductRow(),
      )
      await expect(rows.first()).toBeVisible()
      const visibleCount = await rows.count()
      // The server filters by provider, so the visible count must match the
      // Stripe baseline. Persisted structural evidence (per-row Badge text).
      expect(visibleCount, 'stripe filter must surface the seeded Stripe rows').toBe(stripeCountAll)
      const creemVisible = await rows.filter({ hasText: 'Creem' }).count()
      expect(creemVisible, 'stripe filter must NOT surface any Creem row').toBe(0)
      // Every visible row carries the Stripe Badge — load-bearing per-row check.
      const allStripe = await rows.evaluateAll((els) =>
        els.every((el) => (el.textContent ?? '').includes('Stripe')),
      )
      expect(allStripe, 'every visible row Badge must read Stripe').toBe(true)
      await demoLogger.testCode.log(`stripe filter: ${visibleCount} rows, all Stripe`)
    })

    await test.step('When: 过滤 provider=creem', async () => {
      await mappingsPage.filterByProvider('creem')
      await mappingsPage.waitForDataLoaded()
    })

    await test.step('Then: 列表仅保留 Creem 行（每行 Badge 均为 Creem）', async () => {
      const rows = mappingsPage.mappingProductList.locator(
        SELECTORS.multiPriceMapping.firstMappingProductRow(),
      )
      await expect(rows.first()).toBeVisible()
      const visibleCount = await rows.count()
      expect(visibleCount, 'creem filter must surface the seeded Creem rows').toBe(creemCountAll)
      const stripeVisible = await rows.filter({ hasText: 'Stripe' }).count()
      expect(stripeVisible, 'creem filter must NOT surface any Stripe row').toBe(0)
      const allCreem = await rows.evaluateAll((els) =>
        els.every((el) => (el.textContent ?? '').includes('Creem')),
      )
      expect(allCreem, 'every visible row Badge must read Creem').toBe(true)
      await demoLogger.testCode.log(`creem filter: ${visibleCount} rows, all Creem`)
    })

    await test.step('When: 还原 provider=all', async () => {
      await mappingsPage.filterByProvider('all')
      await mappingsPage.waitForDataLoaded()
    })

    await test.step('Then: All 视图恢复并集（Stripe + Creem 行均可见）', async () => {
      const rows = mappingsPage.mappingProductList.locator(
        SELECTORS.multiPriceMapping.firstMappingProductRow(),
      )
      await expect(rows.first()).toBeVisible()
      const stripeVisible = await rows.filter({ hasText: 'Stripe' }).count()
      const creemVisible = await rows.filter({ hasText: 'Creem' }).count()
      expect(stripeVisible, 'All must restore ≥ 1 Stripe row').toBeGreaterThanOrEqual(1)
      expect(creemVisible, 'All must restore ≥ 1 Creem row').toBeGreaterThanOrEqual(1)
      await demoLogger.testCode.log(
        `All restored: Stripe=${stripeVisible}, Creem=${creemVisible}`,
      )
    })
  })

  // ==========================================================================
  // #6 — Empty-state toolbar visibility
  // When the list is empty the toolbar (`provider-filter-select` +
  // `provider-sync-button`) stays visible. Deterministic empty state is
  // reached via the provider filter on the ADMIN realm, which seeds NO Creem
  // mappings — filtering by `creem` yields zero rows server-side and the
  // `emptyState` card renders. No DB mutation required.
  // US-EM-001 (toolbar accessibility regardless of data state).
  // ==========================================================================
  test('#6 空状态工具栏可见性：列表为空时 filter 与 sync 仍可见', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    const mappingsPage = await setupAdminMappingsPage(page, loginPage, demoLogger)

    await test.step('Given: admin realm 默认 All 视图下工具栏可见', async () => {
      // Toolbar renders unconditionally (frontend lines 188-233), but we anchor
      // the assertion on the populated state first to prove the toolbar exists
      // at all in this realm before driving the empty state.
      await expect(mappingsPage.providerFilterSelect).toBeVisible()
      await expect(mappingsPage.providerSyncButton).toBeVisible()
      await demoLogger.testCode.log('Toolbar visible under All (populated state)')
    })

    await test.step('When: 过滤 provider=creem（admin realm 无 Creem 映射）', async () => {
      // The admin realm seed (`_ensure_points_package_payment_demo_data` +
      // `_ensure_multi_price_demo_data`) inserts ONLY Stripe rows. Filtering
      // by `creem` returns zero mappings server-side → `emptyState` renders.
      await mappingsPage.filterByProvider('creem')
    })

    await test.step('Then: 列表进入空状态（emptyState 可见）', async () => {
      // The frontend renders `emptyState` when `allMappings.length === 0`
      // (line 237). This is the deterministic empty state reachable through
      // the provider filter without disrupting the shared seed.
      await expect(mappingsPage.emptyState).toBeVisible({ timeout: 10000 })
      await expect(mappingsPage.mappingProductList).toHaveCount(0)
      await demoLogger.testCode.log('Empty state reached (admin realm has no Creem mappings)')
    })

    await test.step('Then: 空状态下 provider-filter-select 与 provider-sync-button 仍可见', async () => {
      // Load-bearing: the toolbar must remain accessible when the list is
      // empty — a user can still switch filter or trigger a sync to populate
      // the catalog. Persistent structural evidence (testids, not toasts).
      await expect(mappingsPage.providerFilterSelect).toBeVisible()
      await expect(mappingsPage.providerSyncButton).toBeVisible()
      await demoLogger.testCode.log('Toolbar remains visible in empty state')
    })

    await test.step('Cleanup: 还原 provider=all', async () => {
      await mappingsPage.filterByProvider('all')
      await mappingsPage.waitForDataLoaded()
      await demoLogger.testCode.log('Restored provider filter to all')
    })
  })
})

// ============================================================================
// Helpers
// ============================================================================

/**
 * Log in as the DEMO_ADMIN persona and navigate to the entitlement mappings
 * page (admin realm). Used by #6, where the admin realm's lack of Creem
 * mappings is the deterministic empty-state driver.
 */
async function setupAdminMappingsPage(
  page: import('@playwright/test').Page,
  loginPage: import('../pages/login-page').LoginPage,
  demoLogger: UnifiedLogger,
) {
  const { EntitlementMappingsPage } = await import('../pages/entitlement-mappings-page')
  await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', DEMO_ADMIN.realmId)
  const mappingsPage = new EntitlementMappingsPage(page, demoLogger)
  await mappingsPage.goto(DEMO_ADMIN.realmId)
  await mappingsPage.waitForDataLoaded()
  await demoLogger.testCode.log('Admin on entitlement-mappings page (admin realm, master-detail)')
  return mappingsPage
}

/**
 * Log in as the realm-001 admin persona and navigate to the entitlement
 * mappings page. Used by #5, which requires a realm seeded with BOTH Stripe
 * and Creem mappings (only realm-001 qualifies per
 * `_ensure_points_package_payment_demo_data`).
 */
async function setupRealmMappingsPage(
  page: import('@playwright/test').Page,
  loginPage: import('../pages/login-page').LoginPage,
  demoLogger: UnifiedLogger,
  realmId: string,
) {
  const { EntitlementMappingsPage } = await import('../pages/entitlement-mappings-page')
  const admin = REALM_ADMINS[realmId]
  await loginPage.loginAsAdmin(admin.email, admin.password, realmId)
  const mappingsPage = new EntitlementMappingsPage(page, demoLogger)
  await mappingsPage.goto(realmId)
  await mappingsPage.waitForDataLoaded()
  await demoLogger.testCode.log(`Admin on entitlement-mappings page (realm=${realmId}, master-detail)`)
  return mappingsPage
}
