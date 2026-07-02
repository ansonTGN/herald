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
 * Section IA (purchase-entry-optimization, load-bearing):
 * The page renders two sections by billing type. **Subscriptions section**
 * holds recurring options under a period-aware grid
 * (`purchase-price-grid-${period}`, toggle lives inside this section and is
 * hidden when no recurring options exist). **Credit packs section** holds
 * one_time options under `purchase-price-grid-credit-packs`; one_time cards
 * are NOT period-agnostic duplicates — they live only there. The price-card
 * testid is period-invariant (`purchase-price-card-${priceId}`, no `-annual`
 * suffix), so `selectPriceCard` ignores the `period` argument (kept only for
 * call-site compatibility). Callers that need to scope by section should use
 * `SELECTORS.purchasePriceCard.priceGrid(period)` / `.creditPacksGrid`.
 */

import { expect, type Locator, type Page, type Response } from '@playwright/test'
import { SELECTORS } from '../selectors'

/**
 * Active billing period on the purchase page. Mirrors the frontend
 * `BillingPeriod` type (`'month' | 'year'`).
 */
export type PurchasePeriod = 'month' | 'year'

/**
 * Switch the period toggle and wait for the corresponding pane grid to mount.
 *
 * The toggle lives inside the Subscriptions section and is hidden when no
 * recurring options exist; switching re-renders that section's grid
 * (`purchase-price-grid-${period}`). Waiting on the target grid (rather than a
 * fixed timeout) makes the switch deterministic.
 */
export async function selectPeriod(
  page: Page,
  period: PurchasePeriod,
): Promise<void> {
  const toggle =
    period === 'month'
      ? page.locator(SELECTORS.purchasePriceCard.periodToggleMonth)
      : page.locator(SELECTORS.purchasePriceCard.periodToggleYear)
  await toggle.click()
  // The grid testid is keyed by the active period; wait for it to attach so a
  // subsequent price-card click does not race the re-render.
  await page
    .locator(SELECTORS.purchasePriceCard.priceGrid(period))
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
 *    Next → payment → submit sequence).
 * 2. Click Next (packages → payment).
 * 3. Click Next again on the payment step. The frontend `createPaymentAttempt`
 *    mutation POSTs `{targetType:'entitlement_mapping',
 *    targetId:<selectedMappingId>, paymentProvider:<derived from option>}` —
 *    equivalent to the documented `{mappingId, paymentProvider}` contract.
 *
 * Returns the API `Response` for the checkout POST so the caller can assert on
 * status / body shape (callers own those business assertions). Captures via
 * `page.waitForResponse` against the create-payment-attempt endpoint so the
 * helper does not depend on the post-submit UI state (which may transition to
 * processing/complete asynchronously).
 *
 * The helper does NOT assert on the response — it only captures it. Business
 * assertions (status, redirect URL, payment context) belong to the consuming
 * test so this stays assertion-light per the module boundary.
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
  // Packages → payment.
  await page.locator(SELECTORS.purchasePriceCard.nextButton).click()
  // Wait for the payment step container so the second Next is the submit, not a
  // re-entry into the packages step.
  await page.waitForSelector('[data-testid="purchase-step-payment"]')

  const checkoutResponsePromise = page.waitForResponse(
    (resp) =>
      resp.url().includes('/payment-attempts') &&
      resp.request().method() === 'POST',
  )
  // Payment → submit (createPaymentAttempt mutation fires here).
  await page.locator(SELECTORS.purchasePriceCard.nextButton).click()
  return checkoutResponsePromise
}
