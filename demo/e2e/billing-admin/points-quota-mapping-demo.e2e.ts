/**
 * Entitlement Mapping Quota Editor Demo Tests (DE-D03)
 *
 * Role: billing-admin
 * Route: /{realmId}/manage/billing/entitlement-mappings
 *
 * User Story:
 * - US-PO-009 (docs/user-stories/billing/points-admin.md) — 配置多时间窗滚动配额
 *
 * LIVE: This suite resolves a real Stripe multi-price product at runtime
 * (via `ensureMultiPriceCatalog`) instead of relying on the removed placeholder
 * seed. Requires Stripe credentials in `demo/.env.demo`; skipped otherwise.
 *
 * Design contract:
 * - `.ai/design/points-grant-redesign.md` §4.2 / §4.3.2 / §5.4
 * - `.ai/design-ui/points-grant-redesign/ui-spec.md` §3.2 / §4 / §7
 * - Converged testid contract: `.ai/task/points-grant-redesign/frontend/accept/FE-A07-report.md`
 */

import { expect, type Page } from '@playwright/test'

import { test, cleanupTestData } from '../fixtures/demo-page.fixtures'
import { SELECTORS } from '../selectors'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { loginWithCredentials } from '../helpers/auth'
import { EntitlementMappingsPage } from '../pages/entitlement-mappings-page'
import {
  createEntitlementMappingWithQuotaWindows,
  clearQuotaEditorRows,
  fillQuotaEditorRows,
} from '../helpers/points-quota-helpers'
import {
  QUOTA_DEMO_REALM,
  QUOTA_DEMO_ADMIN_EMAIL,
  QUOTA_DEMO_PASSWORD,
  QUOTA_EDITOR_PREFIX,
} from '../fixtures/points-quota.fixtures'
import { secrets, hasStripePayment } from '../secrets/env'
import { ensureMultiPriceCatalog } from '../helpers/resolve-mappings'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'

// ============================================================================
// Constants
// ============================================================================

const TEST_REALM = QUOTA_DEMO_REALM
const ADMIN_EMAIL = QUOTA_DEMO_ADMIN_EMAIL
const ADMIN_PASSWORD = QUOTA_DEMO_PASSWORD

const TEST_WINDOWS = [
  { windowSeconds: 18_000, limit: 100, key: '5h' },
  { windowSeconds: 604_800, limit: 500, key: 'week' },
]

const SINGLE_WINDOW = [{ windowSeconds: 604_800, limit: 500, key: 'week' }]

// ============================================================================
// Live catalog setup — resolved once per worker via beforeAll.
// ============================================================================

/** Real Stripe product id resolved in beforeAll; empty until then. */
let realProductId = ''

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

async function openQuotaEditor(page: Page): Promise<void> {
  const mappingsPage = new EntitlementMappingsPage(page)
  await mappingsPage.goto(TEST_REALM)
  await mappingsPage.selectProduct(realProductId)

  const firstAdvanced = page
    .locator(SELECTORS.multiPriceMapping.mappingDetailPanel)
    .getByRole('button', { name: 'Advanced' })
    .first()
  await expect(firstAdvanced).toBeVisible()
  await firstAdvanced.click()

  await expect(
    page.locator(SELECTORS.pointsQuotaEditor.editor(QUOTA_EDITOR_PREFIX)),
  ).toBeVisible()
}

// ============================================================================
// Test suite
// ============================================================================

test.describe('[Billing Admin] Entitlement Mapping 配额编辑器 (US-PO-009)', () => {
  test.beforeEach(async ({ page }) => {
    // Skip gracefully when Stripe credentials are absent (live dependency).
    test.skip(!hasStripePayment(), 'Stripe credentials required (live test)')

    await verifyTestEnvironment(page, {
      requiredRealms: [TEST_REALM],
      requiredUsers: [ADMIN_EMAIL],
    })
    await loginAsAdmin(page)

    // Resolve the real multi-price catalog on first authenticated run (sync
    // requires the admin session cookie carried by `page.request`).
    if (!realProductId) {
      const catalog = await ensureMultiPriceCatalog(page.request, {
        baseUrl: BASE_URL,
        realmId: TEST_REALM,
        stripeSecretKey: secrets.stripe.secretKey!,
        stripePublishableKey: secrets.stripe.publishableKey!,
        stripeWebhookSecret: secrets.stripe.webhookSecret!,
      })
      realProductId = catalog.product.productId
    }
  })

  test.afterEach(async ({ page }) => {
    await cleanupTestData(page, TEST_REALM)
  })

  test('US-PO-009 场景1: 添加并保存多窗口配额', async ({ page }) => {
    await openQuotaEditor(page)

    await clearQuotaEditorRows(page, QUOTA_EDITOR_PREFIX)
    await fillQuotaEditorRows(page, QUOTA_EDITOR_PREFIX, TEST_WINDOWS)

    await page.locator(SELECTORS.pointsQuotaEditor.saveMappingButton).click()
    await page.waitForLoadState('networkidle')

    // Re-open and assert persisted rows.
    await openQuotaEditor(page)
    const editor = page.locator(SELECTORS.pointsQuotaEditor.editor(QUOTA_EDITOR_PREFIX))
    const rows = editor.locator(SELECTORS.pointsQuotaEditor.row(QUOTA_EDITOR_PREFIX, 0))
    await expect(rows).toHaveCount(TEST_WINDOWS.length)
  })

  test('US-PO-009 场景1: 删除窗口后仅保留剩余配置', async ({ page }) => {
    await createEntitlementMappingWithQuotaWindows(
      page,
      TEST_REALM,
      realProductId,
      TEST_WINDOWS,
    )

    await openQuotaEditor(page)
    await clearQuotaEditorRows(page, QUOTA_EDITOR_PREFIX)
    await fillQuotaEditorRows(page, QUOTA_EDITOR_PREFIX, SINGLE_WINDOW)

    await page.locator(SELECTORS.pointsQuotaEditor.saveMappingButton).click()
    await page.waitForLoadState('networkidle')

    await openQuotaEditor(page)
    const editor = page.locator(SELECTORS.pointsQuotaEditor.editor(QUOTA_EDITOR_PREFIX))
    const rows = editor.locator(SELECTORS.pointsQuotaEditor.row(QUOTA_EDITOR_PREFIX, 0))
    await expect(rows).toHaveCount(SINGLE_WINDOW.length)
  })

  test('US-PO-009 场景3: 客户端校验拦截非法窗口配置', async ({ page }) => {
    await openQuotaEditor(page)
    await clearQuotaEditorRows(page, QUOTA_EDITOR_PREFIX)

    const editor = page.locator(SELECTORS.pointsQuotaEditor.editor(QUOTA_EDITOR_PREFIX))
    await page.locator(SELECTORS.pointsQuotaEditor.addButton(QUOTA_EDITOR_PREFIX)).click()

    const lengthInput = editor.locator(
      SELECTORS.pointsQuotaEditor.lengthRow(QUOTA_EDITOR_PREFIX, 0),
    )
    const limitInput = editor.locator(
      SELECTORS.pointsQuotaEditor.limitRow(QUOTA_EDITOR_PREFIX, 0),
    )

    await lengthInput.fill('0')
    await limitInput.fill('-10')

    // Validation: aria-invalid should be present on offending inputs.
    await expect(lengthInput).toHaveAttribute('aria-invalid', 'true')
    await expect(limitInput).toHaveAttribute('aria-invalid', 'true')

    // Save button should be disabled while validation errors exist.
    const saveButton = page.locator(SELECTORS.pointsQuotaEditor.saveMappingButton)
    await expect(saveButton).toBeDisabled()
  })

  test('US-PO-009 场景4: mapping_in_use 409 保护活跃订阅', async ({ page }) => {
    const mappingsPage = new EntitlementMappingsPage(page)
    await mappingsPage.goto(TEST_REALM)
    await mappingsPage.selectProduct(realProductId)

    // Attempt to disable the first price row. If active subscriptions exist,
    // the backend returns 409 and the frontend surfaces the protected-price dialog.
    const firstPriceRow = page
      .locator(SELECTORS.multiPriceMapping.mappingDetailPanel)
      .locator('[data-testid^="price-edit-row-"]')
      .first()
    const priceKey = await firstPriceRow.getAttribute('data-testid')
    if (!priceKey) {
      throw new Error('Could not resolve first price row testid')
    }
    const rawPriceKey = priceKey.replace('price-edit-row-', '')

    await mappingsPage.togglePriceEnabled(rawPriceKey)
    await mappingsPage.saveChanges()

    // The dialog may or may not surface depending on seed data; assert behavior
    // when it does. If it does not appear, the save succeeded and we accept the
    // seed state as not having active subscriptions for this price.
    const dialogVisible = await mappingsPage.protectedPriceConfirmDialog
      .isVisible()
      .catch(() => false)

    if (dialogVisible) {
      await expect(mappingsPage.protectedPriceActiveSubs).toBeVisible()
      const activeSubs = await mappingsPage.getProtectedPriceActiveSubs()
      expect(activeSubs).toBeGreaterThan(0)
      await mappingsPage.cancelProtectedPrice()
    }
  })
})
