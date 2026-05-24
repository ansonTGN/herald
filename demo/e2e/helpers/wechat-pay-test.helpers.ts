/**
 * WeChat Pay Test Helpers
 *
 * Provides reusable functions for WeChat Pay configuration and testing.
 * Follows the same pattern as shopify-test.helpers.ts and billing-page.helpers.ts
 */

import { Page, expect } from '@playwright/test'
import { SELECTORS } from '../selectors'

/**
 * WeChat Pay field format specifications
 */
export const WECHAT_PAY_SPECS = {
  /** WeChat Pay AppID: wx followed by 16 hex characters */
  APP_ID_LENGTH: 16,
  /** Merchant ID: 10-digit number */
  MERCHANT_ID_LENGTH: 10,
  /** Certificate Serial No: 12 uppercase hex characters */
  SERIAL_NO_LENGTH: 12,
  /** API v3 Key: minimum 32 characters */
  V3_KEY_MIN_LENGTH: 32,
} as const

/**
 * WeChat Pay configuration data
 */
export interface WechatPayConfigOptions {
  appId: string
  merchantId: string
  serialNo: string
  v3Key: string
  notifyUrl: string
  privateKey: string
}

/**
 * Generate test data for WeChat Pay configuration
 *
 * @param testStartTime - Timestamp seed for generating unique test data
 * @param realmId - Realm ID for notify URL
 * @returns WeChat Pay configuration options
 */
export function generateWechatPayTestData(
  testStartTime: number,
  realmId: string
): WechatPayConfigOptions {
  return {
    appId: `wx${testStartTime.toString(16).padStart(WECHAT_PAY_SPECS.APP_ID_LENGTH, '0')}`,
    merchantId: testStartTime.toString().padStart(WECHAT_PAY_SPECS.MERCHANT_ID_LENGTH, '0'),
    serialNo: testStartTime.toString(16).toUpperCase().padStart(WECHAT_PAY_SPECS.SERIAL_NO_LENGTH, '0'),
    v3Key: `my_v3_${testStartTime.toString().padStart(26, '0')}`.substring(0, 32),
    notifyUrl: `https://example.com/api/third/pay/${realmId}/wechat/webhooks`,
    privateKey: `-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC${testStartTime}
-----END PRIVATE KEY-----`,
  }
}

/**
 * Create WeChat Pay configuration via UI
 *
 * @param page - Playwright page object
 * @param realmId - Realm ID
 * @param config - WeChat Pay configuration options
 */
export async function createWechatPayConfig(
  page: Page,
  realmId: string,
  config: WechatPayConfigOptions
): Promise<void> {
  await page.goto(`/${realmId}/manage/billing/payment-providers`)

  // Wait for page to load and stabilize
  await page.waitForLoadState('networkidle')
  await page.waitForTimeout(500) // Additional wait for React to render

  // Use role-based selector as fallback since data-testid may not be consistently rendered
  const addButton = page.getByRole('button', { name: /Add WeChat Pay/i }).or(page.locator(SELECTORS.wechatPay.addWechatButton))
  await expect(addButton).toBeVisible({ timeout: 10000 })
  await addButton.click()

  // Wait for config form page to be visible (route-based navigation)
  await expect(page.locator(SELECTORS.wechatPay.configFormPage)).toBeVisible({ timeout: 5000 })
  await page.waitForTimeout(200) // Allow form to render

  // Fill form fields sequentially to avoid race conditions
  await page.locator(SELECTORS.wechatPay.appIdInput).fill(config.appId)
  await page.locator(SELECTORS.wechatPay.merchantIdInput).fill(config.merchantId)
  await page.locator(SELECTORS.wechatPay.serialNoInput).fill(config.serialNo)
  await page.locator(SELECTORS.wechatPay.v3KeyInput).fill(config.v3Key)
  await page.locator(SELECTORS.wechatPay.notifyUrlInput).fill(config.notifyUrl)
  await page.locator(SELECTORS.wechatPay.privateKeyInput).fill(config.privateKey)

  // Verify submit button is enabled before clicking
  const submitButton = page.locator(SELECTORS.wechatPay.configSubmitButton)
  await expect(submitButton).toBeEnabled({ timeout: 3000 })
  await submitButton.click()

  // Wait for success toast; page navigates back to providers list after save
  await expect(page.getByText(/WeChat Pay configuration created successfully/i)).toBeVisible({ timeout: 10000 })
  // Page navigates back to the providers list after successful creation
  await expect(page.locator(SELECTORS.wechatPay.configFormPage)).toBeHidden({ timeout: 10000 })
}

/**
 * Delete existing WeChat Pay configuration if present
 *
 * @param page - Playwright page object
 * @param realmId - Realm ID
 */
export async function deleteWechatPayConfigIfExists(
  page: Page,
  realmId: string
): Promise<void> {
  await page.goto(`/${realmId}/manage/billing/payment-providers`)
  await page.waitForLoadState('networkidle')

  const configCard = page.locator(SELECTORS.wechatPay.configCard)
  const isConfigured = await configCard.isVisible().catch(() => false)

  if (isConfigured) {
    // Set up response listener for delete API call
    const deleteResponsePromise = page.waitForResponse(
      response => response.url().includes('/api/third/pay/') && response.url().includes('/wechat') && response.status() === 204,
      { timeout: 10000 }
    )

    // Click delete button
    await page.locator(SELECTORS.wechatPay.deleteConfigButton).click()

    // Wait for confirmation dialog and click confirm
    await expect(page.locator('[data-testid="delete-confirm-dialog"]')).toBeVisible({ timeout: 3000 })
    await page.locator('[data-testid="delete-confirm-button"]').click()

    // Wait for delete API response
    await deleteResponsePromise

    // Wait for success toast message
    await expect(page.getByText(/Payment provider deleted successfully/i)).toBeVisible({ timeout: 10000 })

    // Wait for toast to disappear (animation)
    await page.waitForTimeout(300)

    // Verify config card is no longer visible
    await expect(configCard).toBeHidden({ timeout: 5000 })

    // Wait for network idle to ensure all cleanup operations complete
    await page.waitForLoadState('networkidle')

    // Reload page to ensure clean state
    await page.reload()
    await page.waitForLoadState('networkidle')
    await page.waitForTimeout(500)
  }
}

/**
 * Check if element is visible and log result
 *
 * @param page - Playwright page object
 * @param selector - Test ID selector
 * @param description - Element description for logging
 * @param demoLogger - Logger instance
 * @returns True if element is visible
 */
export async function assertElementVisibleOrLog(
  page: Page,
  selector: string,
  description: string,
  demoLogger: { testCode: { log: (message: string) => void } }
): Promise<boolean> {
  const element = page.locator(selector)
  const isVisible = await element.isVisible().catch(() => false)

  if (isVisible) {
    await element.isVisible()
    demoLogger.testCode.log(`${description} visible`)
  } else {
    demoLogger.testCode.log(`${description} not visible (expected in some scenarios)`)
  }

  return isVisible
}

/**
 * Login and navigate to payment providers page
 *
 * @param page - Playwright page object
 * @param loginPage - Login page fixture
 * @param realmId - Realm ID
 * @param email - User email
 * @param password - User password
 */
export async function loginAndNavigateToPaymentProviders(
  page: Page,
  loginPage: { loginAsAdmin: (email: string, password: string, realmId: string) => Promise<unknown> },
  realmId: string,
  email: string,
  password: string
): Promise<void> {
  await loginPage.loginAsAdmin(email, password, realmId)
  await page.goto(`/${realmId}/manage/billing/payment-providers`)
}

/**
 * Open WeChat Pay configuration form page
 *
 * Clicking "Add WeChat Pay" or "Edit" navigates to a dedicated route page,
 * not a dialog. This helper waits for the page to render.
 *
 * @param page - Playwright page object
 * @param mode - Page mode: 'add' or 'edit'
 */
export async function openWechatPayConfigDialog(page: Page, mode: 'add' | 'edit'): Promise<void> {
  if (mode === 'add') {
    // Use role-based selector as fallback since data-testid may not be consistently rendered
    const addButton = page.getByRole('button', { name: /Add WeChat Pay/ }).or(page.locator(SELECTORS.wechatPay.addWechatButton))
    await expect(addButton).toBeVisible({ timeout: 5000 })
    await addButton.click()
  } else {
    const editButton = page.locator(SELECTORS.wechatPay.editConfigButton)
    await expect(editButton).toBeVisible({ timeout: 5000 })
    await editButton.click()
  }

  // Wait for config form page to render (route-based navigation)
  await expect(page.locator(SELECTORS.wechatPay.configFormPage)).toBeVisible({ timeout: 5000 })

  // Wait for form to be ready (first field should be visible)
  await expect(page.locator(SELECTORS.wechatPay.appIdInput)).toBeVisible({ timeout: 3000 })

  // Allow form to settle
  await page.waitForTimeout(200)
}

/**
 * Fill WeChat Pay configuration form
 *
 * @param page - Playwright page object
 * @param config - WeChat Pay configuration options
 * @param options - Optional parameters
 * @param options.skipFields - Array of field names to skip
 */
export async function fillWechatPayForm(
  page: Page,
  config: WechatPayConfigOptions,
  options?: { skipFields?: string[] }
): Promise<void> {
  const skipFields = options?.skipFields || []

  const fieldActions: [string, string][] = [
    [SELECTORS.wechatPay.appIdInput, config.appId],
    [SELECTORS.wechatPay.merchantIdInput, config.merchantId],
    [SELECTORS.wechatPay.serialNoInput, config.serialNo],
    [SELECTORS.wechatPay.v3KeyInput, config.v3Key],
    [SELECTORS.wechatPay.notifyUrlInput, config.notifyUrl],
    [SELECTORS.wechatPay.privateKeyInput, config.privateKey],
  ]

  // Fill form fields sequentially to avoid race conditions
  for (const [selector, value] of fieldActions) {
    if (!skipFields.includes(selector)) {
      // Wait for field to be visible and editable
      const field = page.locator(selector)
      await expect(field).toBeVisible({ timeout: 3000 })
      await field.fill(value)
      // Verify value was filled
      await expect(field).toHaveValue(value)
    }
  }
}

/**
 * Submit WeChat Pay configuration and wait for result
 *
 * @param page - Playwright page object
 * @param expectSuccess - Whether to expect success (true) or error (false)
 */
export async function submitWechatPayConfig(page: Page, expectSuccess: boolean = true): Promise<void> {
  // Verify submit button is enabled before clicking
  const submitButton = page.locator(SELECTORS.wechatPay.configSubmitButton)
  await expect(submitButton).toBeEnabled({ timeout: 3000 })

  // Click submit button
  await submitButton.click()

  if (expectSuccess) {
    // Wait for success toast; page navigates back to providers list after save
    await expect(page.getByText(/WeChat Pay configuration (created|updated) successfully/i)).toBeVisible({ timeout: 10000 })
    // Page navigates back to the providers list after successful save
    await expect(page.locator(SELECTORS.wechatPay.configFormPage)).toBeHidden({ timeout: 10000 })
  }
  // For error cases, caller should wait for specific error messages
}

/**
 * Verify WeChat Pay configuration display state
 *
 * @param page - Playwright page object
 * @param config - Expected configuration values
 * @param masked - Whether secrets should be masked (true) or visible (false)
 */
export async function verifyConfigDisplay(
  page: Page,
  config: WechatPayConfigOptions,
  masked: boolean = true
): Promise<void> {
  // Verify configuration card is visible
  await expect(page.locator(SELECTORS.wechatPay.configCard)).toBeVisible()

  // Expand the collapsible details row to reveal config fields
  const toggleBtn = page.locator(SELECTORS.wechatPay.toggleDetailsButton)
  await toggleBtn.waitFor({ state: 'visible', timeout: 10000 })
  await toggleBtn.click()

  // Verify non-sensitive fields are displayed
  await expect(page.locator(SELECTORS.wechatPay.appIdDisplay)).toContainText(config.appId)
  await expect(page.locator(SELECTORS.wechatPay.merchantIdDisplay)).toContainText(config.merchantId)
  await expect(page.locator(SELECTORS.wechatPay.serialNoDisplay)).toContainText(config.serialNo)
  await expect(page.locator(SELECTORS.wechatPay.notifyUrlDisplay)).toContainText(config.notifyUrl)

  // Verify v3Key display
  const v3KeyDisplay = page.locator(SELECTORS.wechatPay.v3KeyDisplay)
  if (masked) {
    // Backend masks v3Key showing only first 4 chars + asterisks
    await expect(v3KeyDisplay).toContainText('my_v')
    await expect(v3KeyDisplay).not.toContainText(config.v3Key.substring(5))
  } else {
    await expect(v3KeyDisplay).toContainText(config.v3Key)
  }

  // Verify private key display
  const privateKeyDisplay = page.locator(SELECTORS.wechatPay.privateKeyDisplay)
  if (masked) {
    await expect(privateKeyDisplay).toContainText('(configured)')
    await expect(privateKeyDisplay).not.toContainText('BEGIN PRIVATE KEY')
  } else {
    await expect(privateKeyDisplay).toContainText('BEGIN PRIVATE KEY')
  }
}

/**
 * Update a specific configuration field
 *
 * @param page - Playwright page object
 * @param fieldName - Name of the field to update
 * @param newValue - New value for the field
 */
export async function updateConfigField(
  page: Page,
  fieldName: string,
  newValue: string
): Promise<void> {
  const fieldMap: Record<string, string> = {
    'v3Key': SELECTORS.wechatPay.v3KeyInput,
    'privateKey': SELECTORS.wechatPay.privateKeyInput,
    'serialNo': SELECTORS.wechatPay.serialNoInput,
    'appId': SELECTORS.wechatPay.appIdInput,
    'merchantId': SELECTORS.wechatPay.merchantIdInput,
    'notifyUrl': SELECTORS.wechatPay.notifyUrlInput,
  }

  const selector = fieldMap[fieldName]
  if (!selector) {
    throw new Error(`Unknown field name: ${fieldName}`)
  }

  const field = page.locator(selector)
  await expect(field).toBeVisible({ timeout: 3000 })
  await field.clear()
  await field.fill(newValue)
  // Verify value was updated
  await expect(field).toHaveValue(newValue)
}

/**
 * Initiate WeChat Pay configuration deletion
 *
 * @param page - Playwright page object
 */
export async function initiateConfigDeletion(page: Page): Promise<void> {
  await page.locator(SELECTORS.wechatPay.deleteConfigButton).click()
}

/**
 * Confirm or cancel WeChat Pay configuration deletion
 *
 * @param page - Playwright page object
 * @param confirm - True to confirm deletion, false to cancel
 */
export async function confirmConfigDeletion(page: Page, confirm: boolean): Promise<void> {
  if (confirm) {
    await page.getByRole('button', { name: /delete|confirm/i }).click()
  } else {
    await page.getByRole('button', { name: /cancel/i }).click()
  }
}
