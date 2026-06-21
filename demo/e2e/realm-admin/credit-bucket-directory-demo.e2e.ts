/**
 * Credit Bucket Directory Management Demo Tests (Realm Admin)
 *
 * User Stories:
 * - US-CB-001 场景1: admin create/edit a Credit Bucket; registration pool is
 *   single-per-realm (409 `registration_pool_conflict` when a second is marked).
 * - US-CB-001 场景2: disabling a bucket keeps already-held balances intact.
 * - US-CB-002 场景1: a coverage set of ≥1 client app is required (schema
 *   `.min(1)` — fail-loud validation error, no silent submit).
 * - US-CB-003 场景1: assigning ≥1 entitlement mapping to a bucket persists.
 *
 * Design truth: `.ai/design/credit-bucket.md` §4.2.2/§4.2.3 (POST/PUT/DELETE +
 * error codes `bucket_key_duplicate` 400, `registration_pool_conflict` 409,
 * `bucket_in_use` 409), §4.3.2 (schema), note A4 (NO `isDefault` control).
 *
 * Role: realm-admin (`REALM_ADMINS[realmId]`). Navigation is by route
 * `/${realmId}/manage/billing/credit-buckets` — the sidebar entry testid
 * `sidebar-menu-credit-buckets` is i18n-derived and therefore intentionally
 * NOT used (see selectors.ts creditBucket loud note).
 *
 * Test Structure:
 * - `test.describe('[Realm Admin] Credit Bucket 目录管理 (US-CB-001/002/003)')`.
 * - Each `test()` maps to one US scenario with BDD `test.step('Given/When/Then …')`
 *   in Chinese, mirroring `billing-admin/points-admin-comprehensive-demo.e2e.ts`.
 * - `beforeEach`: `verifyTestEnvironment`; `afterEach`: `cleanupTestData`.
 * - All assertions land on persistent state (list-item / detail / disabled-badge
 *   / stable error region / admin-wallets row). Toasts are auxiliary only.
 *
 * Authoring only (DE-D02). Execution is owned by DE-D06 via
 * `--grep "Credit Bucket 目录管理"`. This file is type-checked, not run, here.
 */

import { test, cleanupTestData, expect } from '../fixtures/demo-page.fixtures'
import { SELECTORS } from '../selectors'
import { DEMO_ADMIN, REALM_ADMINS } from '../helpers/auth'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { CreditBucketDirectoryPage } from '../pages/credit-bucket-directory-page'
import {
  createBucketViaUI,
  editBucketViaUI,
  openBucketEditor,
  setRegistrationPoolViaUI,
  bindCoverageSetViaUI,
  assignMappingsViaUI,
  requestDeleteBucketViaUI,
  confirmDeleteBucket,
  parseBucketIdFromListItem,
  listBucketsViaApi,
} from '../helpers/bucket-helpers'
import {
  CREDIT_BUCKET_KEYS,
  CREDIT_BUCKET_REALMS,
} from '../helpers/bucket-seed-ids'

/**
 * Realm used for admin-side directory tests.
 *
 * `realm-001` is the primary demo realm: it seeds the Credit Bucket directory
 * (`primary-pool` registration receiver + `promo-pool` secondary), a points
 * demo user (`user@realm-001.com`) holding balance in `primary-pool`, and
 * one-time entitlement mappings (one intentionally `bucket_id = NULL`).
 */
const REALM_ID = CREDIT_BUCKET_REALMS.POINTS

/**
 * A4 — design truth: NO `isDefault` / `set-default` / `default-bucket` control
 * exists anywhere in the Bucket editor (`.ai/design/credit-bucket.md` A4 +
 * `frontend/src/components/billing/credit-bucket/*.tsx` — verified by grep:
 * no `data-testid` matching these patterns is emitted). The test below asserts
 * this absence with a negative locator so a future regression that reintroduces
 * a default-bucket control is caught loudly.
 */
const A4_DEFAULT_CONTROL_TESTIDS =
  '[data-testid*="is-default"], [data-testid*="set-default"], [data-testid*="default-bucket"]'

test.describe('[Realm Admin] Credit Bucket 目录管理 (US-CB-001/002/003)', () => {
  let testStartTime: number

  test.beforeEach(async ({ page }) => {
    testStartTime = Date.now()
    await verifyTestEnvironment(page, {
      requiredRealms: [REALM_ID],
      requiredUsers: [REALM_ADMINS[REALM_ID]?.email ?? DEMO_ADMIN.email],
    })
  })

  test.afterEach(async ({ page }) => {
    await cleanupTestData(page, REALM_ID, { timestamp: testStartTime })
  })

  // ==========================================================================
  // US-CB-001 场景1 (a): create + edit a Credit Bucket
  // ==========================================================================

  test('US-CB-001 场景1 (a): 创建并编辑 Credit Bucket', async ({ page, loginPage, demoLogger }) => {
    const directory = new CreditBucketDirectoryPage(page, demoLogger)
    const suffix = `${testStartTime}`
    const bucketKey = `e2e-create-${suffix}`
    const initialName = `E2E Create ${suffix}`
    const editedName = `E2E Edited ${suffix}`

    await test.step('Given: realm-admin 已登录', async () => {
      const admin = REALM_ADMINS[REALM_ID] ?? {
        email: DEMO_ADMIN.email,
        password: DEMO_ADMIN.password,
      }
      await loginPage.loginAsAdmin(admin.email, admin.password, REALM_ID)
      demoLogger.testCode.log('[Test] ✓ realm-admin 登录成功')
    })

    await test.step('When: 通过路由访问 Credit Bucket 目录页（非 sidebar testid）', async () => {
      await directory.gotoDirectory(REALM_ID)
      demoLogger.testCode.log('[Test] ✓ 已通过路由进入目录页，规避 i18n 派生 testid')
    })

    let createdBucketId: string
    await test.step('When: 新建 Bucket（填写 bucket_key + name + ≥1 覆盖项）', async () => {
      // Resolve a real client-app id by briefly opening the editor to read the
      // coverage multiselect (lists every client app in the realm). This avoids
      // hardcoding an app UUID that could drift across re-seeds. The create flow
      // itself is then driven by `createBucketViaUI`, which owns navigation +
      // editor open/close as a single atomic helper.
      const coverageId = await discoverFirstCoverageAppId(page, directory)
      createdBucketId = await createBucketViaUI(page, REALM_ID, {
        bucketKey,
        name: initialName,
        description: 'created by DE-D02',
        clientAppIds: [coverageId],
      })
      demoLogger.testCode.log(`[Test] ✓ Bucket 已创建，bucketId=${createdBucketId}`)
    })

    await test.step('Then: 目录列表渲染新 bucket 项；默认启用（无 -disabled-badge）', async () => {
      const item = page.locator(SELECTORS.creditBucket.listItem(createdBucketId!))
      await expect(item).toBeVisible({ timeout: 10000 })
      // Newly created bucket defaults to enabled=true (POST default). The
      // disabled badge is conditionally rendered only when enabled=false, so its
      // absence on the item is the persistent signal of the default-enabled state.
      await expect(
        page.locator(SELECTORS.creditBucket.listItemDisabledBadge(createdBucketId!)),
      ).toHaveCount(0)
      demoLogger.testCode.log('[Test] ✓ 新 bucket 默认启用，未渲染 disabled badge')
    })

    await test.step('When: 编辑 bucket 的 name 字段并保存', async () => {
      await editBucketViaUI(page, createdBucketId!, { name: editedName })
      demoLogger.testCode.log('[Test] ✓ 编辑表单已提交')
    })

    await test.step('Then: 列表项名称更新为编辑后的值（持久化状态）', async () => {
      const item = page.locator(SELECTORS.creditBucket.listItem(createdBucketId!))
      await expect(item).toContainText(editedName)
      // And the original name is no longer the rendered label.
      await expect(item).not.toContainText(initialName)
      demoLogger.testCode.log('[Test] ✓ 列表项名称已更新为编辑值')
    })

    await test.step('Then: A4 — 编辑器内不存在 *is-default* / *set-default* / *default-bucket* 控件', async () => {
      // Open the editor for the just-edited bucket and assert no default-bucket
      // control exists within the editor scope. Documents design decision A4.
      await openBucketEditor(page, createdBucketId!)
      const editorScope = page.locator(SELECTORS.creditBucket.editor)
      await expect(editorScope.locator(A4_DEFAULT_CONTROL_TESTIDS)).toHaveCount(0)
      demoLogger.testCode.log('[Test] ✓ A4: 编辑器内未发现 default-bucket 控件')
    })
  })

  // ==========================================================================
  // US-CB-001 场景1 (b): registration pool is single-per-realm (409 conflict)
  // ==========================================================================

  test('US-CB-001 场景1 (b): 注册池单 per-realm（第二次设置成功，第三次触发 409 registration_pool_conflict）', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    const directory = new CreditBucketDirectoryPage(page, demoLogger)
    const suffix = `${testStartTime}`
    const secondKey = `e2e-reg-2-${suffix}`
    const thirdKey = `e2e-reg-3-${suffix}`

    await test.step('Given: realm-admin 已登录并进入目录页', async () => {
      const admin = REALM_ADMINS[REALM_ID] ?? {
        email: DEMO_ADMIN.email,
        password: DEMO_ADMIN.password,
      }
      await loginPage.loginAsAdmin(admin.email, admin.password, REALM_ID)
      await directory.gotoDirectory(REALM_ID)
    })

    await test.step('Given: 该 realm 已存在一个注册池（seeded primary-pool）', async () => {
      // The seed always creates `primary-pool` with receives_registration_credits=true.
      // List via API to assert the precondition holds (loud if seed drifted).
      const buckets = await listBucketsViaApi(page, REALM_ID)
      const registrationPool = buckets.find((b) => b.receivesRegistrationCredits)
      expect(
        registrationPool,
        'seeded registration pool (primary-pool) must exist; seed drift suspected',
      ).toBeTruthy()
      demoLogger.testCode.log(
        `[Test] ✓ 前置：realm 已有注册池 bucketKey=${registrationPool!.bucketKey}`,
      )
    })

    let secondBucketId: string
    await test.step('When: 新建第 2 个 bucket 并标记为注册池（应成功，列表显示 -registration-badge）', async () => {
      // To mark the 2nd bucket as the registration pool, we must first UNSET the
      // seeded primary-pool's flag (single-per-realm constraint). Do it via UI
      // so the test exercises the real admin path.
      const seededPrimaryId = await parseBucketIdFromListItem(
        page,
        CREDIT_BUCKET_KEYS.PRIMARY_POOL,
      )
      await setRegistrationPoolViaUI(page, seededPrimaryId, false)

      // Now create a 2nd bucket and mark IT as the registration pool.
      secondBucketId = await createBucketViaUI(page, REALM_ID, {
        bucketKey: secondKey,
        name: `E2E Reg2 ${suffix}`,
        clientAppIds: [await discoverFirstCoverageAppId(page, directory)],
        receivesRegistrationCredits: true,
      })
      const badge = page.locator(
        SELECTORS.creditBucket.listItemRegistrationBadge(secondBucketId),
      )
      await expect(badge).toBeVisible({ timeout: 10000 })
      demoLogger.testCode.log(
        `[Test] ✓ 第 2 个 bucket 成功设为注册池，列表渲染 -registration-badge`,
      )
    })

    await test.step('When: 在已有注册池的前提下，标记第 3 个 bucket 为注册池', async () => {
      // Create a 3rd bucket (NOT a registration pool) and then attempt to flip
      // its registration switch — this must collide with the 2nd bucket.
      const thirdBucketId = await createBucketViaUI(page, REALM_ID, {
        bucketKey: thirdKey,
        name: `E2E Reg3 ${suffix}`,
        clientAppIds: [await discoverFirstCoverageAppId(page, directory)],
      })
      // Attempt to mark it as registration pool while the 2nd bucket still holds
      // the flag. The helper surfaces the 409 conflict alert visibility.
      const conflictShown = await setRegistrationPoolViaUI(page, thirdBucketId, true)
      expect(conflictShown, 'expected registration_pool_conflict 409 alert to render').toBe(true)
    })

    await test.step('Then: 编辑器渲染稳定的 409 registration_pool_conflict 错误区（非 toast）', async () => {
      // The conflict alert is a persistent destructive Alert inside the editor,
      // not an auto-dismissing toast. It must remain visible after the 409.
      const conflictAlert = page.locator(SELECTORS.creditBucket.editorRegistrationConflict)
      await expect(conflictAlert).toBeVisible()
      // Persistent state: still visible after a short wait (not auto-dismissed).
      await page.waitForTimeout(1500)
      await expect(conflictAlert).toBeVisible()
      demoLogger.testCode.log(
        '[Test] ✓ 稳定渲染 registration_pool_conflict 错误区，未依赖 toast',
      )
    })
  })

  // ==========================================================================
  // US-CB-002 场景1: coverage set ≥1 required (fail-loud)
  // ==========================================================================

  test('US-CB-002 场景1: 覆盖集 ≥1 必填（空覆盖提交被阻止；绑定后列表项 coveredClientAppCount ≥1）', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    const directory = new CreditBucketDirectoryPage(page, demoLogger)
    const suffix = `${testStartTime}`
    const bucketKey = `e2e-coverage-${suffix}`
    const bucketName = `E2E Coverage ${suffix}`

    await test.step('Given: realm-admin 已登录并进入目录页', async () => {
      const admin = REALM_ADMINS[REALM_ID] ?? {
        email: DEMO_ADMIN.email,
        password: DEMO_ADMIN.password,
      }
      await loginPage.loginAsAdmin(admin.email, admin.password, REALM_ID)
      await directory.gotoDirectory(REALM_ID)
    })

    await test.step('When: 尝试在未选择任何覆盖项的情况下提交新建表单', async () => {
      await directory.clickNewBucket()
      await page.locator(SELECTORS.creditBucket.editorBucketKey).fill(bucketKey)
      await page.locator(SELECTORS.creditBucket.editorName).fill(bucketName)
      // Intentionally do NOT pick any coverage item.
      await page.locator(SELECTORS.creditBucket.editorSubmit).click()
    })

    await test.step('Then: 表单 fail-loud —— 覆盖集错误区可见，编辑器保持打开，列表未出现该 bucket', async () => {
      // Schema enforces clientAppIds.min(1); TanStack Form shows the schema error
      // inside the coverage multiselect error region after the submit attempt.
      const coverageError = page.locator(SELECTORS.creditBucket.coverageMultiselectError)
      await expect(coverageError).toBeVisible({ timeout: 5000 })

      // Editor stays open (no successful create).
      await expect(page.locator(SELECTORS.creditBucket.editor)).toBeVisible()

      // No list item with this bucketKey has been created.
      const directoryPage = page.locator(SELECTORS.creditBucket.directoryPage)
      await expect(directoryPage).not.toContainText(bucketKey)
      demoLogger.testCode.log(
        '[Test] ✓ 空覆盖提交被 fail-loud 阻止，未创建 bucket',
      )
    })

    let createdBucketId: string
    await test.step('When: 绑定 ≥1 覆盖项后重新提交', async () => {
      // `discoverFirstCoverageAppId` opens a throwaway create editor to read the
      // coverage options, then navigates back to the directory root (closing
      // that editor). Re-open the create editor here and re-fill the
      // bucketKey/name (the previous attempt's values were never persisted —
      // the empty-coverage submit was rejected by schema validation), then
      // bind the discovered coverage id and submit.
      const coverageId = await discoverFirstCoverageAppId(page, directory)
      await directory.clickNewBucket()
      await page.locator(SELECTORS.creditBucket.editorBucketKey).fill(bucketKey)
      await page.locator(SELECTORS.creditBucket.editorName).fill(bucketName)
      await bindCoverageSetViaUI(page, [coverageId])
      await page.locator(SELECTORS.creditBucket.editorSubmit).click()
      // Editor closes on successful create.
      await expect(page.locator(SELECTORS.creditBucket.editor)).toBeHidden({
        timeout: 15000,
      })
      createdBucketId = await parseBucketIdFromListItem(page, bucketKey)
    })

    await test.step('Then: 列表项持久化渲染 coveredClientAppCount ≥1', async () => {
      // The list item footer renders `m.credit_buckets.covered_apps_count({count})`.
      // Assert the count is non-zero by checking the rendered text contains a
      // digit ≥1 for the coverage count. The list item carries the bucketKey
      // text plus counts in its footer; assert it contains at least one covered
      // app by checking the item text reflects a non-zero coverage.
      const item = page.locator(SELECTORS.creditBucket.listItem(createdBucketId!))
      await expect(item).toBeVisible()
      // Persistent state: the bucketKey is present and the coverage count text
      // is rendered (the exact label is i18n, but the count is a number; verify
      // the item does NOT show the zero-coverage shape by confirming the footer
      // rendered the coverage count segment — match a digit other than 0 in
      // the coverage position is brittle, so instead confirm via API that the
      // bucket has coveredClientAppCount >= 1, the authoritative source).
      const buckets = await listBucketsViaApi(page, REALM_ID)
      const created = buckets.find((b) => b.bucketKey === bucketKey)
      expect(created, 'created bucket must be listable via API').toBeTruthy()
      expect(
        created!.coveredClientAppCount,
        'coveredClientAppCount must be >= 1 after binding coverage',
      ).toBeGreaterThanOrEqual(1)
      demoLogger.testCode.log(
        `[Test] ✓ 列表项/API 显示 coveredClientAppCount=${created!.coveredClientAppCount}`,
      )
    })
  })

  // ==========================================================================
  // US-CB-003 场景1: assign mapping → bucket (persists across reopen)
  // ==========================================================================

  test('US-CB-003 场景1: 给 Bucket 分配 ≥1 套餐映射并验证持久化', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    const directory = new CreditBucketDirectoryPage(page, demoLogger)
    const suffix = `${testStartTime}`
    const bucketKey = `e2e-mapping-${suffix}`
    const bucketName = `E2E Mapping ${suffix}`

    await test.step('Given: realm-admin 已登录并进入目录页', async () => {
      const admin = REALM_ADMINS[REALM_ID] ?? {
        email: DEMO_ADMIN.email,
        password: DEMO_ADMIN.password,
      }
      await loginPage.loginAsAdmin(admin.email, admin.password, REALM_ID)
      await directory.gotoDirectory(REALM_ID)
    })

    let createdBucketId: string
    await test.step('When: 新建一个 bucket 作为映射分配目标', async () => {
      const coverageId = await discoverFirstCoverageAppId(page, directory)
      createdBucketId = await createBucketViaUI(page, REALM_ID, {
        bucketKey,
        name: bucketName,
        clientAppIds: [coverageId],
      })
    })

    let assignedMappingId: string
    let countBefore: number
    await test.step('When: 打开编辑器并分配 ≥1 个套餐映射', async () => {
      const buckets = await listBucketsViaApi(page, REALM_ID)
      const created = buckets.find((b) => b.bucketKey === bucketKey)
      expect(created, 'created bucket must be listable').toBeTruthy()
      countBefore = created!.entitlementMappingCount

      await openBucketEditor(page, createdBucketId!)
      // Resolve the first available mapping item rendered by the mappings
      // multiselect, then toggle it ON in the same open-popover session. The
      // multiselect search input lives inside a Radix Popover; click the
      // `bucket-mappings-multiselect` trigger to open it. Reading the item id
      // and toggling it in one session avoids the close+reopen round-trip
      // (which was dropping the toggle because the second trigger-click closed
      // the popover instead of opening it).
      await page.locator(SELECTORS.creditBucket.mappingsMultiselect).click()
      const mappingsSearch = page.locator(SELECTORS.creditBucket.mappingsMultiselectSearch)
      await expect(mappingsSearch).toBeVisible({ timeout: 5000 })
      await mappingsSearch.click()
      const firstMappingItem = page.locator(
        '[data-testid^="bucket-mappings-multiselect-item-"]',
      )
      await expect(firstMappingItem.first()).toBeVisible({ timeout: 5000 })
      const mappingTestid = await firstMappingItem.first().getAttribute('data-testid')
      assignedMappingId = mappingTestid!.replace('bucket-mappings-multiselect-item-', '')
      // Click the item to toggle it ON. Radix CommandItem's onSelect fires on
      // click; the popover stays open (we close it via Escape afterwards).
      await firstMappingItem.first().click()
      // Wait for the multiselect trigger to reflect the selection: when
      // `value.length > 0`, the trigger renders a count Badge with the value
      // length followed by the selected labels (see BucketMultiselect). Assert
      // the trigger's text contains "1" (the count) as the visible signal that
      // the form's `entitlementMappingIds` now holds the toggled mapping.
      await expect(page.locator(SELECTORS.creditBucket.mappingsMultiselect)).toContainText('1', {
        timeout: 3000,
      })
      await page.keyboard.press('Escape')
      await expect(mappingsSearch).toBeHidden({ timeout: 2000 })

      // The PUT writes the new mapping→bucket binding transactionally. Wait
      // for the PUT response (a positive save-completion signal) before any
      // re-read: the editor stays open on update (it only refetches list/
      // detail via onSaved), so editor-closed cannot be used as the signal
      // here (unlike the create flow). Waiting on the conflict-alert's
      // absence is a WEAK signal — the alert simply has not rendered yet,
      // so a subsequent list re-read can race ahead of the commit and see
      // the pre-PUT count (observed flaky 0-vs-1 in DE-D06 re-run).
      const putResponse = page.waitForResponse(
        (r) =>
          r.request().method() === 'PUT' &&
          r.url().includes(`/billing/credit-buckets/${createdBucketId!}`),
        { timeout: 15000 },
      )
      await page.locator(SELECTORS.creditBucket.editorSubmit).click()
      const resp = await putResponse
      expect(resp.ok(), `editor PUT must succeed: ${resp.status()}`).toBe(true)
      // On a successful save the conflict alert must not render.
      await expect(page.locator(SELECTORS.creditBucket.editorRegistrationConflict)).toBeHidden({
        timeout: 5000,
      })
    })

    await test.step('Then: API 显示 entitlementMappingCount 增长（持久化）', async () => {
      const buckets = await listBucketsViaApi(page, REALM_ID)
      const created = buckets.find((b) => b.bucketKey === bucketKey)
      expect(created, 'created bucket must still be listable').toBeTruthy()
      expect(
        created!.entitlementMappingCount,
        'entitlementMappingCount must increase after assigning a mapping',
      ).toBeGreaterThan(countBefore!)
      demoLogger.testCode.log(
        `[Test] ✓ entitlementMappingCount: ${countBefore} -> ${created!.entitlementMappingCount}`,
      )
    })

    await test.step('Then: 重新打开编辑器确认映射选择已持久化', async () => {
      // The editor resets from the bucket detail on open; the assigned mapping
      // must show as selected (Check icon visible) in the mappings multiselect.
      await openBucketEditor(page, createdBucketId!)
      // Open the mappings popover BEFORE interacting with its search input:
      // the search input lives inside a Radix Popover that is closed until the
      // multiselect trigger is clicked (mirrors the open-popover pattern used
      // at the assignment step above; clicking the search while the popover
      // is closed hangs until the test timeout).
      await page.locator(SELECTORS.creditBucket.mappingsMultiselect).click()
      const mappingsSearch = page.locator(SELECTORS.creditBucket.mappingsMultiselectSearch)
      await expect(mappingsSearch).toBeVisible({ timeout: 5000 })
      await mappingsSearch.click()
      const assignedItem = page.locator(
        SELECTORS.creditBucket.mappingsMultiselectItem(assignedMappingId!),
      )
      await expect(assignedItem).toBeVisible({ timeout: 5000 })
      // The Check icon inside the selected item has opacity-100 when selected.
      const checkIcon = assignedItem.locator('svg.lucide-check')
      await expect(checkIcon).toBeVisible()
      await expect(checkIcon).toHaveClass(/opacity-100/)
      demoLogger.testCode.log('[Test] ✓ 重新打开编辑器后映射仍处于已选状态')
    })
  })

  // ==========================================================================
  // US-CB-001 场景2: disabling a bucket keeps held balances intact
  // ==========================================================================

  test('US-CB-001 场景2: 禁用 Bucket 后已持有余额保持不变', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    await test.step('Given: realm-admin 已登录', async () => {
      const admin = REALM_ADMINS[REALM_ID] ?? {
        email: DEMO_ADMIN.email,
        password: DEMO_ADMIN.password,
      }
      await loginPage.loginAsAdmin(admin.email, admin.password, REALM_ID)
    })

    const { userId, bucketId, balanceBefore } = await test.step('When: 在 admin wallets 页定位一个有余额的 seed bucket 持有者', async () => {
      // The admin wallets page groups rows by (userId, bucketId). The seeded
      // `primary-pool` always has holders with non-zero balance. Resolve both
      // ids from the rendered row testid rather than hardcoding UUIDs.
      await page.goto(`/${REALM_ID}/manage/points/wallets`)
      await expect(page.locator(SELECTORS.pointsAdmin.accountsPage)).toBeVisible({
        timeout: 10000,
      })

      // Find the first row whose testid encodes the primary-pool bucket. We
      // don't know the seeded bucket's UUID ahead of time, so scan the rendered
      // rows and pick one that exists. If the wallets page is empty (seed drift),
      // fail loud with a clear message — do not fake a holder.
      //
      // The row testid format is `admin-wallet-row-${userId}-${bucketId}` where
      // BOTH ids are UUIDs (each containing 4 hyphens). A greedy `(.+)-(.+)`
      // regex would split at the LAST hyphen and mis-extract — pin to the UUID
      // shape `[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}` so
      // the two ids are unambiguous.
      const anyRow = page.locator('[data-testid^="admin-wallet-row-"]')
      await expect(anyRow.first()).toBeVisible({ timeout: 10000 })
      const rowTestid = await anyRow.first().getAttribute('data-testid')
      const UUID = '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}'
      const rowTestidRe = new RegExp(`^admin-wallet-row-(${UUID})-(${UUID})$`)
      expect(rowTestid, 'admin-wallets row testid must encode userId+bucketId').toMatch(
        rowTestidRe,
      )
      const [, userId, bucketId] = rowTestid!.match(rowTestidRe)!

      // Read the held balance TOTAL from this row before disabling. The row
      // renders the bucket total inside a `<div class="text-2xl font-bold">`
      // (PointsWalletsPage.tsx renderWalletRow). Reading the WHOLE row text
      // would be brittle: disabling the bucket later inserts a `Disabled`
      // Badge into the metadata row, which legitimately changes the row text
      // without affecting the balance — comparing whole-row text would then
      // produce a phantom failure. Pin to the total cell so the assertion
      // verifies the actual invariant (held balance unchanged by disable).
      const row = page.locator(SELECTORS.pointsAdmin.walletRowByBucket(userId, bucketId))
      const balanceTotalCell = row.locator('.text-2xl.font-bold').first()
      await expect(balanceTotalCell).toBeVisible()
      const balanceBefore = ((await balanceTotalCell.textContent()) ?? '').trim()
      demoLogger.testCode.log(
        `[Test] ℹ 定位到持有者 userId=${userId} bucketId=${bucketId} 余额=${balanceBefore}`,
      )
      return { userId, bucketId, balanceBefore }
    })

    await test.step('When: 禁用该 bucket（editor-enabled 关闭并保存）', async () => {
      // Navigate back to the directory and disable the bucket via the editor.
      await page.goto(`/${REALM_ID}/manage/billing/credit-buckets`)
      await expect(page.locator(SELECTORS.creditBucket.directoryPage)).toBeVisible()
      await editBucketViaUI(page, bucketId, { enabled: false })
      demoLogger.testCode.log(`[Test] ✓ bucket ${bucketId} 已禁用`)
    })

    await test.step('Then: 目录列表渲染 -disabled-badge', async () => {
      const disabledBadge = page.locator(
        SELECTORS.creditBucket.listItemDisabledBadge(bucketId),
      )
      await expect(disabledBadge).toBeVisible({ timeout: 10000 })
    })

    await test.step('Then: admin wallets 页该持有者余额不变（非零，持久状态）', async () => {
      // Re-read the SAME balance total cell after disabling. Disable affects
      // future grants, not existing holdings (design §4.2.3), so the total
      // must be identical. The `Disabled` Badge is intentionally NOT part of
      // this comparison (it is the expected new UI state, not a balance
      // change).
      await page.goto(`/${REALM_ID}/manage/points/wallets`)
      await expect(page.locator(SELECTORS.pointsAdmin.accountsPage)).toBeVisible()
      const row = page.locator(SELECTORS.pointsAdmin.walletRowByBucket(userId, bucketId))
      await expect(row).toBeVisible({ timeout: 10000 })
      const balanceTotalCell = row.locator('.text-2xl.font-bold').first()
      await expect(balanceTotalCell).toBeVisible()
      const balanceAfter = ((await balanceTotalCell.textContent()) ?? '').trim()
      expect(balanceAfter, 'held balance total must be unchanged after disable').toBe(balanceBefore)
      // Also assert the balance is non-trivial (seeded holder carries balance).
      expect(
        balanceAfter,
        'held balance must be a non-zero numeric total (seeded holder)',
      ).toMatch(/[1-9]/)
      demoLogger.testCode.log(
        `[Test] ✓ 禁用后余额未变: before="${balanceBefore}" after="${balanceAfter}"`,
      )
    })

    await test.step('Cleanup: 恢复 bucket 启用状态（避免污染其他 demo）', async () => {
      await page.goto(`/${REALM_ID}/manage/billing/credit-buckets`)
      await editBucketViaUI(page, bucketId, { enabled: true })
      demoLogger.testCode.log('[Test] ✓ 已恢复 bucket 启用状态')
    })
  })

  // ==========================================================================
  // US-CB-001 delete: blocked when bucket in use; succeeds for empty throwaway
  // ==========================================================================

  test('US-CB-001 删除: 使用中 bucket 被 409 bucket_in_use 阻止；空 throwaway bucket 删除成功', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    const directory = new CreditBucketDirectoryPage(page, demoLogger)
    await test.step('Given: realm-admin 已登录', async () => {
      const admin = REALM_ADMINS[REALM_ID] ?? {
        email: DEMO_ADMIN.email,
        password: DEMO_ADMIN.password,
      }
      await loginPage.loginAsAdmin(admin.email, admin.password, REALM_ID)
    })

    await test.step('When: 选取一个有活跃持有者/订阅的 seed bucket 并请求删除', async () => {
      await page.goto(`/${REALM_ID}/manage/billing/credit-buckets`)
      await expect(page.locator(SELECTORS.creditBucket.directoryPage)).toBeVisible()

      // The seeded `primary-pool` carries registration + wallet holders + the
      // seeded subscription, so deleting it must hit `bucket_in_use`.
      // LOUD NOTE: if this 409 cannot be reproduced because the seed has been
      // emptied (env drift), the assertion below will fail loudly rather than
      // fake the rejection. See item handoff_summary.
      const seededPrimaryId = await parseBucketIdFromListItem(
        page,
        CREDIT_BUCKET_KEYS.PRIMARY_POOL,
      )
      await requestDeleteBucketViaUI(page, seededPrimaryId)
    })

    await test.step('Then: 确认删除触发 409 bucket_in_use（持久错误区，非 toast）', async () => {
      const outcome = await confirmDeleteBucket(page)
      expect(outcome.success, 'delete of in-use bucket must be refused').toBe(false)
      expect(
        outcome.errorCode,
        'expected bucket_in_use error code in stable error region',
      ).toBe('bucket_in_use')
      // The error message region is persistent (AlertDialog stays open on 409).
      await expect(page.locator(SELECTORS.creditBucket.deleteErrorMessage)).toBeVisible()
      // Cancel out of the dialog so the directory returns to a clean state.
      await page.locator(SELECTORS.creditBucket.deleteCancelButton).click()
      await expect(page.locator(SELECTORS.creditBucket.deleteConfirmDialog)).toBeHidden()
      demoLogger.testCode.log(
        `[Test] ✓ bucket_in_use 409 触发，errorCode=${outcome.errorCode}`,
      )
    })

    let throwawayId: string
    await test.step('When: 新建一个空 throwaway bucket', async () => {
      const suffix = `${testStartTime}`
      const coverageId = await discoverFirstCoverageAppId(page, directory)
      throwawayId = await createBucketViaUI(page, REALM_ID, {
        bucketKey: `e2e-throwaway-${suffix}`,
        name: `E2E Throwaway ${suffix}`,
        clientAppIds: [coverageId],
      })
      await expect(page.locator(SELECTORS.creditBucket.listItem(throwawayId!))).toBeVisible()
    })

    await test.step('Then: 删除空 throwaway bucket 成功（列表项移除）', async () => {
      await requestDeleteBucketViaUI(page, throwawayId!)
      const outcome = await confirmDeleteBucket(page)
      expect(outcome.success, 'empty throwaway bucket delete must succeed').toBe(true)
      // Persistent state: the list item is gone.
      await expect(page.locator(SELECTORS.creditBucket.listItem(throwawayId!))).toHaveCount(0)
      demoLogger.testCode.log('[Test] ✓ 空 throwaway bucket 删除成功，列表项已移除')
    })
  })

  // ==========================================================================
  // Helper: resolve the first coverage app id from the editor multiselect.
  // ==========================================================================

  /**
   * Resolve a client-app id from the coverage multiselect by briefly opening the
   * "new bucket" editor, reading the first item's testid suffix, then navigating
   * back to the directory root. Used to avoid hardcoding client-app UUIDs that
   * could drift across re-seeds. The directory POM is used so the helper leaves
   * the page on the directory page (no open editor) when it returns.
   *
   * Implemented as a module-scope helper so every test in this describe shares
   * one deterministic coverage-id resolution path.
   */
  async function discoverFirstCoverageAppId(
    page: import('@playwright/test').Page,
    directory: CreditBucketDirectoryPage,
  ): Promise<string> {
    // Navigate to the directory root (closes any open editor), then open the
    // create editor to read the coverage multiselect. The multiselect search
    // input lives inside a Radix Popover and is NOT in the DOM until the
    // `bucket-coverage-multiselect` trigger is clicked — click the trigger
    // first, then the search input.
    await directory.gotoDirectory(REALM_ID)
    await directory.clickNewBucket()
    await page.locator(SELECTORS.creditBucket.coverageMultiselect).click()
    const search = page.locator(SELECTORS.creditBucket.coverageMultiselectSearch)
    await expect(search).toBeVisible({ timeout: 5000 })
    await search.click()
    const firstItem = page.locator('[data-testid^="bucket-coverage-multiselect-item-"]').first()
    await expect(firstItem).toBeVisible({ timeout: 5000 })
    const testid = await firstItem.getAttribute('data-testid')
    // Close the popover via Escape (blur does NOT close Radix Popover; without
    // this the popover stays open and interferes with the next editor interaction).
    await page.keyboard.press('Escape')
    await expect(search).toBeHidden({ timeout: 2000 })
    // Return to the directory root so callers (e.g. `createBucketViaUI`) start
    // from a known navigation state.
    await directory.gotoDirectory(REALM_ID)
    return testid!.replace('bucket-coverage-multiselect-item-', '')
  }
})
