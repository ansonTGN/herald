/**
 * Points Quota Demo Helpers
 *
 * User stories covered:
 * - US-PU-010: 滚动窗口额度与充值余额的可用性体验
 * - US-PO-009: 配置多时间窗滚动配额
 * - US-FU-005: 免费周期积分改为滚动窗口配额
 */

import { Page, expect, type Locator } from '@playwright/test'
import { SELECTORS } from '../selectors'
import { makeExtApiRequest } from './ext-api-helper'
import { loginAsAdmin, loginWithCredentials } from './auth'
import { registerUser, POINTS_ROUTES } from './points-helpers'
import { initiatePurchaseFlow } from './unified-purchase.helpers'
import type { QuotaWindowFixture } from '../fixtures/points-quota.fixtures'

export type { QuotaWindowFixture }

export interface ConsumePointsExtApiBody {
  userId: string
  amount: number
  clientAppId: string
  description?: string
  idempotencyKey?: string
}

export interface ConsumePointsResult {
  status: number
  body: unknown
}

/**
 * Clear all rows in a `MultiWindowQuotaEditor`.
 *
 * Repeatedly clicks the first delete button until no data rows remain.
 */
export async function clearQuotaEditorRows(
  page: Page,
  prefix: string,
): Promise<void> {
  const editor = page.locator(SELECTORS.pointsQuotaEditor.editor(prefix))
  await expect(editor).toBeVisible()

  // Defensive: cap iterations at the component MAX_WINDOWS (8) + margin.
  for (let attempts = 0; attempts < 12; attempts += 1) {
    const firstRow = editor.locator(SELECTORS.pointsQuotaEditor.row(prefix, 0))
    if ((await firstRow.count()) === 0) break

    const deleteButton = editor.locator(
      SELECTORS.pointsQuotaEditor.deleteRow(prefix, 0),
    )
    if ((await deleteButton.count()) === 0) break

    await deleteButton.click()
  }
}

/**
 * Fill a `MultiWindowQuotaEditor` with the supplied window configuration.
 *
 * The editor is normalized to `seconds` before each length input is filled,
 * avoiding surprises from the component's display-unit derivation.
 */
export async function fillQuotaEditorRows(
  page: Page,
  prefix: string,
  windows: QuotaWindowFixture[],
): Promise<void> {
  const editor = page.locator(SELECTORS.pointsQuotaEditor.editor(prefix))
  await expect(editor).toBeVisible()

  for (let index = 0; index < windows.length; index += 1) {
    const rowCount = await editor
      .locator(SELECTORS.pointsQuotaEditor.row(prefix, 0))
      .count()

    if (rowCount <= index) {
      await editor.locator(SELECTORS.pointsQuotaEditor.addButton(prefix)).click()
    }

    const lengthInput = editor.locator(
      SELECTORS.pointsQuotaEditor.lengthRow(prefix, index),
    )
    const unitTrigger = editor.locator(
      SELECTORS.pointsQuotaEditor.unitRow(prefix, index),
    )
    const limitInput = editor.locator(
      SELECTORS.pointsQuotaEditor.limitRow(prefix, index),
    )

    await expect(lengthInput).toBeVisible()
    await expect(unitTrigger).toBeVisible()
    await expect(limitInput).toBeVisible()

    // Normalize to seconds so callers can pass raw windowSeconds.
    await unitTrigger.click()
    await page.getByRole('option', { name: 'seconds' }).click()

    await lengthInput.fill(windows[index].windowSeconds.toString())
    await limitInput.fill(windows[index].limit.toString())
  }
}

/**
 * Fill the required non-quota fields on the realm default config form.
 *
 * The quota editor is embedded in a larger form; saving quota rows is gated by
 * these required fields as well as the row values.
 */
export async function fillRealmDefaultRequiredFields(page: Page): Promise<void> {
  await page.locator(SELECTORS.points.freePeriodicPointsAmountInput).fill('50')
  await page.locator(SELECTORS.points.freePeriodicValidityDaysInput).fill('1')

  const periodSelect = page.locator(SELECTORS.points.freePeriodicGrantPeriodTypeSelect)
  await periodSelect.click()
  await page.getByRole('option', { name: /daily/i }).click()
}

/**
 * Create (or overwrite) multi-window quota configuration on an entitlement
 * mapping.
 */
export async function createEntitlementMappingWithQuotaWindows(
  page: Page,
  realmId: string,
  productHint: string,
  windows: QuotaWindowFixture[],
): Promise<void> {
  await loginAsAdmin(page, { realmId, waitNavigation: true })
  await page.goto(`/manage/billing/entitlement-mappings`)
  await expect(page.locator(SELECTORS.multiPriceMapping.page)).toBeVisible()

  const productRow = page.locator(
    SELECTORS.multiPriceMapping.mappingProductRow(productHint),
  )
  await expect(productRow).toBeVisible()
  await productRow.click()
  await expect(page.locator(SELECTORS.multiPriceMapping.mappingDetailPanel)).toBeVisible()

  // The quota editor lives inside each price row's Advanced collapsible.
  const firstAdvanced = page
    .locator(SELECTORS.multiPriceMapping.mappingDetailPanel)
    .getByRole('button', { name: 'Advanced' })
    .first()
  await expect(firstAdvanced).toBeVisible()
  await firstAdvanced.click()

  const prefix = 'quota-window'
  await expect(page.locator(SELECTORS.pointsQuotaEditor.editor(prefix))).toBeVisible()

  await clearQuotaEditorRows(page, prefix)
  await fillQuotaEditorRows(page, prefix, windows)

  await page.locator(SELECTORS.pointsQuotaEditor.saveMappingButton).click()
  await page.waitForLoadState('networkidle')
}

/** Set the realm default free-periodic quota windows. */
export async function setRealmDefaultFreePeriodicQuota(
  page: Page,
  realmId: string,
  windows: QuotaWindowFixture[],
): Promise<void> {
  await loginAsAdmin(page, { realmId, waitNavigation: true })
  await page.goto(`/manage/points/default-config`)
  await expect(page.locator('[data-testid="points-default-config-form"]')).toBeVisible()

  const prefix = 'realm-default-window'
  await expect(page.locator(SELECTORS.pointsQuotaEditor.editor(prefix))).toBeVisible()

  await fillRealmDefaultRequiredFields(page)
  await clearQuotaEditorRows(page, prefix)
  await fillQuotaEditorRows(page, prefix, windows)

  await page.locator(SELECTORS.pointsQuotaEditor.saveConfigButton).click()
  await page.waitForLoadState('networkidle')
}

/**
 * Purchase a subscription that is configured with quota windows.
 *
 * This is a thin wrapper around the unified purchase flow. It returns the
 * payment attempt id and waits for the completion step to surface.
 *
 * @param providerHint Payment provider name used by the purchase page
 *                     (e.g. `'stripe'` or `'creem'`).
 */
export async function purchaseSubscriptionToGetQuota(
  page: Page,
  realmId: string,
  userEmail: string,
  password: string,
  providerHint: string,
): Promise<string> {
  await loginWithCredentials(page, { realmId, email: userEmail, password })
  await page.evaluate(() => localStorage.removeItem('cas-purchase-flow'))

  const attemptId = await initiatePurchaseFlow(page, providerHint as 'stripe' | 'creem', realmId)

  // Wait for either the completion step or a provider redirect prompt.
  await expect(
    page.locator(SELECTORS.purchasePoints.stepComplete)
      .or(page.locator(SELECTORS.paymentProviderUI.redirectPrompt))
      .or(page.locator(SELECTORS.paymentProviderUI.contextDegraded)),
  ).toBeVisible({ timeout: 15000 })

  return attemptId
}

/**
 * Consume points through the external API.
 *
 * Thin wrapper around `makeExtApiRequest` for
 * `POST /api/ext/points/{realmId}/consume`.
 */
export async function consumePointsViaExtApi(
  apiKey: string,
  realmId: string,
  body: ConsumePointsExtApiBody,
): Promise<ConsumePointsResult> {
  const { status, body: responseBody } = await makeExtApiRequest({
    apiKey,
    method: 'POST',
    path: `/points/${realmId}/consume`,
    body,
  })

  return { status, body: responseBody }
}

/** Register a new user and assert the realm-default quota windows render. */
export async function registerNewUserWithRealmDefaultQuota(
  page: Page,
  realmId: string,
  email: string,
  password: string = 'password123',
): Promise<void> {
  await page.context().clearCookies()
  await page.evaluate(() => {
    localStorage.clear()
    sessionStorage.clear()
  })
  await registerUser(page, realmId, email, password)
  await loginWithCredentials(page, { realmId, email, password })

  await page.goto(POINTS_ROUTES.USER_POINTS(realmId))
  await expect(page.locator(SELECTORS.pointsUser.page)).toBeVisible()
  await expect(
    page.locator(SELECTORS.pointsUsageDashboard.page),
  ).toBeVisible({ timeout: 15000 })

  // Assert at least one window row exists for each configured window without
  // requiring callers to know the bucket UUID up front.
  const windowRows = page.locator('[data-testid^="points-window-row-"]')
  await expect(windowRows).toHaveCount(2)
}

export function getWindowRow(
  page: Page,
  bucketId: string,
  winKey: string,
): Locator {
  return page.locator(SELECTORS.pointsUsageDashboard.windowRow(bucketId, winKey))
}

/**
 * Read the "remaining" value from a window row.
 *
 * The row renders `remaining / limit · used`; this returns the first integer.
 */
export async function getWindowRemaining(
  page: Page,
  bucketId: string,
  winKey: string,
): Promise<number> {
  const row = getWindowRow(page, bucketId, winKey)
  await expect(row).toBeVisible()
  const text = (await row.textContent()) || ''
  const match = text.match(/([\d,]+)\s*\//)
  return parseAmount(match?.[1])
}

/**
 * Read the resets-in copy from a window row.
 *
 * The dedicated testid is not emitted; the copy is the last text span inside the row.
 */
export async function getWindowResetsIn(
  page: Page,
  bucketId: string,
  winKey: string,
): Promise<string> {
  const row = getWindowRow(page, bucketId, winKey)
  await expect(row).toBeVisible()
  const spans = row.locator('span')
  const count = await spans.count()
  if (count === 0) return ''
  const text = (await spans.last().textContent()) || ''
  return text.trim()
}

export async function getSpendableNow(page: Page): Promise<number> {
  const el = page.locator(SELECTORS.pointsUsageDashboard.spendableNow)
  await expect(el).toBeVisible()
  const text = (await el.textContent()) || ''
  return parseAmount(text)
}

/**
 * Read the demo user's `spendable_from_pool` (topup + registration + granted
 * balances) for a bucket directly from the wallets API.
 *
 * Used by the total-formula test to assert `spendableNow === smallestRemaining
 * + pool` without hard-coding the pool value, which accumulates across demo
 * runs because the ext grant API has no idempotency key.
 */
export async function getSpendableFromPool(
  page: Page,
  realmId: string,
  bucketId: string,
): Promise<number> {
  const baseUrl =
    process.env.API_BASE_URL ||
    process.env.BASE_URL?.replace(/:\d+/, ':8080') ||
    'http://localhost:8080'
  const resp = await page.context().request.get(
    `${baseUrl}/api/points/${realmId}/wallets`,
  )
  if (!resp.ok()) return 0
  const body = await resp.json()
  const items = (body?.items ?? []) as {
    bucketId?: string
    spendableFromPool?: number | null
  }[]
  const match = items.find((i) => i.bucketId === bucketId)
  return match?.spendableFromPool ?? 0
}

function parseAmount(text: string | undefined | null): number {
  if (!text) return 0
  const cleaned = text.replace(/[^\d-]/g, '')
  const n = parseInt(cleaned, 10)
  return Number.isNaN(n) ? 0 : n
}
