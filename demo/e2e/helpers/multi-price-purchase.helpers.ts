/**
 * Multi-Price Purchase Helpers (support-multiple-price)
 *
 * Self-contained, assertion-light helpers for the rewritten price-card purchase
 * flow (frontend/src/routes/$realmId/user/purchase-points.tsx). These helpers
 * are ADDITIVE infra — NO `.e2e.ts` flow lives here, and these helpers carry NO
 * business assertions (callers own the user-story assertions; this module only
 * encapsulates the selection + checkout-initiation mechanics so those items stay
 * short).
 *
 * Boundary (NON-NEGOTIABLE):
 * - Uses ONLY `SELECTORS.purchasePriceCard.*` + `SELECTORS.stripeCheckoutButton`.
 * - Does NOT import the entitlement-mappings POM — the purchase page and the
 *   admin master-detail page are separate routes.
 * - Does NOT hardcode selector strings — every locator flows from `selectors.ts`.
 *
 * Section IA (purchase-entry-optimization, load-bearing — current frontend):
 * The page renders two sections by billing type and there is NO period toggle.
 * **Subscriptions section** holds ALL recurring options (both monthly and
 * annual) together under a single grid `purchase-price-grid-subscriptions`.
 * **Credit packs section** holds one_time options under
 * `purchase-price-grid-credit-packs`. The price-card testid is
 * period-invariant (`purchase-price-card-${priceId}`, no `-annual` suffix), so
 * `selectPriceCard` ignores the `period` argument (kept only for call-site
 * compatibility) and selecting an annual recurring card is a direct click — no
 * period switch is needed. `selectPeriod` is therefore a no-op that only waits
 * for the Subscriptions grid to attach (kept for call-site compatibility).
 * Callers that need to scope by section should use
 * `SELECTORS.purchasePriceCard.subscriptionsGrid` / `.creditPacksGrid`
 * (`priceGrid(period)` still resolves to the Subscriptions grid).
 */

import { expect, type Locator, type Page, type Response } from '@playwright/test'
import { SELECTORS } from '../selectors'

/**
 * Active billing period on the purchase page. Mirrors the frontend
 * `BillingPeriod` type (`'month' | 'year'`).
 */
export type PurchasePeriod = 'month' | 'year'

/**
 * Wait for the Subscriptions grid to mount.
 *
 * The current frontend purchase page has NO period toggle: all recurring
 * options (monthly + annual) render together under a single grid
 * (`purchase-price-grid-subscriptions`). This function is therefore a no-op
 * with respect to the former toggle click — it only waits for that grid to
 * attach so a subsequent price-card click does not race the render. The
 * `period` argument is accepted (and ignored) for call-site compatibility.
 */
export async function selectPeriod(
  page: Page,
  _period: PurchasePeriod,
): Promise<void> {
  await page
    .locator(SELECTORS.purchasePriceCard.subscriptionsGrid)
    .waitFor({ state: 'attached' })
}

/**
 * Click a price card to select it. The `priceId` is the card's
 * `externalPriceId ?? mappingId` (Creem NULL-price rows use mappingId).
 *
 * The `period` argument is accepted for call-site compatibility but no longer
 * affects the testid — the card testid is period-invariant
 * (`purchase-price-card-${priceId}`). The same priceId never renders in two
 * grids, so scoping is unnecessary for a unique click.
 */
export async function selectPriceCard(
  page: Page,
  priceId: string,
  _period?: PurchasePeriod,
): Promise<void> {
  await page.locator(SELECTORS.purchasePriceCard.priceCard(priceId)).click()
}

/**
 * Build a locator for the disabled-reason row of a price card. The reason row
 * is only rendered when the card is NOT purchasable (mapping disabled or no
 * payment provider wired). Returns a Locator so the caller can choose
 * `expect(...).toBeVisible()` (disabled) or `.toBeHidden()` (purchasable).
 *
 * `period` is accepted for compatibility but ignored (card testid is
 * period-invariant under the section IA).
 */
export function priceCardReasonLocator(
  page: Page,
  priceId: string,
  _period?: PurchasePeriod,
): Locator {
  return page.locator(SELECTORS.purchasePriceCard.priceCardReason(priceId))
}

/**
 * Assert a price card is rendered as disabled (not purchasable). Verifies BOTH
 * the disabled visual cue (`opacity-60` is applied via the card's class) AND
 * the reason row is present — the reason row is the persistent, stable signal
 * (the card has no `aria-disabled`; the reason testid is the load-bearing
 * marker). Does NOT rely on auto-dismissing toasts.
 *
 * `period` is accepted for compatibility but ignored (card testid is
 * period-invariant under the section IA).
 */
export async function expectDisabledPriceCard(
  page: Page,
  priceId: string,
  _period?: PurchasePeriod,
): Promise<void> {
  await expect(
    page.locator(SELECTORS.purchasePriceCard.priceCardReason(priceId)),
  ).toBeVisible()
}

/**
 * Initiate a multi-price checkout from the packages step.
 *
 * Flow:
 * 1. Ensure a price card is already selected (caller selects via
 *    `selectPriceCard` first — this helper does NOT pick a card, it drives the
 *    Next → (payment?) → submit sequence).
 * 2. Register a `waitForResponse` against the create-payment-attempt POST
 *    BEFORE clicking Next, so the response is captured regardless of whether
 *    the payment step is shown.
 * 3. Click Next (packages → payment OR packages → submit).
 *
 * Payment-step-skip branch (current frontend `handleNextStep`): when the
 * selected price's `paymentProvider` resolves to at most one matching provider,
 * the dedicated "Select Payment Method" step is redundant and the frontend
 * calls `createPaymentAttempt` directly from the packages step. In that case
 * there is NO `purchase-step-payment` to wait for — the single Next click IS
 * the submit. When more than one provider matches, the payment step renders and
 * a second Next click submits. This helper handles both branches by waiting
 * for EITHER the payment step to mount (then clicking Next again) OR the
 * checkout response to arrive (skip branch, already submitted).
 *
 * Returns the API `Response` for the checkout POST so the caller can assert on
 * status / body shape (callers own those business assertions). The helper does
 * NOT assert on the response — it only captures it.
 *
 * NOTE: the mappingId is passed for response-matching context only; the actual
 * `targetId` is whatever the page holds in `selectedMappingId` after
 * `selectPriceCard`. Callers that need to pin a specific mapping should select
 * its price card first.
 */
export async function initiateMultiPriceCheckout(
  page: Page,
  _options: { mappingId: string; paymentProvider: string },
): Promise<Response> {
  // Register the response listener BEFORE clicking Next so the checkout POST is
  // captured whether or not the payment step is shown (the skip branch submits
  // directly from the packages step on the first Next click).
  const checkoutResponsePromise = page.waitForResponse(
    (resp) =>
      resp.url().includes('/payment-attempts') &&
      resp.request().method() === 'POST',
  )

  // Packages → payment (or packages → submit on the skip branch).
  await page.locator(SELECTORS.purchasePriceCard.nextButton).click()

  // If the payment step mounts, a second Next click is the submit. If the
  // payment step is skipped, the first Next already submitted and the response
  // is in flight — racing the payment-step selector against the response wait
  // resolves either branch without a fixed timeout.
  const paymentStep = page.locator('[data-testid="purchase-step-payment"]')
  const steppedToPayment = await Promise.race([
    paymentStep
      .waitFor({ state: 'visible' })
      .then(() => true)
      .catch(() => false),
    checkoutResponsePromise.then(() => false),
  ])

  if (steppedToPayment) {
    // Payment → submit (createPaymentAttempt mutation fires here).
    await page.locator(SELECTORS.purchasePriceCard.nextButton).click()
  }
  return checkoutResponsePromise
}
