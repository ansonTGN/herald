/**
 * Unified Purchase Test Helpers
 *
 * Reusable helper functions for the user-domain purchase flow.
 *
 * The purchase-page rewrite replaced the entitlement-key-grouped
 * `mapping-card-*` cards with a price-card grid + period toggle
 * (`purchase-price-card-*`). The exported names importers rely on are
 * PRESERVED (`selectFirstMappingAndProceed`, `selectMappingAndProceed`,
 * `TEST_DATA`, `extractPaymentAttemptId`, `selectPaymentMethodAndProceed`,
 * `initiatePurchaseFlow`, `verifyRedirectPromptOrDegraded`); only the
 * card-selection internals switch to the price-card grid.
 *
 * Purchase flow (rewritten page):
 * 1. User sees price cards on /{realm}/user/purchase-points (Monthly pane by
 *    default; Annual pane via period toggle).
 * 2. Selects a price card (click sets `selectedMappingId = option.mappingId`).
 * 3. Clicks Next to proceed to the payment step.
 * 4. Selects payment method and clicks Complete Purchase.
 * 5. `createPaymentAttempt` POSTs `{targetType:'entitlement_mapping',
 *    targetId:<selectedMappingId>, paymentProvider:<derived from option>}`.
 *
 * Section IA (load-bearing — mirrors `multi-price-purchase.helpers.ts`):
 * The page splits options into two sections by billing type. Recurring options
 * live in the Subscriptions section under `purchase-price-grid-${period}`;
 * one_time options live in the Credit packs section under
 * `purchase-price-grid-credit-packs` (NOT a period-agnostic duplicate). The
 * price-card testid is period-invariant (`purchase-price-card-${priceId}`,
 * no `-annual` suffix). This helper searches BOTH grids so a single-price /
 * one-time product resolves regardless of the requested period.
 *
 * Boundary:
 * - Uses ONLY `SELECTORS.purchasePriceCard.*` + `SELECTORS.purchasePoints.*` +
 *   `SELECTORS.paymentMethodSelector.*` + `SELECTORS.paymentProviderUI.*`.
 * - Does NOT hardcode selector strings (every locator flows from selectors.ts).
 * - Re-exports the period-toggle helper from `multi-price-purchase.helpers.ts`
 *   to avoid duplicating the period-switch mechanics.
 */

import { expect, type Page } from '@playwright/test'
import { SELECTORS } from '../selectors'
import {
  selectPeriod,
  selectPriceCard,
  type PurchasePeriod,
} from './multi-price-purchase.helpers'

export type { PurchasePeriod }

// Re-export so callers that need pane-level mechanics can reach the canonical
// helpers without a second import line.
export { selectPeriod, selectPriceCard }

// Test constants
export const TEST_DATA = {
  REALMS: {
    REALM_001: 'realm-001',
  },
  USERS: {
    ADMIN_REALM_001: 'admin@realm-001.com',
    USER_REALM_001: 'user@realm-001.com',
  },
  CREDENTIALS: {
    DEFAULT_PASSWORD: 'password',
  },
  TIMEOUTS: {
    DEFAULT_NAVIGATION: 5000,
    PAYMENT_POLLING: 15000,
    ATTEMPT_CREATION: 2000,
    ELEMENT_VISIBLE: 10000,
  },
  PAYMENT_PROVIDERS: {
    STRIPE: 'stripe',
    CREEM: 'creem',
  } as const,
} as const

export type PaymentProvider =
  (typeof TEST_DATA.PAYMENT_PROVIDERS)[keyof typeof TEST_DATA.PAYMENT_PROVIDERS]

/**
 * Extracts payment attempt ID from localStorage
 */
export async function extractPaymentAttemptId(page: Page): Promise<string> {
  await page.waitForTimeout(TEST_DATA.TIMEOUTS.ATTEMPT_CREATION)

  const attemptId = await page.evaluate(() => {
    const state = localStorage.getItem('cas-purchase-flow')
    if (state) {
      const parsed = JSON.parse(state)
      return parsed?.state?.attemptId
    }
    return null
  })

  if (!attemptId) {
    throw new Error('Payment attempt ID not found in localStorage')
  }

  return attemptId
}

/**
 * Discover the first purchasable price card on the page and return its
 * (priceId, period) tuple so the caller can target the exact DOM node via
 * `selectPriceCard`.
 *
 * Under the section IA the page renders two grids: the Subscriptions-section
 * grid (`purchase-price-grid-${period}`, recurring only) and the Credit-packs
 * grid (`purchase-price-grid-credit-packs`, one_time only). The card testid is
 * period-invariant (`purchase-price-card-${priceId}`, no `-annual` suffix).
 *
 * We search the Subscriptions grid for the requested period first; if it has
 * no purchasable card we fall back to the Credit-packs grid. This makes a
 * single-price / one-time product resolve under the default `'month'` without
 * callers pinning a section.
 *
 * Disabled cards (mapping disabled or no provider wired) render the SAME card
 * testid plus a child `-reason` row; they are skipped here so the helper never
 * attempts to click a card whose `onClick` is undefined.
 *
 * Default period is `'month'` (page boots in Monthly). Callers with a known
 * annual-recurring product should pass `'year'`.
 */
async function discoverFirstPurchasablePriceCard(
  page: Page,
  period: PurchasePeriod = 'month',
): Promise<{ priceId: string; period: PurchasePeriod }> {
  // Candidate grids in priority order: the requested-period Subscriptions
  // grid, then the Credit-packs grid (one_time). A grid may be absent (e.g.
  // no recurring options → Subscriptions section not rendered).
  const candidateGrids = [
    SELECTORS.purchasePriceCard.priceGrid(period),
    SELECTORS.purchasePriceCard.creditPacksGrid,
  ]

  for (const gridSelector of candidateGrids) {
    const grid = page.locator(gridSelector)
    // Skip grids that are not rendered (section hidden) rather than asserting
    // visibility — the requested period's Subscriptions grid is legitimately
    // absent for a one-time-only product.
    if (!(await grid.isVisible().catch(() => false))) continue

    const cards = grid.locator('[data-testid^="purchase-price-card-"]')
    const count = await cards.count()
    for (let i = 0; i < count; i++) {
      const card = cards.nth(i)
      const testid = (await card.getAttribute('data-testid')) ?? ''
      // Skip reason rows (defensive — they live as children of cards, not the
      // grid).
      if (testid.endsWith('-reason')) continue
      const priceId = testid.replace(/^purchase-price-card-/, '')
      // Confirm the card is not disabled (no `-reason` child row) before
      // returning it.
      const reason = card.locator(`[data-testid="${testid}-reason"]`)
      if ((await reason.count()) > 0) continue
      return { priceId, period }
    }
  }

  throw new Error(
    `No purchasable price card found (Subscriptions ${period} + Credit packs; all disabled or seed empty?).`,
  )
}

/**
 * Selects the first available price card and proceeds to the payment step.
 *
 * Replaces the legacy `mappingCard.firstCard()` click. Defaults to the Monthly
 * period so both one-time (Credit packs section) and month-recurring
 * (Subscriptions section) products resolve. Pass `period: 'year'` for
 * annual-recurring products.
 *
 * Internals:
 * - card testid → `purchase-price-card-${priceId}` (period-invariant; no
 *   `-annual` suffix under the section IA).
 * - The page derives the checkout `mappingId` itself from the clicked option
 *   (`selectedMappingId = option.mappingId`), so this helper does NOT need to
 *   resolve mappingId — the click is the load-bearing act.
 */
export async function selectFirstMappingAndProceed(
  page: Page,
  opts?: { period?: PurchasePeriod },
): Promise<void> {
  const period = opts?.period ?? 'month'
  const { priceId, period: resolvedPeriod } =
    await discoverFirstPurchasablePriceCard(page, period)
  await selectPriceCard(page, priceId, resolvedPeriod)

  // Verify the card shows selected state and Next is enabled.
  await expect(
    page.locator(SELECTORS.purchasePriceCard.nextButton),
  ).toBeEnabled()

  await page.locator(SELECTORS.purchasePriceCard.nextButton).click()
}

/**
 * Selects a specific price card by priceId and proceeds to the payment step.
 *
 * No current importer calls this; it is preserved for parity and any future
 * price-pinned caller. `priceId` is `externalPriceId ?? mappingId` (Creem
 * NULL-price rows use mappingId); targets `purchase-price-card-${priceId}`
 * (period-invariant under the section IA).
 */
export async function selectMappingAndProceed(
  page: Page,
  priceId: string,
  period?: PurchasePeriod,
): Promise<void> {
  await selectPriceCard(page, priceId, period)

  await expect(
    page.locator(SELECTORS.purchasePriceCard.nextButton),
  ).toBeEnabled()
  await page.locator(SELECTORS.purchasePriceCard.nextButton).click()
}

/**
 * Selects payment method and proceeds to processing
 */
export async function selectPaymentMethodAndProceed(
  page: Page,
  provider: PaymentProvider
): Promise<void> {
  await page.getByTestId(`payment-method-select-${provider}`).click()
  await expect(page.getByTestId(`payment-method-selected-${provider}`)).toBeVisible()

  await expect(page.locator(SELECTORS.purchasePoints.nextButton)).toBeEnabled()
  await page.locator(SELECTORS.purchasePoints.nextButton).click()
}

/**
 * Full purchase flow: navigate -> select first price card -> select provider ->
 * complete purchase. Returns the payment attempt ID from localStorage.
 *
 * The card-selection step is price-card driven via
 * `selectFirstMappingAndProceed` (Monthly pane by default; pass `period:'year'`
 * for annual-recurring products). The checkout `mappingId` is resolved by the
 * page from the clicked option; this helper does not pin it.
 */
export async function initiatePurchaseFlow(
  page: Page,
  provider: PaymentProvider,
  realmId: string = TEST_DATA.REALMS.REALM_001,
  opts?: { period?: PurchasePeriod }
): Promise<string> {
  await page.evaluate(() => localStorage.removeItem('cas-purchase-flow'))
  await page.goto(`/user/purchase-points`)
  await expect(page.locator(SELECTORS.purchasePoints.page)).toBeVisible()

  // Precondition: realm must have at least one purchasable price card in the
  // target period pane. If none exist, the page shows the empty state and this
  // helper will fail. Callers in conditional test contexts should check for
  // price cards first.
  await selectFirstMappingAndProceed(page, opts)
  await expect(page.locator(SELECTORS.purchasePoints.stepPayment)).toBeVisible()

  await selectPaymentMethodAndProceed(page, provider)

  await expect(page.locator(SELECTORS.purchasePoints.stepProcessing)).toBeVisible({
    timeout: TEST_DATA.TIMEOUTS.ELEMENT_VISIBLE,
  })

  return extractPaymentAttemptId(page)
}

/**
 * Verifies redirect prompt or degraded UI for a payment provider.
 * Handles two cases: checkout URL present (redirect prompt) or absent (degraded UI).
 * Verifies the redirect prompt (with checkout URL) or degraded UI is shown.
 */
export async function verifyRedirectPromptOrDegraded(
  page: Page,
  providerName: string
): Promise<void> {
  await expect(
    page
      .locator(SELECTORS.paymentProviderUI.redirectPrompt)
      .or(page.locator(SELECTORS.paymentProviderUI.contextDegraded))
  ).toBeVisible({ timeout: 5000 })

  const redirectPrompt = page.locator(SELECTORS.paymentProviderUI.redirectPrompt)
  const isRedirectPromptVisible = await redirectPrompt.isVisible()

  if (isRedirectPromptVisible) {
    const manualLink = page.locator(SELECTORS.paymentProviderUI.redirectManualLink)
    await expect(manualLink).toBeVisible()

    const href = await manualLink.getAttribute('href')
    expect(href).toBeTruthy()

    const promptText = await redirectPrompt.textContent()
    expect(promptText).toMatch(new RegExp(`${providerName}|Redirecting`, 'i'))
    // Do not cancel here: the redirect is user-initiated (no auto-redirect), and
    // cancelling would clearPurchaseState() and wipe the persisted attemptId that
    // the caller verifies afterwards. The pending attempt is cleaned up in afterEach.
  } else {
    const degraded = page.locator(SELECTORS.paymentProviderUI.contextDegraded)
    await expect(degraded).toBeVisible()
    await expect(degraded).toContainText('Payment Information Unavailable')

    const cancelButton = page.locator(SELECTORS.paymentProviderUI.cancelButton)
    await expect(cancelButton).toBeVisible()
  }
}
