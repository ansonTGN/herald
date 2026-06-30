/**
 * Realm Default Free-Periodic Quota Editor Demo Tests (DE-D04)
 *
 * Role: billing-admin / realm-admin
 * Route: /{realmId}/manage/points/default-config
 *
 * User Story:
 * - US-FU-005 (docs/user-stories/billing/points-free-user.md) — 免费周期积分改为滚动窗口配额
 * - US-PO-009 (docs/user-stories/billing/points-admin.md) — 配置多时间窗滚动配额
 *
 * Design contract:
 * - `.ai/design/points-grant-redesign.md` §4.2 / §4.3.2 / §5.4
 * - `.ai/design-ui/points-grant-redesign/ui-spec.md` §3.3 / §4 / §7
 * - Converged testid contract: `.ai/task/points-grant-redesign/frontend/accept/FE-A07-report.md`
 */

import { expect, type Page } from '@playwright/test'

import { test, cleanupTestData } from '../fixtures/demo-page.fixtures'
import { SELECTORS } from '../selectors'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { loginWithCredentials } from '../helpers/auth'
import {
  setRealmDefaultFreePeriodicQuota,
  clearQuotaEditorRows,
  fillQuotaEditorRows,
  registerNewUserWithRealmDefaultQuota,
} from '../helpers/points-quota-helpers'
import { registerUser } from '../helpers/points-helpers'
import {
  QUOTA_DEMO_REALM,
  QUOTA_DEMO_ADMIN_EMAIL,
  QUOTA_DEMO_PASSWORD,
  DEMO_FREE_QUOTA_WINDOWS,
  REALM_DEFAULT_EDITOR_PREFIX,
} from '../fixtures/points-quota.fixtures'

// ============================================================================
// Constants
// ============================================================================

const TEST_REALM = QUOTA_DEMO_REALM
const ADMIN_EMAIL = QUOTA_DEMO_ADMIN_EMAIL
const ADMIN_PASSWORD = QUOTA_DEMO_PASSWORD

const ORIGINAL_WINDOWS: typeof DEMO_FREE_QUOTA_WINDOWS = []

// ============================================================================
// Helpers
// ============================================================================

async function loginAsAdmin(page: Page): Promise<void> {
  await loginWithCredentials(page, {
    realmId: TEST_REALM,
    email: ADMIN_EMAIL,
    password: ADMIN_PASSWORD,
  })
}

async function openRealmDefaultEditor(page: Page): Promise<void> {
  await page.goto(`/${TEST_REALM}/manage/points/default-config`)
  await expect(
    page.locator('[data-testid="points-default-config-form"]'),
  ).toBeVisible()
  await expect(
    page.locator(SELECTORS.pointsQuotaEditor.editor(REALM_DEFAULT_EDITOR_PREFIX)),
  ).toBeVisible()
}

// ============================================================================
// Test suite
// ============================================================================

test.describe('[Billing Admin] Realm 默认免费周期配额编辑器 (US-FU-005 / US-PO-009)', () => {
  test.beforeEach(async ({ page }) => {
    await verifyTestEnvironment(page, {
      requiredRealms: [TEST_REALM],
      requiredUsers: [ADMIN_EMAIL],
    })
    await loginAsAdmin(page)
  })

  test.afterEach(async ({ page }) => {
    await cleanupTestData(page, TEST_REALM)
  })

  test('US-FU-005 场景1: 配置免费周期滚动窗口并持久化', async ({ page }) => {
    await openRealmDefaultEditor(page)
    await clearQuotaEditorRows(page, REALM_DEFAULT_EDITOR_PREFIX)
    await fillQuotaEditorRows(page, REALM_DEFAULT_EDITOR_PREFIX, DEMO_FREE_QUOTA_WINDOWS)

    await page.locator(SELECTORS.pointsQuotaEditor.saveConfigButton).click()
    await page.waitForLoadState('networkidle')

    // Reload and assert persisted rows.
    await openRealmDefaultEditor(page)
    const editor = page.locator(
      SELECTORS.pointsQuotaEditor.editor(REALM_DEFAULT_EDITOR_PREFIX),
    )
    const rows = editor.locator(
      SELECTORS.pointsQuotaEditor.row(REALM_DEFAULT_EDITOR_PREFIX, 0),
    )
    await expect(rows).toHaveCount(DEMO_FREE_QUOTA_WINDOWS.length)
  })

  test('US-FU-005 场景2: 客户端校验拦截非法窗口配置', async ({ page }) => {
    await openRealmDefaultEditor(page)
    await clearQuotaEditorRows(page, REALM_DEFAULT_EDITOR_PREFIX)

    const editor = page.locator(
      SELECTORS.pointsQuotaEditor.editor(REALM_DEFAULT_EDITOR_PREFIX),
    )
    await page.locator(SELECTORS.pointsQuotaEditor.addButton(REALM_DEFAULT_EDITOR_PREFIX)).click()

    const lengthInput = editor.locator(
      SELECTORS.pointsQuotaEditor.lengthRow(REALM_DEFAULT_EDITOR_PREFIX, 0),
    )
    const limitInput = editor.locator(
      SELECTORS.pointsQuotaEditor.limitRow(REALM_DEFAULT_EDITOR_PREFIX, 0),
    )

    await lengthInput.fill('0')
    await limitInput.fill('-10')

    await expect(lengthInput).toHaveAttribute('aria-invalid', 'true')
    await expect(limitInput).toHaveAttribute('aria-invalid', 'true')

    const saveButton = page.locator(SELECTORS.pointsQuotaEditor.saveConfigButton)
    await expect(saveButton).toBeDisabled()
  })

  test('US-FU-005 场景1: 新注册用户获得默认配额窗口', async ({ page }) => {
    await setRealmDefaultFreePeriodicQuota(
      page,
      TEST_REALM,
      DEMO_FREE_QUOTA_WINDOWS,
    )

    const newUserEmail = `free-${Date.now()}@realm-001.com`
    await registerNewUserWithRealmDefaultQuota(
      page,
      TEST_REALM,
      newUserEmail,
      ADMIN_PASSWORD,
    )

    // registerNewUserWithRealmDefaultQuota already asserts window rows exist.
    const windowRows = page.locator('[data-testid^="points-window-row-"]')
    await expect(windowRows).toHaveCount(DEMO_FREE_QUOTA_WINDOWS.length)
  })

  test('US-FU-005 场景4: 已存在用户不受新默认配置影响', async ({ page }) => {
    // Ensure a fresh realm default is configured.
    await setRealmDefaultFreePeriodicQuota(
      page,
      TEST_REALM,
      DEMO_FREE_QUOTA_WINDOWS,
    )

    // Pre-existing user: use the seeded demo user who has no free-periodic grant.
    await loginWithCredentials(page, {
      realmId: TEST_REALM,
      email: 'user@realm-001.com',
      password: 'password',
    })

    await page.goto(`/${TEST_REALM}/user/points`)
    await expect(page.locator(SELECTORS.pointsUser.page)).toBeVisible()

    // A pre-existing user should not suddenly see free-periodic quota rows
    // because entitlements are snapshotted at grant time.
    const windowRows = page.locator('[data-testid^="points-window-row-"]')
    const count = await windowRows.count()
    expect(count).toBe(0)
  })

  test('US-FU-005 场景4: 编辑器显示"仅影响新注册用户"提示', async ({ page }) => {
    await openRealmDefaultEditor(page)
    await expect(
      page.locator(SELECTORS.pointsQuotaEditor.impactAlert(REALM_DEFAULT_EDITOR_PREFIX)),
    ).toBeVisible()
  })
})
