/**
 * Settings Page Object
 *
 * Encapsulates Settings page operations.
 * Provides methods for managing Realm configuration (TOTP, Registration).
 *
 * @see ../../../spec/demo/e2e-testing.md#page-object-model-pom-规范
 */

import { Page, Locator, expect } from '@playwright/test'
import { SELECTORS } from '../selectors'
import { BasePage } from './base-page'
import type { UnifiedLogger } from '../helpers/unified-logger'

/**
 * TOTP Configuration Data
 */
export interface TOTPConfigData {
  enabled: boolean          // Realm 是否启用 TOTP
  force_enabled: boolean    // 是否强制所有用户启用
}

/**
 * Registration Configuration Data
 */
export interface RegistrationConfigData {
  allowed: boolean                        // 是否允许注册
  require_email_verification: boolean     // 是否需要邮箱验证
}

/**
 * OAuth Provider Configuration Data
 */
export interface OAuthProviderConfigData {
  providerType: 'google' | 'github' | 'facebook' | 'apple' | 'wechat' | 'wechat_miniprogram'
  clientId: string
  clientSecret: string
  scopes?: string[]
  enabled?: boolean
}

/**
 * Email Configuration Form Data
 * Matches frontend EmailConfigForm schema.
 */
export interface EmailConfigFormData {
  provider: 'resend' | 'smtp'
  fromAddress: string
  resendApiKey?: string
  smtpHost?: string
  smtpPort?: string
  smtpEncryption?: 'starttls' | 'ssl'
  smtpUsername?: string
  smtpPassword?: string
}

/**
 * Settings Page Object
 *
 * Represents Settings page at /admin/settings
 *
 * @example
 * ```typescript
 * const settingsPage = new SettingsPage(page, logger, 'admin')
 * await settingsPage.goto()
 * await settingsPage.enableTOTP()
 * await settingsPage.saveTOTPConfig()
 * ```
 */
export class SettingsPage extends BasePage {
  private readonly realmId: string

  // Page container
  readonly container: Locator
  readonly heading: Locator

  // Tabs
  readonly totpTab: Locator
  readonly registrationTab: Locator
  readonly providersTab: Locator

  // TOTP Configuration elements
  readonly totpEnabledSwitch: Locator
  readonly totpForceEnabledSwitch: Locator
  readonly totpSaveButton: Locator

  // Registration Configuration elements
  readonly allowRegistrationSwitch: Locator
  readonly requireEmailVerificationSwitch: Locator
  readonly registrationSaveButton: Locator

  // OAuth Provider Configuration elements
  readonly addProviderButton: Locator
  readonly providerTypeSelect: Locator
  readonly clientIdInput: Locator
  readonly clientSecretInput: Locator
  readonly scopesInput: Locator
  readonly enabledCheckbox: Locator
  readonly saveProviderButton: Locator
  readonly cancelProviderButton: Locator

  // Email Configuration elements
  readonly emailTab: Locator
  readonly emailStatusBadge: Locator
  readonly emailStatusError: Locator
  readonly emailProviderResend: Locator
  readonly emailProviderSmtp: Locator
  readonly emailFromAddressInput: Locator
  readonly emailResendApiKeyInput: Locator
  readonly emailSmtpHostInput: Locator
  readonly emailSmtpPortInput: Locator
  readonly emailSmtpEncryptionSelect: Locator
  readonly emailSmtpUsernameInput: Locator
  readonly emailSmtpPasswordInput: Locator
  readonly emailTestRecipientInput: Locator
  readonly emailTestButton: Locator
  readonly emailTestError: Locator
  readonly emailTestSuccess: Locator
  readonly emailSaveButton: Locator
  readonly emailSaveError: Locator

  constructor(page: Page, logger?: UnifiedLogger, realmId: string = 'admin') {
    super(page, logger)
    this.realmId = realmId

    // Page container
    this.container = page.getByTestId('settings-page')
    this.heading = page.getByRole('heading', { name: 'Settings' })

    // Tabs - using data-testid selectors for reliability
    this.totpTab = page.getByTestId('totp-tab')
    this.registrationTab = page.getByTestId('registration-tab')
    this.providersTab = page.getByTestId('providers-tab')

    // TOTP Configuration - using data-testid (semantic selectors not available)
    // ⚠️ Note: The Switch component doesn't set aria-labelledby,
    // so we use data-testid as the primary reliable selector
    // Future improvement: Add aria-label or aria-labelledby to Switch component
    this.totpEnabledSwitch = page.getByTestId('totp-enabled-switch')
    this.totpForceEnabledSwitch = page.getByTestId('totp-force-enabled-switch')

    // Save button in TOTP form
    this.totpSaveButton = page.getByTestId('totp-save-button')

    // Registration Configuration - using data-testid
    this.allowRegistrationSwitch = page.getByTestId('reg-allowed-switch')
    this.requireEmailVerificationSwitch = page.getByTestId('reg-require-email-switch')

    // Save button in Registration form
    this.registrationSaveButton = page.getByTestId('reg-save-button')

    // OAuth Provider Configuration - using data-testid selectors
    this.addProviderButton = page.getByTestId('add-provider-button')
    this.providerTypeSelect = page.getByTestId('oauth-provider-type-select')
    this.clientIdInput = page.getByTestId('oauth-client-id-input')
    this.clientSecretInput = page.getByTestId('oauth-client-secret-input')
    this.scopesInput = page.getByLabel('Scopes')
    this.enabledCheckbox = page.getByTestId('oauth-enabled-checkbox')
    this.saveProviderButton = page.getByTestId('oauth-save-provider-button')
    this.cancelProviderButton = page.getByTestId('oauth-cancel-provider-button')

    // Email Configuration - using data-testid selectors
    this.emailTab = page.getByTestId('email-tab')
    this.emailStatusBadge = page.getByTestId('email-config-status-badge')
    this.emailStatusError = page.getByTestId('email-status-error')
    this.emailProviderResend = page.getByTestId('email-provider-resend')
    this.emailProviderSmtp = page.getByTestId('email-provider-smtp')
    this.emailFromAddressInput = page.getByTestId('email-from-address-input')
    this.emailResendApiKeyInput = page.getByTestId('email-resend-api-key-input')
    this.emailSmtpHostInput = page.getByTestId('email-smtp-host-input')
    this.emailSmtpPortInput = page.getByTestId('email-smtp-port-input')
    this.emailSmtpEncryptionSelect = page.getByTestId('email-smtp-encryption-select')
    this.emailSmtpUsernameInput = page.getByTestId('email-smtp-username-input')
    this.emailSmtpPasswordInput = page.getByTestId('email-smtp-password-input')
    this.emailTestRecipientInput = page.getByTestId('email-test-recipient-input')
    this.emailTestButton = page.getByTestId('email-test-button')
    this.emailTestError = page.getByTestId('email-test-error')
    this.emailTestSuccess = page.getByTestId('email-test-success')
    this.emailSaveButton = page.getByTestId('email-save-button')
    this.emailSaveError = page.getByTestId('email-save-error')
  }

  /**
   * Navigate to Settings page
   */
  async goto(): Promise<void> {
    // 通过点击侧边栏菜单来导航，模拟真实用户操作
    // 这样可以避免权限加载的时序问题
    const settingsMenuLink = this.page.locator(SELECTORS.sidebar.menuSettings)
    await this.smartClick(settingsMenuLink)

    // 等待页面加载完成
    await this.waitForReady()
  }

  /**
   * Wait for Settings page to be ready
   */
  async waitForReady(): Promise<void> {
    await expect(this.container).toBeVisible()
    await expect(this.heading).toBeVisible()
  }

  /**
   * Switch to Security/OTP Tab
   *
   * ✅ Fix: Increased timeout to 10 seconds to handle re-login scenarios.
   * Wait accounts for: navigation, API loading, React Query cache update, component re-rendering.
   */
  async switchToTOTPTab(): Promise<void> {
    await this.smartClick(this.totpTab)

    // Wait for tab content to be visible with longer timeout
    // Account for: navigation, API loading, React Query cache update, re-rendering
    await expect(this.totpEnabledSwitch).toBeVisible({ timeout: 10000 })

    // Additional wait to ensure React state is fully settled
    await this.page.waitForLoadState('networkidle')
  }

  /**
   * Switch to Registration Tab
   *
   * ✅ Fix: Increased timeout to 10 seconds to handle re-login scenarios.
   * Wait accounts for: navigation, API loading, React Query cache update, component re-rendering.
   */
  async switchToRegistrationTab(): Promise<void> {
    await this.smartClick(this.registrationTab)

    // Wait for tab content to be visible with longer timeout
    // Account for: navigation, API loading, React Query cache update, re-rendering
    await expect(this.allowRegistrationSwitch).toBeVisible({ timeout: 10000 })

    // Additional wait to ensure React state is fully settled
    await this.page.waitForLoadState('networkidle')
  }

  // ============================================================================
  // Email Configuration Methods
  // ============================================================================

  /**
   * Switch to Email Tab
   *
   * Follows the same pattern as switchToTOTPTab/switchToRegistrationTab.
   * Waits for the save button to indicate tab content is fully loaded.
   */
  async switchToEmailTab(): Promise<void> {
    await this.smartClick(this.emailTab)

    // Wait for tab content to be visible with longer timeout
    await expect(this.emailSaveButton).toBeVisible({ timeout: 10000 })

    // Additional wait to ensure React state is fully settled
    await this.page.waitForLoadState('networkidle')
  }

  /**
   * Configure Resend email provider
   *
   * Selects the Resend radio and fills the from address and API key fields.
   */
  async configureResend(config: EmailConfigFormData): Promise<void> {
    // Select Resend provider
    await this.smartClick(this.emailProviderResend)

    // Wait for Resend-specific fields to be visible
    await expect(this.emailResendApiKeyInput).toBeVisible({ timeout: 5000 })

    // Fill from address
    await this.fillField(this.emailFromAddressInput, config.fromAddress)

    // Fill API key if provided
    if (config.resendApiKey) {
      await this.fillField(this.emailResendApiKeyInput, config.resendApiKey)
    }
  }

  /**
   * Configure SMTP email provider
   *
   * Selects the SMTP radio and fills all SMTP fields (host, port, encryption, username, password).
   */
  async configureSmtp(config: EmailConfigFormData): Promise<void> {
    // Select SMTP provider
    await this.smartClick(this.emailProviderSmtp)

    // Wait for SMTP-specific fields to be visible
    await expect(this.emailSmtpHostInput).toBeVisible({ timeout: 5000 })

    // Fill from address
    await this.fillField(this.emailFromAddressInput, config.fromAddress)

    // Fill SMTP fields if provided
    if (config.smtpHost) {
      await this.fillField(this.emailSmtpHostInput, config.smtpHost)
    }

    if (config.smtpPort) {
      await this.fillField(this.emailSmtpPortInput, config.smtpPort)
    }

    if (config.smtpEncryption) {
      // Radix Select: click trigger, then select option from dropdown
      await this.smartClick(this.emailSmtpEncryptionSelect)
      const option = this.page.getByRole('option', { name: config.smtpEncryption === 'starttls' ? 'STARTTLS' : 'SSL', exact: true })
      await this.smartClick(option)
    }

    if (config.smtpUsername) {
      await this.fillField(this.emailSmtpUsernameInput, config.smtpUsername)
    }

    if (config.smtpPassword) {
      await this.fillField(this.emailSmtpPasswordInput, config.smtpPassword)
    }
  }

  /**
   * Save Email Configuration
   *
   * Clicks the save button and waits for the button text to return to 'Save',
   * matching the pattern used in saveTOTPConfig/saveRegistrationConfig.
   */
  async saveEmailConfig(): Promise<void> {
    await this.smartClick(this.emailSaveButton)

    // Wait for button text to return to "Save" (indicates save completed)
    await expect(async () => {
      const buttonText = await this.emailSaveButton.textContent()
      expect(buttonText).toBe('Save')
    }).toPass({ timeout: 15000 })
  }

  /**
   * Send a test email
   *
   * Fills the test recipient input and clicks the send test email button.
   */
  async sendTestEmail(recipient: string): Promise<void> {
    await this.fillField(this.emailTestRecipientInput, recipient)
    await this.smartClick(this.emailTestButton)
  }

  /**
   * Get the text content of the email status badge
   */
  async getEmailStatusBadgeText(): Promise<string> {
    return await this.getText(this.emailStatusBadge)
  }

  /**
   * Check if email is configured (based on status badge text)
   */
  async isEmailConfigured(): Promise<boolean> {
    const text = await this.getEmailStatusBadgeText()
    return text.toLowerCase().includes('configured') && !text.toLowerCase().includes('not configured')
  }

  // ============================================================================
  // TOTP Configuration Methods
  // ============================================================================

  /**
   * Enable TOTP
   */
  async enableTOTP(): Promise<void> {
    await this.setCheckbox(this.totpEnabledSwitch, true)
  }

  /**
   * Disable TOTP
   */
  async disableTOTP(): Promise<void> {
    await this.setCheckbox(this.totpEnabledSwitch, false)
  }

  /**
   * Enable Force TOTP
   */
  async enableForceTOTP(): Promise<void> {
    await this.setCheckbox(this.totpForceEnabledSwitch, true)
  }

  /**
   * Disable Force TOTP
   */
  async disableForceTOTP(): Promise<void> {
    await this.setCheckbox(this.totpForceEnabledSwitch, false)
  }

  /**
   * Save TOTP Configuration
   *
   * ✅ Verification: Relies on button state change as confirmation of save completion
   * ⚠️ Note: Removed waitForResponse because Playwright's network monitoring
   * was not capturing the POST response to /api/configs/admin/batch reliably.
   * Button state change (text returning to "Save") is a more reliable indicator.
   */
  async saveTOTPConfig(): Promise<void> {
    // Click save button
    await this.smartClick(this.totpSaveButton)

    // Wait for button text to return to "Save" (indicates save completed)
    await expect(async () => {
      const buttonText = await this.totpSaveButton.textContent()
      expect(buttonText).toBe('Save')
    }).toPass({ timeout: 15000 }) // Increased timeout for API processing
  }

  /**
   * Get current TOTP Configuration state
   */
  async getTOTPConfig(): Promise<TOTPConfigData> {
    const enabled = await this.totpEnabledSwitch.isChecked()
    const force_enabled = await this.totpForceEnabledSwitch.isChecked()

    return { enabled, force_enabled }
  }

  /**
   * Verify TOTP Configuration matches expected values
   */
  async verifyTOTPConfig(expected: TOTPConfigData): Promise<void> {
    const actual = await this.getTOTPConfig()

    expect(actual.enabled).toBe(expected.enabled)
    expect(actual.force_enabled).toBe(expected.force_enabled)
  }

  // ============================================================================
  // Registration Configuration Methods
  // ============================================================================

  /**
   * Enable user registration
   */
  async allowRegistration(): Promise<void> {
    await this.setCheckbox(this.allowRegistrationSwitch, true)
  }

  /**
   * Disable user registration
   */
  async disallowRegistration(): Promise<void> {
    await this.setCheckbox(this.allowRegistrationSwitch, false)
  }

  /**
   * Enable email verification requirement
   */
  async requireEmailVerification(): Promise<void> {
    await this.setCheckbox(this.requireEmailVerificationSwitch, true)
  }

  /**
   * Disable email verification requirement
   */
  async disableEmailVerification(): Promise<void> {
    await this.setCheckbox(this.requireEmailVerificationSwitch, false)
  }

  /**
   * Save Registration Configuration
   *
   * ✅ Verification: Handles page navigation that occurs after save
   * ⚠️ Note: Page reloads after clicking save, causing tab to reset to default (TOTP)
   * This method waits for navigation, re-selects Registration tab, then verifies button state.
   */
  async saveRegistrationConfig(): Promise<void> {
    // Click save button
    await this.smartClick(this.registrationSaveButton)

    // Wait for navigation to complete (page reloads after save)
    await this.page.waitForLoadState('networkidle')

    // Re-navigate to Registration tab (page may have reset to default TOTP tab)
    await this.switchToRegistrationTab()

    // Wait for button text to return to "Save" (indicates save completed)
    await expect(async () => {
      const buttonText = await this.registrationSaveButton.textContent()
      expect(buttonText).toBe('Save')
    }).toPass({ timeout: 15000 }) // Increased timeout for API processing
  }

  /**
   * Get current Registration Configuration state
   */
  async getRegistrationConfig(): Promise<RegistrationConfigData> {
    const allowed = await this.allowRegistrationSwitch.isChecked()
    const require_email_verification = await this.requireEmailVerificationSwitch.isChecked()

    return { allowed, require_email_verification }
  }

  /**
   * Verify Registration Configuration matches expected values
   */
  async verifyRegistrationConfig(expected: RegistrationConfigData): Promise<void> {
    const actual = await this.getRegistrationConfig()

    expect(actual.allowed).toBe(expected.allowed)
    expect(actual.require_email_verification).toBe(expected.require_email_verification)
  }

  // ============================================================================
  // OAuth Provider Configuration Methods
  // ============================================================================

  /**
   * Switch to Providers Tab
   *
   * ✅ Fix: Improved reliability with retry mechanism to handle Vite dev mode reloads.
   * - Uses retry wrapper to handle page reloads that may occur during tab switching
   * - Verifies tab is fully selected (aria-selected='true') before proceeding
   * - Waits for tab panel to be visible, not just the add button
   * - Adds network idle wait to ensure page is fully stable after tab switch
   * - Increased timeout to 15 seconds to accommodate retries
   */
  async switchToProvidersTab(): Promise<void> {
    // Retry mechanism to handle Vite dev mode reloads
    await expect(async () => {
      // Check current tab state
      const isSelected = await this.providersTab.getAttribute('aria-selected')

      // If not selected, click the tab
      if (isSelected !== 'true') {
        await this.smartClick(this.providersTab)
      }

      // Wait for tab to be selected (handle async state updates)
      await expect(async () => {
        const selected = await this.providersTab.getAttribute('aria-selected')
        expect(selected).toBe('true')
      }).toPass({ timeout: 5000 })

      // Wait for tab panel to be visible (indicates tab content is fully loaded)
      const tabPanel = this.page.getByRole('tabpanel', { name: 'Providers' })
      await expect(tabPanel).toBeVisible({ timeout: 5000 })

      // Wait for add button to be visible (secondary verification)
      await expect(this.addProviderButton).toBeVisible({ timeout: 5000 })
    }).toPass({ timeout: 15000 }) // Increased timeout for retries

    // Wait for network to be idle to ensure page is fully stable
    await this.page.waitForLoadState('networkidle')
  }

  /**
   * Add new OAuth Provider
   *
   * ✅ Fix: Wait for provider to appear in list instead of POST response.
   * - Forces switch to Providers tab with retry logic built into switchToProvidersTab()
   * - Adds explicit verification that we're on the right tab (tab panel visible)
   * - Prevents page reload in Vite dev mode that interrupts waitForResponse listeners
   * - Ensures addProviderButton is stable and clickable after tab switch
   * - Adds network idle wait before form operations
   * - Waits for dialog to close and provider to appear in list as success condition
   * - This approach is more reliable than waitForResponse for POST requests
   *   because it verifies the actual user-visible state (provider appears in list)
   */
  async addProvider(config: OAuthProviderConfigData): Promise<void> {
    // ✅ Fix: Force switch to Providers tab with retry logic
    // This ensures we're on the correct tab even if Vite dev mode caused a reload
    await this.switchToProvidersTab()

    // ✅ Fix: Add explicit verification that we're on the right tab
    // Verify the Providers tab panel is visible, not just the tab button
    const tabPanel = this.page.getByRole('tabpanel', { name: 'Providers' })
    await expect(tabPanel).toBeVisible({ timeout: 5000 })

    // ✅ Fix: Additional check to ensure addProviderButton is stable and clickable
    // Wait for button to be fully visible and enabled before clicking
    await expect(async () => {
      const isVisible = await this.addProviderButton.isVisible()
      const isEnabled = await this.addProviderButton.isEnabled()
      expect(isVisible).toBeTruthy()
      expect(isEnabled).toBeTruthy()
    }).toPass({ timeout: 5000 })

    // Click "Add Provider" button
    await this.smartClick(this.addProviderButton)

    // Wait for dialog to open (indicated by provider type select being visible)
    await expect(this.providerTypeSelect).toBeVisible({ timeout: 5000 })

    // ✅ Fix: Wait for network idle to ensure dialog is fully rendered
    await this.page.waitForLoadState('networkidle')

    // Fill in form fields
    await this.smartClick(this.providerTypeSelect)
    // Use exact matching to avoid matching multiple providers (e.g., 'wechat' matching both 'WeChat' and 'WeChat Mini Program')
    const displayName = this.getProviderDisplayName(config.providerType)
    await this.page.getByRole('option', { name: displayName, exact: true }).click()
    await this.fillField(this.clientIdInput, config.clientId)
    await this.fillField(this.clientSecretInput, config.clientSecret)

    // Set enabled state
    if (config.enabled !== undefined) {
      const isEnabled = await this.enabledCheckbox.isChecked()
      if (isEnabled !== config.enabled) {
        await this.smartClick(this.enabledCheckbox)
      }
    }

    // ✅ Fix: Wait for save button to be enabled (form validation complete)
    // TanStack Form's canSubmit state must be true before the click will submit
    await expect(async () => {
      const isDisabled = await this.saveProviderButton.isDisabled()
      expect(isDisabled).toBeFalsy()
    }).toPass({ timeout: 5000 })

    // ✅ Fix: Save provider configuration - DON'T wait for POST response
    // The real success condition is that the provider appears in the list
    // Not that we captured the POST response (which may be missed due to page navigation)
    await this.smartClick(this.saveProviderButton)

    // Wait for dialog to close (provider type select should not be visible)
    await expect(this.providerTypeSelect).not.toBeVisible({ timeout: 5000 })

    // Wait for network to be idle (POST request has been sent and processed)
    await this.page.waitForLoadState('networkidle')

    // Force refresh the providers tab to ensure React Query updates
    await this.switchToProvidersTab()

    // ✅ Fix: Verify provider exists in list (this is the real success condition)
    // This confirms the POST request succeeded and data was persisted
    await this.waitForProviderInList(config.providerType, { timeout: 15000 })
  }

  /**
   * Toggle Provider enabled/disabled state
   */
  async toggleProvider(providerType: string): Promise<void> {
    const providerRow = this.getProviderRow(providerType)
    const toggleButton = providerRow.getByTestId(`provider-toggle-button-${providerType}`)

    // Wait for button to be visible
    await expect(toggleButton).toBeVisible({ timeout: 10000 })

    // Click the toggle button and wait for API to complete
    await this.smartClick(toggleButton)

    // Wait for the API request to complete (simple timeout instead of waitForResponse)
    await this.page.waitForTimeout(2000)
  }

  /**
   * Edit Provider configuration
   *
   * ✅ Fix: Wait for provider to remain in list instead of PUT response.
   * - Added comprehensive page state checks and error handling to prevent race conditions
   * - Waits for provider to remain visible with updated configuration as success condition
   * - This approach is more reliable than waitForResponse for PUT requests
   *   because it verifies the actual user-visible state (provider remains in list)
   */
  async editProvider(providerType: string, config: Partial<OAuthProviderConfigData>): Promise<void> {
    const providerRow = this.getProviderRow(providerType)

    // Click edit button
    const editButton = providerRow.getByTestId(`provider-edit-button-${providerType}`)
    await this.smartClick(editButton)

    // Wait for dialog to open
    await expect(this.providerTypeSelect).toBeVisible({ timeout: 5000 })

    // ✅ Fix: Wait for network idle to ensure dialog is fully rendered
    await this.page.waitForLoadState('networkidle')

    // Update fields if provided
    if (config.clientId !== undefined) {
      // Clear the input first
      await this.clientIdInput.clear()
      // Use type() instead of fill() to ensure React onChange is triggered
      await this.clientIdInput.type(config.clientId)
      // Wait for React state to update
      await this.page.waitForTimeout(500)
    }

    if (config.clientSecret !== undefined && config.clientSecret !== '') {
      await this.fillField(this.clientSecretInput, config.clientSecret)
    }

    if (config.scopes !== undefined && config.scopes.length > 0) {
      await this.fillField(this.scopesInput, config.scopes.join(', '))
    }

    if (config.enabled !== undefined) {
      const isEnabled = await this.enabledCheckbox.isChecked()
      if (isEnabled !== config.enabled) {
        await this.smartClick(this.enabledCheckbox)
      }
    }

    // ✅ Fix: Wait for save button to be enabled (form validation complete)
    // TanStack Form's canSubmit state must be true before the click will submit
    await expect(async () => {
      const isDisabled = await this.saveProviderButton.isDisabled()
      expect(isDisabled).toBeFalsy()
    }).toPass({ timeout: 5000 })

    // ✅ Fix: Save provider configuration - DON'T wait for PUT response
    // The real success condition is that the provider remains in the list
    await this.smartClick(this.saveProviderButton)

    // Wait for dialog to close (provider type select should not be visible)
    await expect(this.providerTypeSelect).not.toBeVisible({ timeout: 3000 })

    // Wait for network to be idle (PUT request has been sent and processed)
    await this.page.waitForLoadState('networkidle')

    // Force refresh the providers tab to ensure React Query updates
    // ✅ Fix: Only switch if not already active to prevent unnecessary page navigation
    const isActive = await this.providersTab.getAttribute('aria-selected')
    if (isActive !== 'true') {
      await this.switchToProvidersTab()
    }

    // ✅ Fix: Verify provider still exists in list (edit was successful)
    await this.waitForProviderInList(providerType, { timeout: 15000 })
  }

  /**
   * Delete Provider
   *
   * ✅ Fix: Directly wait for UI update to verify final user-visible state.
   * Matches the pattern used in addProvider() and editProvider().
   * Backend deletion may take longer, so we use 15000ms timeout.
   */
  async deleteProvider(providerType: string): Promise<void> {
    const providerRow = this.getProviderRow(providerType)

    // Click delete button
    const deleteButton = providerRow.getByTestId(`provider-delete-button-${providerType}`)
    await this.smartClick(deleteButton)

    // Confirm deletion in dialog
    const confirmDeleteButton = this.page.getByTestId('provider-delete-confirm-button')
    await this.smartClick(confirmDeleteButton)

    // Wait for provider to be removed from list (UI verification)
    await this.waitForProviderNotInList(providerType, { timeout: 15000 })
  }

  /**
   * Check if Provider exists in list
   *
   * ✅ Fix: Use waitFor() instead of isVisible() to ensure reliability.
   * isVisible() returns false when element is not in viewport, causing false negatives.
   * waitFor() waits for the element to become visible anywhere in the DOM.
   */
  async providerExists(providerType: string): Promise<boolean> {
    try {
      // Wait for the provider row to be visible in the DOM (not just viewport)
      await this.page.getByTestId(`provider-row-${providerType}`).waitFor({ timeout: 5000 })
      return true
    } catch {
      return false
    }
  }

  /**
   * Get Provider status (enabled/disabled)
   */
  async getProviderStatus(providerType: string): Promise<boolean> {
    const providerRow = this.getProviderRow(providerType)
    const statusBadge = providerRow.getByTestId(`provider-status-${providerType}`)
    const text = await statusBadge.textContent()
    return text === 'Enabled'
  }

  /**
   * Get Client ID for a specific provider
   */
  async getClientId(providerType: string): Promise<string | null> {
    const providerRow = this.getProviderRow(providerType)
    const clientIdElement = providerRow.getByTestId(`provider-client-id-${providerType}`)
    // Wait for element to be visible before reading text
    await expect(clientIdElement).toBeVisible({ timeout: 10000 })
    const text = await clientIdElement.textContent()
    // Extract the actual client ID from "clientId: xxx" format
    const match = text?.match(/clientId:\s*(.+)/)
    return match ? match[1] : null
  }

  /**
   * Get all Client IDs from provider list
   */
  async getClientIds(): Promise<string[]> {
    const elements = await this.page.locator('[data-testid^="provider-client-id-"]').all()
    const ids = await Promise.all(elements.map(async el => {
      const text = await el.textContent()
      const match = text?.match(/clientId:\s*(.+)/)
      return match ? match[1] : null
    }))
    return ids.filter((id): id is string => id !== null && id !== undefined)
  }

  // ============================================================================
  // Private Helper Methods
  // ============================================================================

  /**
   * Get display name for provider type
   * Maps internal provider types to UI display names
   */
  private getProviderDisplayName(providerType: string): string {
    const displayNames: Record<string, string> = {
      google: 'Google',
      github: 'GitHub',
      facebook: 'Facebook',
      apple: 'Apple',
      wechat: 'WeChat',
      wechat_miniprogram: 'WeChat Mini Program',
    }
    return displayNames[providerType] || providerType
  }

  /**
   * Get provider row element by provider type
   */
  private getProviderRow(providerType: string): Locator {
    return this.page.getByTestId(`provider-row-${providerType}`)
  }

  /**
   * Wait for provider to appear in list
   *
   * ✅ Fix: Use polling with count() instead of toBeVisible() for reliability.
   * count() directly checks element presence in DOM, better than visibility checks.
   * Compatible with React Query async updates.
   */
  private async waitForProviderInList(providerType: string, options?: { timeout?: number }): Promise<void> {
    const timeout = options?.timeout || 10000
    const startTime = Date.now()

    while (Date.now() - startTime < timeout) {
      const count = await this.getProviderRow(providerType).count()
      if (count > 0) {
        const isVisible = await this.getProviderRow(providerType).isVisible()
        if (isVisible) {
          return
        }
      }
      await this.page.waitForTimeout(100)
    }

    throw new Error(`Provider "${providerType}" did not appear in the list within ${timeout}ms`)
  }

  /**
   * Wait for provider to be removed from list
   *
   * ✅ Fix: Use polling with count() instead of not.toBeVisible() for reliability.
   * count() directly checks element presence in DOM, better than visibility checks.
   * Compatible with React Query async updates.
   */
  private async waitForProviderNotInList(providerType: string, options?: { timeout?: number }): Promise<void> {
    const timeout = options?.timeout || 2000
    const startTime = Date.now()

    while (Date.now() - startTime < timeout) {
      const count = await this.getProviderRow(providerType).count()
      if (count === 0) {
        return // Element has been removed from DOM
      }
      await this.page.waitForTimeout(100)
    }

    throw new Error(`Provider "${providerType}" was not removed from the list within ${timeout}ms`)
  }
}
