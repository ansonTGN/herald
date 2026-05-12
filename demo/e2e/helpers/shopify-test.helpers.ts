/**
 * Shopify Test Helper Functions
 *
 * Helper functions for Shopify payment provider E2E tests.
 * These functions encapsulate common UI interactions for testing.
 */

import type { Page } from '@playwright/test'

/**
 * Creates a Shopify payment provider configuration through the UI
 *
 * @param page - Playwright Page object
 * @param realmId - Realm ID
 * @param options - Configuration options
 * @returns Promise that resolves when configuration is created
 */
export async function createShopifyConfig(
  page: Page,
  realmId: string,
  options: {
    shopDomain: string
    adminAccessToken: string
    storefrontAccessToken: string
    appClientSecret: string
    apiVersion?: string
    skipConnectionTest?: boolean
  }
): Promise<void> {
  // Navigate to payment providers page
  await page.goto(`/${realmId}/billing/payment-providers`)

  // Wait for page to load
  await page.waitForSelector('[data-testid="payment-providers-page"]')

  // Click "Add Provider" button
  await page.click('[data-testid="add-provider-button"]')

  // Wait for provider selection dialog
  await page.waitForSelector('[data-testid="shopify-config-form-dialog"]')

  // Fill in the form
  await page.fill('[data-testid="shop-domain-input"]', options.shopDomain)
  await page.fill('[data-testid="admin-access-token-input"]', options.adminAccessToken)
  await page.fill('[data-testid="storefront-access-token-input"]', options.storefrontAccessToken)
  await page.fill('[data-testid="app-client-secret-input"]', options.appClientSecret)

  if (options.apiVersion) {
    await page.fill('[data-testid="api-version-input"]', options.apiVersion)
  }

  // Check "Skip connection test" if provided (useful for demo/test environments)
  if (options.skipConnectionTest) {
    const skipCheckbox = page.locator('[data-testid="skip-connection-test-checkbox"]')
    if (await skipCheckbox.isVisible()) {
      await skipCheckbox.check()
    }
  }

  // Submit the form
  await page.click('[data-testid="shopify-config-submit-button"]')

  // Wait for success message
  await page.waitForSelector('text=/Shopify configuration created successfully|Payment provider configured successfully/')
}

/**
 * Claims a Shopify subscription through the UI
 *
 * @param page - Playwright Page object
 * @param realmId - Realm ID
 * @param options - Claim options
 * @returns Promise that resolves when subscription is claimed
 */
export async function claimSubscription(
  page: Page,
  realmId: string,
  options: {
    shopifyCustomerId?: string
    contractId?: string
    grantCurrentPeriod?: boolean
  }
): Promise<void> {
  // Navigate to my subscriptions page
  await page.goto(`/${realmId}/subscription/my-subscriptions`)

  // Wait for page to load
  await page.waitForSelector('[data-testid="my-subscriptions-page"]')

  // Click "Claim Subscription" button
  await page.click('[data-testid="claim-subscription-button"]')

  // Wait for claim dialog
  await page.waitForSelector('[data-testid="claim-subscription-dialog"]')

  // Fill in the form
  if (options.shopifyCustomerId) {
    await page.fill('[data-testid="shopify-customer-id-input"]', options.shopifyCustomerId)
  }

  if (options.contractId) {
    await page.fill('[data-testid="contract-id-input"]', options.contractId)
  }

  // Set grant current period checkbox
  if (options.grantCurrentPeriod !== undefined) {
    const checkbox = page.locator('[data-testid="grant-current-period-checkbox"]')
    const isChecked = await checkbox.isChecked()

    if (options.grantCurrentPeriod && !isChecked) {
      await checkbox.check()
    } else if (!options.grantCurrentPeriod && isChecked) {
      await checkbox.uncheck()
    }
  }

  // Submit the form
  await page.click('[data-testid="claim-submit-button"]')

  // Wait for success message
  await page.waitForSelector('text=/Subscription claimed successfully|subscription claimed/i')
}

/**
 * Tests Shopify connection through the UI
 *
 * @param page - Playwright Page object
 * @param realmId - Realm ID
 * @returns Promise that resolves when connection test completes
 */
export async function testShopifyConnection(page: Page, realmId: string): Promise<void> {
  // Navigate to payment providers page
  await page.goto(`/${realmId}/billing/payment-providers`)

  // Wait for page to load
  await page.waitForSelector('[data-testid="payment-providers-page"]')

  // Click "Test Connection" button (assuming there's an existing config)
  await page.click('[data-testid="test-connection-button"]')

  // Wait for test connection dialog
  await page.waitForSelector('[data-testid="test-connection-dialog"]')

  // Wait for connection status indicators
  await page.waitForSelector('[data-testid="connection-status-admin-api"]')
  await page.waitForSelector('[data-testid="connection-status-storefront-api"]')
}

/**
 * Creates an unclaimed Shopify subscription via API
 *
 * **Note**: This function requires the test API endpoint to be available.
 * This is typically only available in test environments.
 *
 * @param page - Playwright Page object
 * @param realmId - Realm ID
 * @param options - Subscription options
 * @returns Promise that resolves when subscription is created
 */
export async function createUnclaimedSubscriptionViaAPI(
  page: Page,
  realmId: string,
  options: {
    shopifyCustomerId: string
    contractId: string
    planId: string
    shopDomain?: string
  }
): Promise<void> {
  const apiUrl = `/api/test/${realmId}/shopify/unclaimed-subscriptions`
  const testApiToken = process.env.TEST_API_TOKEN || 'test-token-123'

  try {
    const response = await page.context().request.post(apiUrl, {
      headers: {
        'Content-Type': 'application/json',
        'x-test-api-token': testApiToken,
      },
      data: {
        shopDomain: options.shopDomain || 'demo-store.myshopify.com',
        shopifyCustomerId: options.shopifyCustomerId,
        shopifyCustomerGid: `gid://shopify/Customer/${options.shopifyCustomerId}`,
        contractId: options.contractId,
        planId: options.planId,
        status: 'active',
      },
    })

    if (!response.ok()) {
      throw new Error(`Failed to create unclaimed subscription: ${response.status()} ${await response.text()}`)
    }
  } catch (error) {
    console.error('Error creating unclaimed subscription:', error)
    throw error
  }
}

/**
 * Navigates to the payment providers page
 *
 * @param page - Playwright Page object
 * @param realmId - Realm ID
 * @returns Promise that resolves when page is loaded
 */
export async function navigateToPaymentProvidersPage(
  page: Page,
  realmId: string
): Promise<void> {
  await page.goto(`/${realmId}/billing/payment-providers`)
  await page.waitForSelector('[data-testid="payment-providers-page"]')
}

/**
 * Navigates to the my subscriptions page
 *
 * @param page - Playwright Page object
 * @param realmId - Realm ID
 * @returns Promise that resolves when page is loaded
 */
export async function navigateToMySubscriptionsPage(
  page: Page,
  realmId: string
): Promise<void> {
  await page.goto(`/${realmId}/subscription/my-subscriptions`)
  await page.waitForSelector('[data-testid="my-subscriptions-page"]')
}

/**
 * Checks if an unclaimed subscription banner is present
 *
 * @param page - Playwright Page object
 * @returns Promise<boolean> - true if banner is present
 */
export async function hasUnclaimedSubscriptionBanner(page: Page): Promise<boolean> {
  const banner = page.locator('[data-testid="unclaimed-subscription-banner"]')
  return await banner.count() > 0
}

/**
 * Gets the unclaimed subscription count from the banner
 *
 * @param page - Playwright Page object
 * @returns Promise<number> - Number of unclaimed subscriptions
 */
export async function getUnclaimedSubscriptionCount(page: Page): Promise<number> {
  const countElement = page.locator('[data-testid="unclaimed-count-display"]')
  const countText = await countElement.textContent()

  if (!countText) {
    return 0
  }

  const match = countText.match(/(\d+)/)
  return match ? parseInt(match[1], 10) : 0
}
