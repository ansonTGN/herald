/**
 * Client Apps Page Object
 *
 * Encapsulates Client App management page operations.
 * Updated to support the new 4-step wizard flow (separate pages).
 *
 * User Stories:
 * - US-TP-005: Client App Configuration Management
 * - US-TP-008: Configure Client App Redirect URI Whitelist
 * - US-TP-009: Manage Client App Icon
 * - US-TP-010: Enable/Disable Client App
 * - US-TP-011: Configure Session TTL Policy
 *
 * @see ../../../spec/demo/e2e-testing.md#page-object-model-pom-规范
 * @see .ai/design/client-app-management-frontend-and-demo.md
 */

import { Page, Locator, expect } from '@playwright/test'
import { SELECTORS } from '../selectors'
import { BasePage } from './base-page'
import type { UnifiedLogger } from '../helpers/unified-logger'

/**
 * Client App data interface
 */
export interface ClientAppData {
  id?: string  // UUID (internal ID)
  clientId?: string  // OAuth client_id
  name: string
  description?: string
  redirectUris?: string[]
  enabled?: boolean
  sessionTtl?: number
  renewalTtl?: number | null
  iconUrl?: string
}

/**
 * Client Apps Page Object
 *
 * Represents Client App management page at /{realmId}/manage/client-apps
 *
 * @example
 * ```typescript
 * const clientAppsPage = new ClientAppsPage(page, logger)
 * await clientAppsPage.goto()
 * await clientAppsPage.createClientApp({
 *   clientId: 'my-app',
 *   name: 'My Application',
 *   redirectUris: ['https://example.com/callback']
 * })
 * ```
 */
export class ClientAppsPage extends BasePage {
  // Page container
  readonly container: Locator
  readonly heading: Locator
  readonly table: Locator
  readonly addButton: Locator

  // Wizard selectors (new flow)
  readonly wizardContainer: Locator
  readonly wizardHeading: Locator
  readonly cancelButton: Locator
  readonly backButton: Locator
  readonly nextButton: Locator
  readonly submitButton: Locator

  // Step 1: Basic Info
  readonly basicInfoStep: Locator
  readonly nameInput: Locator
  readonly descriptionInput: Locator
  readonly appTypeSelect: Locator
  readonly clientTypeRadioGroup: Locator

  // Step 2: Redirect URIs
  readonly redirectUrisStep: Locator
  readonly redirectUrisInput: Locator

  // Step 3: Security
  readonly securityStep: Locator
  readonly sessionTtlPreset: (seconds: number) => Locator
  readonly sessionTtlCustomField: Locator
  readonly sessionRenewalTtlField: Locator

  // Step 4: Review
  readonly reviewStep: Locator

  // Delete confirmation (legacy, may still be needed)
  readonly deleteConfirmDialog: Locator
  readonly deleteConfirmButton: Locator

  // Client Secret display (toast notification)
  readonly successMessage: Locator

  // Draft restore dialog
  readonly draftRestoreDialog: Locator
  readonly draftRestoreButton: Locator
  readonly draftDiscardButton: Locator
  readonly autoSaveToast: Locator

  // Table rows
  readonly clientAppRow: (clientId: string) => Locator

  constructor(page: Page, logger?: UnifiedLogger) {
    super(page, logger)

    // Page selectors
    this.container = this.page.locator(SELECTORS.clientApps.page)
    this.heading = this.page.getByRole('heading', { name: /client apps/i })
    this.table = this.page.locator(SELECTORS.clientApps.table)
    this.addButton = this.page.locator(SELECTORS.clientApps.addButton)

    // Wizard selectors
    this.wizardContainer = this.page.locator(SELECTORS.clientAppWizard.container)
    this.wizardHeading = this.page.locator(SELECTORS.clientAppWizard.heading)
    this.cancelButton = this.page.locator(SELECTORS.clientAppWizard.cancelButton)
    this.backButton = this.page.locator(SELECTORS.clientAppWizard.backButton)
    this.nextButton = this.page.locator(SELECTORS.clientAppWizard.nextButton)
    this.submitButton = this.page.locator(SELECTORS.clientAppWizard.submitButton)

    // Step 1: Basic Info
    this.basicInfoStep = this.page.locator(SELECTORS.clientAppWizard.basicInfoStep)
    this.nameInput = this.page.locator(SELECTORS.clientAppWizard.nameInput)
    this.descriptionInput = this.page.locator(SELECTORS.clientAppWizard.descriptionInput)
    this.appTypeSelect = this.page.locator(SELECTORS.clientAppWizard.appTypeSelect)
    this.clientTypeRadioGroup = this.page.locator(SELECTORS.clientAppWizard.clientTypeRadioGroup)

    // Step 2: Redirect URIs
    this.redirectUrisStep = this.page.locator(SELECTORS.clientAppWizard.redirectUrisStep)
    this.redirectUrisInput = this.page.locator(SELECTORS.clientAppWizard.redirectUrisInput)

    // Step 3: Security
    this.securityStep = this.page.locator(SELECTORS.clientAppWizard.securityStep)
    this.sessionTtlPreset = (seconds: number) =>
      this.page.locator(SELECTORS.clientAppWizard.sessionTtlPreset(seconds))
    this.sessionTtlCustomField = this.page.locator(SELECTORS.clientAppWizard.sessionTtlCustomField)
    this.sessionRenewalTtlField = this.page.locator(SELECTORS.clientAppWizard.sessionRenewalTtlField)

    // Step 4: Review
    this.reviewStep = this.page.locator(SELECTORS.clientAppWizard.reviewStep)

    // Delete confirmation
    this.deleteConfirmDialog = this.page.locator('[data-testid="delete-confirmation-dialog"]')
    this.deleteConfirmButton = this.page.locator('[data-testid="confirm-delete-button"]')

    // Success message (toast)
    this.successMessage = this.page.locator('[data-sonner-toast][data-type="success"]')

    // Draft restore dialog
    this.draftRestoreDialog = this.page.locator(SELECTORS.clientAppWizard.draftRestoreDialog)
    this.draftRestoreButton = this.page.locator(SELECTORS.clientAppWizard.draftRestoreButton)
    this.draftDiscardButton = this.page.locator(SELECTORS.clientAppWizard.draftDiscardButton)
    this.autoSaveToast = this.page.locator('[data-sonner-toast]').filter({ hasText: 'Auto-saved' })

    // Table rows helper
    this.clientAppRow = (clientId: string) =>
      this.page.locator(SELECTORS.clientApps.row(clientId))
  }

  /**
   * Navigate to Client Apps page
   *
   * @param realmId Realm ID (defaults to 'admin')
   */
  async goto(realmId: string = 'admin'): Promise<void> {
    const url = `/${realmId}/manage/client-apps`
    await this.page.goto(url)
    await this.waitForReady()
  }

  /**
   * Wait for Client Apps page to be visible
   */
  async waitForReady(): Promise<void> {
    await expect(this.container).toBeVisible()
    await expect(this.heading).toBeVisible()
    await expect(this.table).toBeVisible()
  }

  /**
   * Navigate to Create Client App Wizard (new flow)
   *
   * @param realmId Realm ID
   */
  async gotoCreateWizard(realmId: string = 'admin'): Promise<void> {
    const url = `/${realmId}/manage/client-apps/new`
    await this.page.goto(url)
    await this.waitForWizardReady()
  }

  /**
   * Navigate to Edit Client App Wizard (new flow)
   *
   * @param realmId Realm ID
   * @param clientAppId Client App ID to edit
   */
  async gotoEditWizard(realmId: string, clientAppId: string): Promise<void> {
    const url = `/${realmId}/manage/client-apps/${clientAppId}/edit`
    await this.page.goto(url)
    await this.waitForWizardReady()
  }

  /**
   * Wait for Wizard to be ready
   */
  async waitForWizardReady(): Promise<void> {
    await expect(this.wizardContainer).toBeVisible()
    await expect(this.wizardHeading).toBeVisible()
  }

  // ============================================================================
  // Wizard Step Methods
  // ============================================================================

  /**
   * Fill Step 1: Basic Information
   *
   * @param data Basic info data
   */
  async fillStep1BasicInfo(data: {
    name: string
    description?: string
    appType?: string
    clientType?: 'confidential' | 'public'
  }): Promise<void> {
    // Verify we're on Step 1
    await expect(this.basicInfoStep).toBeVisible()

    // Fill Name
    await expect(this.nameInput).toBeVisible()
    await this.nameInput.fill(data.name)
    this.logger?.testCode.log(`✓ Filled App Name: "${data.name}"`)

    // Fill Description (optional)
    if (data.description) {
      await this.descriptionInput.fill(data.description)
      this.logger?.testCode.log(`✓ Filled Description: "${data.description}"`)
    }

    // Select App Type (optional)
    if (data.appType) {
      await this.appTypeSelect.click()
      await this.page.getByRole('option', { name: data.appType }).click()
      this.logger?.testCode.log(`✓ Selected App Type: "${data.appType}"`)
    }

    // Select Client Type (optional)
    if (data.clientType) {
      const radio = this.clientTypeRadioGroup.locator(
        SELECTORS.clientAppWizard.clientTypeRadio(data.clientType)
      )
      await radio.click()
      this.logger?.testCode.log(`✓ Selected Client Type: "${data.clientType}"`)
    }
  }

  /**
   * Fill Step 2: Redirect URIs
   *
   * @param redirectUris Array of redirect URIs
   */
  async fillStep2RedirectUris(redirectUris: string[]): Promise<void> {
    // Verify we're on Step 2
    await expect(this.redirectUrisStep).toBeVisible()

    // The redirect URIs input is a dynamic component that manages multiple URIs
    // We'll fill URIs by interacting with the input component
    const input = this.redirectUrisInput

    for (const uri of redirectUris) {
      // Click the input to focus it
      await input.click()

      // Fill the URI
      await input.fill(uri)

      // Press Enter to add the URI (this is how the component works)
      await this.page.keyboard.press('Enter')

      // Wait for the URI to be added to the list
      await expect(this.page.locator(`[data-testid^="uri-item-"]`).filter({ hasText: uri })).toBeVisible({ timeout: 5000 })

      this.logger?.testCode.log(`✓ Added Redirect URI: "${uri}"`)
    }
  }

  /**
   * Fill Step 3: Security Settings
   *
   * @param sessionTtl Session TTL in seconds
   * @param renewalTtl Optional renewal TTL in seconds
   */
  async fillStep3Security(sessionTtl: number, renewalTtl?: number): Promise<void> {
    // Verify we're on Step 3
    await expect(this.securityStep).toBeVisible()

    // Use preset button if available, otherwise use custom field
    const presetButtons = [
      1800, 3600, 7200, 86400 // Common presets
    ]

    if (presetButtons.includes(sessionTtl)) {
      await this.sessionTtlPreset(sessionTtl).click()
      this.logger?.testCode.log(`✓ Selected Session TTL preset: ${sessionTtl}s`)
    } else {
      await this.sessionTtlCustomField.fill(sessionTtl.toString())
      this.logger?.testCode.log(`✓ Filled custom Session TTL: ${sessionTtl}s`)
    }

    // Fill Renewal TTL (optional)
    if (renewalTtl !== undefined) {
      await this.sessionRenewalTtlField.fill(renewalTtl.toString())
      this.logger?.testCode.log(`✓ Filled Renewal TTL: ${renewalTtl}s`)
    }
  }

  /**
   * Navigate to next wizard step
   */
  async goToNextStep(): Promise<void> {
    await expect(this.nextButton).toBeEnabled()
    await this.smartClick(this.nextButton)
    this.logger?.testCode.log('✓ Clicked Next button')

    // Wait for frontend transition animation to complete (200ms transition + 100ms buffer)
    // Technical reason: Frontend wizard uses 200ms transition animations between steps
    await this.page.waitForTimeout(300)
  }

  /**
   * Navigate to previous wizard step
   */
  async goToPreviousStep(): Promise<void> {
    await expect(this.backButton).toBeVisible()
    await this.smartClick(this.backButton)
    this.logger?.testCode.log('✓ Clicked Back button')

    // Wait for frontend transition animation to complete (200ms transition + 100ms buffer)
    // Technical reason: Frontend wizard uses 200ms transition animations between steps
    await this.page.waitForTimeout(300)
  }

  /**
   * Submit wizard (create or update)
   */
  async submitWizard(): Promise<void> {
    // Verify we're on Review step
    await expect(this.reviewStep).toBeVisible()

    // Click submit button
    await expect(this.submitButton).toBeEnabled()
    await this.smartClick(this.submitButton)
    this.logger?.testCode.log('✓ Clicked Submit button')

    // Wait for success message (toast notification)
    await expect(this.successMessage).toBeVisible({ timeout: 10000 })
    this.logger?.testCode.log('✓ Success message appeared')

    // Wait for navigation back to list page
    await this.waitForReady()
  }

  /**
   * Cancel wizard
   */
  async cancelWizard(): Promise<void> {
    await this.smartClick(this.cancelButton)
    this.logger?.testCode.log('✓ Clicked Cancel button')

    // Wait for navigation back to list page
    await this.waitForReady()
  }

  /**
   * Create a new Client App using the wizard flow
   *
   * @param data Client App data
   * @param realmId Realm ID
   */
  async createClientApp(data: ClientAppData, realmId: string = 'admin'): Promise<void> {
    // Navigate to wizard
    await this.gotoCreateWizard(realmId)

    // Step 1: Basic Information
    await this.fillStep1BasicInfo({
      name: data.name,
      description: data.description,
      appType: 'Web', // Default
      clientType: 'confidential', // Default
    })
    await this.goToNextStep()

    // Step 2: Redirect URIs
    if (data.redirectUris && data.redirectUris.length > 0) {
      await this.fillStep2RedirectUris(data.redirectUris)
    }
    await this.goToNextStep()

    // Step 3: Security Settings
    await this.fillStep3Security(
      data.sessionTtl || 3600,
      data.renewalTtl
    )
    await this.goToNextStep()

    // Step 4: Review & Submit
    await this.submitWizard()

    // Navigate back to list page to verify creation
    await this.goto(realmId)

    // Verify the Client App was created
    await this.waitForClientAppByName(data.name)
    this.logger?.testCode.log(`✓ Verified Client App "${data.name}" created successfully`)
  }

  /**
   * Edit existing Client App using the wizard flow
   *
   * @param clientAppId Client App ID to edit
   * @param data Updated Client App data
   * @param realmId Realm ID
   */
  async editClientAppWizard(clientAppId: string, data: Partial<ClientAppData>, realmId: string = 'admin'): Promise<void> {
    // Navigate to edit wizard
    await this.gotoEditWizard(realmId, clientAppId)

    // Step 1: Basic Information (if provided)
    if (data.name || data.description) {
      await this.fillStep1BasicInfo({
        name: data.name || '',
        description: data.description,
      })
      await this.goToNextStep()
    } else {
      await this.goToNextStep()
    }

    // Step 2: Redirect URIs (if provided)
    if (data.redirectUris) {
      await this.fillStep2RedirectUris(data.redirectUris)
    }
    await this.goToNextStep()

    // Step 3: Security Settings (if provided)
    if (data.sessionTtl !== undefined || data.renewalTtl !== undefined) {
      await this.fillStep3Security(
        data.sessionTtl || 3600,
        data.renewalTtl
      )
    }
    await this.goToNextStep()

    // Step 4: Review & Submit
    await this.submitWizard()

    // Navigate back to list page
    await this.goto(realmId)

    this.logger?.testCode.log(`✓ Client App "${clientAppId}" updated successfully`)
  }

  // ============================================================================
  // Legacy Methods (for backward compatibility)
  // ============================================================================

  /**
   * Click "Add Client App" button (legacy, opens new wizard page)
   */
  async clickAddClientApp(): Promise<void> {
    await this.smartClick(this.addButton)
    // Wait for navigation to wizard page
    await this.waitForWizardReady()
  }

  /**
   * Edit existing Client App (legacy, navigates to edit wizard page)
   *
   * @param appId Client App UUID (id field) to edit
   */
  async editClientApp(appId: string): Promise<void> {
    const row = this.clientAppRow(appId)
    const editButton = row.locator('[data-testid="edit-client-app-button"]')

    await this.smartClick(editButton)
    // Wait for navigation to edit wizard page
    await this.waitForWizardReady()
  }

  /**
   * Edit existing Client App by OAuth client_id
   *
   * @param clientId OAuth client_id to edit
   */
  async editClientAppByClientId(clientId: string): Promise<void> {
    const row = this.page.locator(SELECTORS.clientApps.rowByClientId(clientId))
    const editButton = row.locator('[data-testid="edit-client-app-button"]')

    await this.smartClick(editButton)
    // Wait for navigation to edit wizard page
    await this.waitForWizardReady()
  }

  /**
   * Delete Client App
   *
   * @param appId Client App UUID (id field) to delete
   */
  async deleteClientApp(appId: string): Promise<void> {
    const row = this.clientAppRow(appId)
    const deleteButton = row.locator('[data-testid="delete-client-app-button"]')

    // Click delete button to open confirmation dialog
    await this.smartClick(deleteButton)

    // Wait for delete confirmation dialog to appear
    await expect(this.deleteConfirmDialog).toBeVisible({ timeout: 10000 })

    // Click confirm button to delete
    await this.smartClick(this.deleteConfirmButton)

    // Wait for dialog to close
    await expect(this.deleteConfirmDialog).toBeHidden({ timeout: 10000 })
  }

  /**
   * Check if Client App exists in table
   *
   * @param appId Client App UUID (id field) to check
   * @returns True if Client App exists
   */
  async clientAppExists(appId: string): Promise<boolean> {
    const row = this.clientAppRow(appId)
    const count = await row.count()
    if (count > 0) return true

    // Fallback: try to find by name in table
    const nameCell = this.page.getByText(appId).first()
    return await nameCell.isVisible({ timeout: 2000 }).catch(() => false)
  }

  /**
   * Wait for Client App to appear in table by name
   *
   * @param name Client App name
   * @param timeout Timeout in milliseconds (default: 10000)
   */
  async waitForClientAppByName(name: string, timeout: number = 10000): Promise<void> {
    await expect(this.page.getByText(name)).toBeVisible({ timeout })
  }

  /**
   * Get Client App UUID by name
   *
   * @param name Client App name
   * @returns Client App UUID (id field) or empty string if not found
   */
  async getClientIdByName(name: string): Promise<string> {
    const nameCell = this.page.getByText(name).first()
    const isVisible = await nameCell.isVisible({ timeout: 5000 }).catch(() => false)

    if (!isVisible) {
      return ''
    }

    const row = nameCell.locator('xpath=ancestor::tr[1]')
    const appId = await row.getAttribute('data-app-id')

    return appId || ''
  }

  /**
   * Search for Client App by name or Client ID
   *
   * @param query Search query
   */
  async search(query: string): Promise<void> {
    const searchInput = this.page.getByPlaceholder(/search/i)
    await searchInput.fill(query)
  }

  /**
   * Get list of Client Apps from table
   *
   * @returns Array of Client App data with both UUID and client_id
   */
  async getClientAppsList(): Promise<Array<{ id: string; clientId: string; name: string }>> {
    await this.waitForReady()

    const rows = this.page.locator('[data-app-id]')
    const count = await rows.count()

    const apps: Array<{ id: string; clientId: string; name: string }> = []

    for (let i = 0; i < count; i++) {
      const row = rows.nth(i)
      const id = await row.getAttribute('data-app-id')
      const clientId = await row.getAttribute('data-client-id')
      const name = await row.locator('[data-field="name"]').textContent()

      if (id && clientId && name) {
        apps.push({ id, clientId, name })
      }
    }

    return apps
  }

  // ============================================================================
  // Draft Auto-Save Methods
  // ============================================================================

  /**
   * Wait for auto-save toast to appear (Method 1: Toast monitoring)
   *
   * This is the recommended method for testing auto-save functionality.
   * It waits for the "Auto-saved" toast notification to appear.
   *
   * @param timeout Timeout in milliseconds (default: 35000ms to accommodate 30s interval)
   *
   * Testing Strategy:
   * - This method uses assertion-based waiting (Playwright auto-wait)
   * - No fixed delays are used
   * - Waits for the actual toast notification from the frontend
   */
  async waitForDraftSaved(timeout: number = 35000): Promise<void> {
    await expect(this.autoSaveToast).toBeVisible({ timeout })
    this.logger?.testCode.log('✓ Auto-save toast appeared')
  }

  /**
   * Reload the current page
   *
   * Used to test draft recovery functionality
   */
  async reloadPage(): Promise<void> {
    await this.page.reload()
    this.logger?.testCode.log('✓ Page reloaded')
  }

  /**
   * Check if draft restore dialog is visible
   *
   * @returns True if dialog is visible
   */
  async isDraftRestoreDialogVisible(): Promise<boolean> {
    const isVisible = await this.draftRestoreDialog.isVisible().catch(() => false)
    this.logger?.testCode.log(`✓ Draft restore dialog visible: ${isVisible}`)
    return isVisible
  }

  /**
   * Click "Restore Draft" button in draft restore dialog
   */
  async clickRestoreDraft(): Promise<void> {
    await expect(this.draftRestoreButton).toBeVisible()
    await this.smartClick(this.draftRestoreButton)
    this.logger?.testCode.log('✓ Clicked Restore Draft button')
  }

  /**
   * Click "Discard Draft" button in draft restore dialog
   */
  async clickDiscardDraft(): Promise<void> {
    await expect(this.draftDiscardButton).toBeVisible()
    await this.smartClick(this.draftDiscardButton)
    this.logger?.testCode.log('✓ Clicked Discard Draft button')
  }

  /**
   * Get draft data from localStorage (Method 2: LocalStorage polling)
   *
   * This is a fallback method if toast monitoring is not reliable.
   * It directly checks localStorage for draft data.
   *
   * @param draftKey The localStorage key for the draft
   * @returns Draft data or null if not found
   *
   * Testing Strategy:
   * - This method polls localStorage for draft data
   * - Useful when toast notifications are not implemented or unreliable
   * - Still uses assertion-based waiting (retry logic)
   */
  async getDraftFromStorage(draftKey: string): Promise<any> {
    const draftData = await this.page.evaluate((key) => {
      const stored = localStorage.getItem(key)
      if (!stored) return null

      try {
        return JSON.parse(stored)
      } catch {
        return null
      }
    }, draftKey)

    this.logger?.testCode.log(`✓ Retrieved draft from storage: ${draftKey}`)
    return draftData
  }

  /**
   * Verify draft has been cleared from localStorage
   *
   * @param draftKey The localStorage key for the draft
   * @returns True if draft is cleared (not found)
   */
  async verifyDraftCleared(draftKey: string): Promise<boolean> {
    const isCleared = await this.page.evaluate((key) => {
      return localStorage.getItem(key) === null
    }, draftKey)

    if (isCleared) {
      this.logger?.testCode.log(`✓ Draft cleared from storage: ${draftKey}`)
    } else {
      this.logger?.testCode.log(`✗ Draft still exists in storage: ${draftKey}`)
    }

    return isCleared
  }

  /**
   * Wait for draft to be saved to localStorage (Method 2: LocalStorage polling)
   *
   * This method polls localStorage until draft data is found.
   *
   * @param draftKey The localStorage key for the draft
   * @param timeout Timeout in milliseconds (default: 35000ms)
   *
   * Testing Strategy:
   * - Uses retry logic with assertions instead of fixed delays
   * - Polls localStorage every 500ms
   * - Throws error if timeout is reached
   */
  async waitForDraftInStorage(draftKey: string, timeout: number = 35000): Promise<void> {
    const startTime = Date.now()
    const pollInterval = 500

    while (Date.now() - startTime < timeout) {
      const draft = await this.getDraftFromStorage(draftKey)
      if (draft && draft.data && draft.timestamp) {
        this.logger?.testCode.log(`✓ Draft found in storage: ${draftKey}`)
        return
      }
      // Wait before next poll (using small delay for polling, not fixed wait)
      await this.page.waitForTimeout(pollInterval)
    }

    throw new Error(`Draft not found in storage within ${timeout}ms: ${draftKey}`)
  }

  // ============================================================================
  // Step Validation and Navigation Methods (DEMO-D04)
  // ============================================================================

  /**
   * Get validation errors for current step
   *
   * @returns Array of validation error messages
   */
  async getValidationErrors(): Promise<string[]> {
    const errors: string[] = []

    // Get all error messages with role="alert"
    const errorElements = this.page.locator('[role="alert"]')
    const count = await errorElements.count()

    for (let i = 0; i < count; i++) {
      const errorText = await errorElements.nth(i).textContent()
      if (errorText && errorText.trim()) {
        errors.push(errorText.trim())
      }
    }

    return errors
  }

  /**
   * Verify step validation errors
   *
   * @param step Step number (1-4)
   * @param expectedErrors Expected error messages
   */
  async verifyStepValidation(step: number, expectedErrors: string[]): Promise<void> {
    const actualErrors = await this.getValidationErrors()

    // Verify all expected errors are present
    for (const expectedError of expectedErrors) {
      const hasError = actualErrors.some((error) =>
        error.toLowerCase().includes(expectedError.toLowerCase())
      )
      if (!hasError) {
        throw new Error(`Expected validation error not found: "${expectedError}"`)
      }
    }

    // Verify error count matches
    if (actualErrors.length !== expectedErrors.length) {
      throw new Error(
        `Expected ${expectedErrors.length} validation errors, but found ${actualErrors.length}: ${actualErrors.join(', ')}`
      )
    }

    this.logger?.testCode.log(`✓ Step ${step} validation verified: ${expectedErrors.length} errors`)
  }

  /**
   * Click step indicator in progress bar
   *
   * @param stepId Step ID: 'basic' | 'redirect-uris' | 'security' | 'review'
   */
  async clickStepIndicator(stepId: 'basic' | 'redirect-uris' | 'security' | 'review'): Promise<void> {
    const stepButton = this.page.locator(SELECTORS.clientAppWizard.progressStep(stepId))
    await this.smartClick(stepButton)
    this.logger?.testCode.log(`✓ Clicked step indicator: ${stepId}`)

    // Wait for frontend transition animation to complete (200ms transition + 100ms buffer)
    // Technical reason: Frontend wizard uses 200ms transition animations between steps
    await this.page.waitForTimeout(300)
  }

  /**
   * Get current wizard step
   *
   * @returns Current step number (1-4)
   */
  async getCurrentStep(): Promise<number> {
    // Check which step container is visible
    const steps = [
      { number: 1, selector: '[data-testid="wizard-step-basic"]' },
      { number: 2, selector: '[data-testid="wizard-step-redirect-uris"]' },
      { number: 3, selector: '[data-testid="wizard-step-security"]' },
      { number: 4, selector: '[data-testid="wizard-step-review"]' },
    ]

    for (const step of steps) {
      const isVisible = await this.page.locator(step.selector).isVisible().catch(() => false)
      if (isVisible) {
        return step.number
      }
    }

    throw new Error('Unable to determine current step')
  }

  /**
   * Verify progress indicator state
   *
   * @param currentStep Expected current step (1-4)
   */
  async verifyProgressIndicator(currentStep: number): Promise<void> {
    const actualStep = await this.getCurrentStep()

    if (actualStep !== currentStep) {
      throw new Error(`Expected step ${currentStep}, but current step is ${actualStep}`)
    }

    // Map step number to step ID
    const stepIdMap = {
      1: 'basic',
      2: 'redirect-uris',
      3: 'security',
      4: 'review',
    } as const
    const stepId = stepIdMap[currentStep as keyof typeof stepIdMap]

    // Verify the step indicator shows the correct current step
    const currentStepIndicator = this.page.locator(SELECTORS.clientAppWizard.currentStep(stepId))
    await expect(currentStepIndicator).toBeVisible()

    this.logger?.testCode.log(`✓ Progress indicator verified: Step ${currentStep}`)
  }

  /**
   * Verify individual step status
   *
   * This method checks the status of a specific step using the data-status attribute.
   * Unlike verifyProgressIndicator() which only checks the current step, this method
   * can verify any step's status (completed/current/pending).
   *
   * @param stepNumber Step number (1-4)
   * @param expectedStatus Expected status: 'completed' | 'current' | 'pending'
   */
  async verifyStepStatus(
    stepNumber: number,
    expectedStatus: 'completed' | 'current' | 'pending'
  ): Promise<void> {
    // Map step number to step ID
    const stepIdMap = {
      1: 'basic',
      2: 'redirect-uris',
      3: 'security',
      4: 'review',
    } as const
    const stepId = stepIdMap[stepNumber as keyof typeof stepIdMap]

    const stepIndicator = this.page.locator(SELECTORS.clientAppWizard.progressStep(stepId))

    // Verify the step has the expected status attribute
    await expect(stepIndicator).toHaveAttribute('data-status', expectedStatus)

    this.logger?.testCode.log(`✓ Step ${stepNumber} status verified: ${expectedStatus}`)
  }

  /**
   * Verify Next button state
   *
   * @param enabled Expected enabled state
   */
  async verifyNextButtonState(enabled: boolean): Promise<void> {
    if (enabled) {
      await expect(this.nextButton).toBeEnabled()
      this.logger?.testCode.log('✓ Next button is enabled')
    } else {
      await expect(this.nextButton).toBeDisabled()
      this.logger?.testCode.log('✓ Next button is disabled')
    }
  }

  /**
   * Verify step data is preserved
   *
   * @param step Step number (1-4)
   * @param expectedData Expected data to verify
   */
  async verifyStepData(step: number, expectedData: Record<string, any>): Promise<void> {
    switch (step) {
      case 1: {
        // Verify Step 1 data
        if (expectedData.name) {
          const nameValue = await this.nameInput.inputValue()
          expect(nameValue).toBe(expectedData.name)
        }
        if (expectedData.description !== undefined) {
          const descriptionValue = await this.descriptionInput.inputValue()
          expect(descriptionValue).toBe(expectedData.description)
        }
        this.logger?.testCode.log(`✓ Step 1 data verified`)
        break
      }
      case 2: {
        // Verify Step 2 data (redirect URIs)
        if (expectedData.redirectUris) {
          // Redirect URIs are displayed as uri-item elements within the redirect-uris container
          const uriContainer = this.page.locator('[data-testid="redirect-uris"]')
          const uriItems = uriContainer.locator('[data-testid^="uri-item-"]')
          const count = await uriItems.count()
          expect(count).toBe(expectedData.redirectUris.length)

          for (let i = 0; i < expectedData.redirectUris.length; i++) {
            const itemText = await uriItems.nth(i).textContent()
            expect(itemText).toContain(expectedData.redirectUris[i])
          }
        }
        this.logger?.testCode.log(`✓ Step 2 data verified`)
        break
      }
      case 3: {
        // Verify Step 3 data (session settings)
        if (expectedData.sessionTtl !== undefined) {
          const ttlValue = await this.sessionTtlCustomField.inputValue()
          expect(parseInt(ttlValue)).toBe(expectedData.sessionTtl)
        }
        if (expectedData.renewalTtl !== undefined) {
          const renewalValue = await this.sessionRenewalTtlField.inputValue()
          expect(parseInt(renewalValue)).toBe(expectedData.renewalTtl)
        }
        this.logger?.testCode.log(`✓ Step 3 data verified`)
        break
      }
      default:
        throw new Error(`Invalid step number: ${step}`)
    }
  }
}
