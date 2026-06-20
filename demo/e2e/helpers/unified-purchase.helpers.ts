/**
 * Unified Purchase Test Helpers
 *
 * Reusable helper functions for one-time mapping purchase flow tests.
 * The purchase flow is:
 * 1. User sees mapping cards on purchase-points page
 * 2. Selects a mapping card (click sets selectedMappingId)
 * 3. Clicks Next to proceed to payment step
 * 4. Selects payment method and clicks Complete Purchase
 * 5. Payment attempt is created with targetType 'entitlement_mapping'
 */

import { expect, type Page } from '@playwright/test'
import { SELECTORS } from '../selectors'

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
 * Selects the first available mapping card and proceeds to payment step.
 * Uses SELECTORS.mappingCard selectors from selectors.ts.
 */
export async function selectFirstMappingAndProceed(page: Page): Promise<void> {
  const firstCard = page.locator(SELECTORS.mappingCard.firstCard()).first()
  await expect(firstCard).toBeVisible()
  await firstCard.click()

  // Verify the card shows selected state (ring-2 ring-primary)
  await expect(page.locator(SELECTORS.purchasePoints.nextButton)).toBeEnabled()

  await page.locator(SELECTORS.purchasePoints.nextButton).click()
}

/**
 * Selects a specific mapping card by entitlement key and proceeds to payment step.
 */
export async function selectMappingAndProceed(
  page: Page,
  entitlementKey: string
): Promise<void> {
  const card = page.locator(SELECTORS.mappingCard.card(entitlementKey))
  await expect(card).toBeVisible()
  await card.click()

  await expect(page.locator(SELECTORS.purchasePoints.nextButton)).toBeEnabled()
  await page.locator(SELECTORS.purchasePoints.nextButton).click()
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
 * Full purchase flow: navigate -> select mapping -> select provider -> complete purchase.
 * Returns the payment attempt ID from localStorage.
 */
export async function initiatePurchaseFlow(
  page: Page,
  provider: PaymentProvider,
  realmId: string = TEST_DATA.REALMS.REALM_001
): Promise<string> {
  await page.evaluate(() => localStorage.removeItem('cas-purchase-flow'))
  await page.goto(`/${realmId}/user/purchase-points`)
  await expect(page.locator(SELECTORS.purchasePoints.page)).toBeVisible()

  // Precondition: realm must have at least one active one-time entitlement mapping.
  // If no mappings exist, the page shows empty state and this helper will fail.
  // Callers in conditional test contexts should check for mapping cards first.
  await selectFirstMappingAndProceed(page)
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
 * Clicks cancel button in redirect case to prevent auto-redirect.
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

    const cancelButton = page.locator(SELECTORS.paymentProviderUI.cancelButton)
    await expect(cancelButton).toBeVisible()
    await cancelButton.click()
  } else {
    const degraded = page.locator(SELECTORS.paymentProviderUI.contextDegraded)
    await expect(degraded).toBeVisible()
    await expect(degraded).toContainText('Payment Information Unavailable')

    const cancelButton = page.locator(SELECTORS.paymentProviderUI.cancelButton)
    await expect(cancelButton).toBeVisible()
  }
}
