/**
 * Unified Purchase Test Helpers
 *
 * Reusable helper functions for unified purchase flow tests
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
    WECHAT: 'wechat',
    STRIPE: 'stripe',
  } as const,
} as const

export type PaymentProvider = typeof TEST_DATA.PAYMENT_PROVIDERS[keyof typeof TEST_DATA.PAYMENT_PROVIDERS]

/**
 * Extracts payment attempt ID from localStorage
 */
export async function extractPaymentAttemptId(page: Page): Promise<string> {
  await page.waitForTimeout(TEST_DATA.TIMEOUTS.ATTEMPT_CREATION)

  const attemptId = await page.evaluate(() => {
    const state = localStorage.getItem('purchase-flow-storage')
    if (state) {
      const parsed = JSON.parse(state)
      return parsed?.state?.paymentAttempt?.attemptId
    }
    return null
  })

  if (!attemptId) {
    throw new Error('Payment attempt ID not found in localStorage')
  }

  return attemptId
}

/**
 * Fills points package form with provided options
 */
export async function fillPointsPackageForm(
  page: Page,
  options: {
    name: string
    title: string
    description: string
    points: string
    price: string
    currency?: string
    sortOrder?: string
    enabled?: boolean
  }
): Promise<void> {
  await page.getByTestId(SELECTORS.pointsPackageForm.nameInput).fill(options.name)
  await page.getByTestId(SELECTORS.pointsPackageForm.titleInput).fill(options.title)
  await page.getByTestId(SELECTORS.pointsPackageForm.descriptionInput).fill(options.description)
  await page.getByTestId(SELECTORS.pointsPackageForm.pointsInput).fill(options.points)
  await page.getByTestId(SELECTORS.pointsPackageForm.priceInput).fill(options.price)

  if (options.currency) {
    await page.getByTestId(SELECTORS.pointsPackageForm.currencySelect).fill(options.currency)
  }

  if (options.sortOrder) {
    await page.getByTestId(SELECTORS.pointsPackageForm.sortOrderInput).fill(options.sortOrder)
  }

  if (options.enabled !== undefined) {
    const isEnabled = await page.getByTestId(SELECTORS.pointsPackageForm.enabledSwitch).isChecked()
    if (options.enabled && !isEnabled) {
      await page.getByTestId(SELECTORS.pointsPackageForm.enabledSwitch).click()
    }
  }
}

/**
 * Configures payment provider for a package
 */
export async function configurePaymentProvider(
  page: Page,
  provider: PaymentProvider,
  externalId: string,
  enabled: boolean = true
): Promise<void> {
  await page.locator('select[id="paymentProvider"]').selectOption(provider)
  await page.getByTestId(`payment-provider-external-id-input-${provider}`).fill(externalId)

  const isEnabled = await page.getByTestId(`payment-provider-enabled-switch-${provider}`).isChecked()
  if (enabled && !isEnabled) {
    await page.getByTestId(`payment-provider-enabled-switch-${provider}`).click()
  } else if (!enabled && isEnabled) {
    await page.getByTestId(`payment-provider-enabled-switch-${provider}`).click()
  }

  await page.getByTestId('payment-provider-add-button').click()
  await expect(page.getByText('Payment provider mapping added')).toBeVisible()
}

/**
 * Selects first available package and proceeds to payment step
 */
export async function selectFirstPackageAndProceed(page: Page): Promise<void> {
  const firstPackageButton = page.getByTestId(/^points-package-select-button-/)
  await expect(firstPackageButton.first()).toBeEnabled()
  await firstPackageButton.first().click()

  await expect(page.getByTestId(/^points-package-selected-/)).toBeVisible()
  await expect(page.getByTestId(SELECTORS.purchasePoints.nextButton)).toBeEnabled()

  await page.getByTestId(SELECTORS.purchasePoints.nextButton).click()
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

  await page.getByTestId(SELECTORS.purchasePoints.nextButton).click()
}
