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
 * Period semantics (load-bearing — mirrors `multi-price-purchase.helpers.ts`):
 * `selectPeriodPane` renders `recurring` items ONLY in the pane whose
 * `billingPeriod` matches, and renders `one_time` items in BOTH panes
 * (period-agnostic). A one-time card therefore resolves identically under
 * either period; a recurring month card has NO Annual counterpart. This helper
 * defaults to the Monthly pane so it works for both one-time and month-recurring
 * products without callers pinning a period.
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
 * Discover the first purchasable price card on the active period pane and
 * return its (priceId, period) tuple so the caller can target the exact DOM
 * node via `selectPriceCard`.
 *
 * The rewritten page renders price cards as
 * `purchase-price-card-${priceId}` (Monthly) or
 * `purchase-price-card-${priceId}-annual` (Annual). There is no enumerating
 * grid child testid, so we read the testid attribute off the first card in the
 * active grid and reverse-engineer the priceId + period.
 *
 * Disabled cards (mapping disabled or no provider wired) render the SAME card
 * testid plus a sibling `-reason` row; they are skipped here so the helper
 * never attempts to click a card whose `onClick` is undefined.
 *
 * Default period is `'month'`: the page boots in Monthly, and `one_time` cards
 * appear in both panes so a single-price/one-time product resolves under
 * either. Callers with a known annual-recurring product should pass `'year'`.
 */
async function discoverFirstPurchasablePriceCard(
  page: Page,
  period: PurchasePeriod = 'month',
): Promise<{ priceId: string; period: PurchasePeriod }> {
  const grid = page.locator(SELECTORS.purchasePriceCard.priceGrid(period))
  await expect(grid).toBeVisible({
    timeout: TEST_DATA.TIMEOUTS.ELEMENT_VISIBLE,
  })

  // Enumerate the card testids in the active pane and pick the first that has
  // no `-reason` sibling (i.e. is purchasable). Cards share the testid prefix
  // `purchase-price-card-`; the grid only contains cards, so children are all
  // cards.
  const cards = grid.locator('[data-testid^="purchase-price-card-"]')
  const count = await cards.count()
  if (count === 0) {
    throw new Error(
      `No price cards rendered in the ${period} pane — purchase page seed empty?`,
    )
  }

  for (let i = 0; i < count; i++) {
    const card = cards.nth(i)
    const testid = (await card.getAttribute('data-testid')) ?? ''
    // Skip reason rows just in case (they live as children of cards, not the
    // grid, so this is defensive).
    if (testid.endsWith('-reason')) continue

    // Recover priceId + detect annual suffix.
    // testid shape: `purchase-price-card-<priceId>` or
    // `purchase-price-card-<priceId>-annual`.
    const stripped = testid.replace(/^purchase-price-card-/, '')
    if (stripped.endsWith('-annual')) {
      const priceId = stripped.slice(0, -'-annual'.length)
      // Annual-pane cards are only recurring-year; respect the requested period.
      if (period === 'year') {
        return { priceId, period: 'year' }
      }
      continue
    }
    return { priceId: stripped, period }
  }

  throw new Error(
    `No purchasable price card found in the ${period} pane (all cards disabled?).`,
  )
}

/**
 * Selects the first available price card and proceeds to the payment step.
 *
 * Replaces the legacy `mappingCard.firstCard()` click. Defaults to the Monthly
 * pane so both one-time (period-agnostic) and month-recurring products resolve.
 * Pass `period: 'year'` for annual-recurring products.
 *
 * Internals:
 * - mapping-card testid → `purchase-price-card-${priceId}` (Monthly) /
 *   `purchase-price-card-${priceId}-annual` (Annual).
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
 * (Monthly) or `purchase-price-card-${priceId}-annual` (Annual).
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
  await page.goto(`/${realmId}/user/purchase-points`)
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
