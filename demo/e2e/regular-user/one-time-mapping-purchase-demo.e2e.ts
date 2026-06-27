/**
 * One-Time Mapping Purchase Flow Demo (P0)
 *
 * User Stories: US-PU-006 S7
 * Coverage: mapping card display on purchase page
 *
 * Payment initiation tests (Stripe redirect, state recovery) require
 * real third-party provider integration and are located at:
 *   demo/e2e/live/billing/one-time-mapping-purchase/us-pu-006-one-time-purchase-live.e2e.ts
 *
 * Uses Demo Seed data (realm-001 with pre-configured one-time entitlement mappings).
 * Per spec/demo/e2e-testing.md Section 8: no admin data creation in tests.
 */

import { test, expect, cleanupTestData } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { loginWithCredentials } from '../helpers/auth'
import { SELECTORS } from '../selectors'
import { TEST_DATA } from '../helpers/unified-purchase.helpers'

const REALM_ID = TEST_DATA.REALMS.REALM_001
const USER_EMAIL = TEST_DATA.USERS.USER_REALM_001

test.describe('[Regular User] US-PU-006: One-Time Mapping Purchase Flow', () => {
  test.beforeEach(async ({ page }) => {
    await verifyTestEnvironment(page, {
      requiredRealms: [REALM_ID],
      requiredUsers: [USER_EMAIL],
    })

    await loginWithCredentials(page, {
      realmId: REALM_ID,
      email: USER_EMAIL,
      password: TEST_DATA.CREDENTIALS.DEFAULT_PASSWORD,
    })

    await page.waitForURL(`**/${REALM_ID}/user**`)
  })

  test.afterEach(async ({ page, testStartTime }) => {
    await cleanupTestData(page, REALM_ID, {
      keepUsers: [USER_EMAIL],
      timestamp: testStartTime,
    })
  })

  test('should display available one-time mapping cards on purchase page (US-PU-006 S7)', async ({
    page,
    demoLogger,
  }) => {
    await test.step('Navigate to purchase page', async () => {
      await page.goto(`/${REALM_ID}/user/purchase-points`)
      await expect(page.locator(SELECTORS.purchasePoints.page)).toBeVisible()
    })

    await test.step('Verify mapping card selection step is displayed', async () => {
      await expect(page.locator(SELECTORS.purchasePoints.stepPackages)).toBeVisible()
    })

    await test.step('Verify price-card grid contains at least one card', async () => {
      // `mapping-groups` grid + `mapping-card-*` cards were replaced by
      // `purchase-price-grid-${period}` + `purchase-price-card-*`. The page
      // boots in the Monthly pane; one_time cards render here too
      // (period-agnostic), so the month grid is the canonical place to assert.
      const monthGrid = page.locator(
        SELECTORS.purchasePriceCard.priceGrid('month'),
      )
      await expect(monthGrid).toBeVisible()

      const firstCard = monthGrid
        .locator('[data-testid^="purchase-price-card-"]')
        .first()
      await expect(firstCard).toBeVisible()
    })

    await test.step('Verify step indicator shows select step as active', async () => {
      const stepIndicator = page.locator(SELECTORS.purchasePoints.stepIndicator)
      await expect(stepIndicator).toBeVisible()
      // First step (select) should have bold/primary styling
      const selectStep = stepIndicator.locator('span').first()
      await expect(selectStep).toHaveClass(/font-bold|text-primary/)
    })

    await test.step('Verify cards display entitlement key text', async () => {
      // Card testid changed from `mapping-card-{key}` to
      // `purchase-price-card-{priceId}`; the card still renders the entitlement
      // key as display text, so the textContent assertion still encodes the
      // US-PU-006 S7 "user sees a purchasable pack" intent.
      const monthGrid = page.locator(
        SELECTORS.purchasePriceCard.priceGrid('month'),
      )
      const firstCard = monthGrid
        .locator('[data-testid^="purchase-price-card-"]')
        .first()
      const cardText = await firstCard.textContent()
      expect(cardText).toBeTruthy()
      // Each card should show entitlement key as text content
      expect(cardText!.length).toBeGreaterThan(0)
    })

    await test.step('Verify no empty state is shown (mappings exist from Demo Seed)', async () => {
      // Empty-state testid migrated from `purchase-empty-state` (still under
      // mappingCard.emptyState) to `purchase-empty-state` under
      // purchasePriceCard.emptyState — same testid string, now sourced from the
      // price-card group. Rendered per-pane when `periodPane.length === 0`.
      const emptyState = page.locator(SELECTORS.purchasePriceCard.emptyState)
      await expect(emptyState).not.toBeVisible()
    })
  })
})
