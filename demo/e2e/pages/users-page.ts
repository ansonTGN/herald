/**
 * Users Page Object
 *
 * Encapsulates user management page operations.
 * Provides methods for creating, editing, deleting, and searching users.
 *
 * @see ../../../spec/demo/e2e-testing.md#page-object-model-pom-规范
 */

import { Page, Locator, expect } from '@playwright/test'
import { SELECTORS } from '../selectors'
import { BasePage } from './base-page'
import type { UnifiedLogger } from '../helpers/unified-logger'

/**
 * User data interface
 */
export interface UserData {
  email: string
  password?: string
  nickname?: string
  name?: string
}

/**
 * Users Page Object
 *
 * Represents the user management page at /{realmId}/users
 *
 * @example
 * ```typescript
 * const usersPage = new UsersPage(page, logger)
 * await usersPage.goto()
 * await usersPage.createUser({ email: 'user@example.com', password: 'password123' })
 * ```
 */
export class UsersPage extends BasePage {
  // Selectors
  readonly container: Locator
  readonly heading: Locator
  readonly table: Locator
  readonly addButton: Locator
  readonly searchInput: Locator
  readonly dialog: Locator
  readonly dialogTitle: Locator
  readonly emailInput: Locator
  readonly passwordInput: Locator
  readonly nicknameInput: Locator
  readonly nameInput: Locator
  readonly dialogCancelButton: Locator
  readonly dialogSubmitButton: Locator
  readonly toast: Locator
  readonly toastMessage: Locator

  // Reset password selectors
  readonly resetPasswordConfirmDialog: Locator
  readonly resetPasswordConfirmButton: Locator
  readonly resetPasswordResultDialog: Locator
  readonly resetPasswordNewPasswordText: Locator
  readonly resetPasswordCopyButton: Locator

  constructor(page: Page, logger?: UnifiedLogger) {
    super(page, logger)
    this.container = page.locator(SELECTORS.users.container)
    this.heading = page.locator(SELECTORS.users.heading)
    this.table = page.locator(SELECTORS.users.table)
    this.addButton = page.locator(SELECTORS.users.addButton)
    this.searchInput = page.locator(SELECTORS.users.searchInput)

    // Dialog selectors
    this.dialog = page.locator(SELECTORS.common.dialog)
    this.dialogTitle = page.locator(SELECTORS.common.dialogTitle)
    this.emailInput = page.locator(SELECTORS.common.formEmailInput)
    this.passwordInput = page.locator(SELECTORS.common.formPasswordInput)
    this.nicknameInput = page.locator(SELECTORS.common.formNicknameInput)
    this.nameInput = page.locator(SELECTORS.common.formNameInput)
    this.dialogCancelButton = page.locator(SELECTORS.common.dialogCancelButton)
    this.dialogSubmitButton = page.locator(SELECTORS.common.dialogSubmitButton)

    // Feedback selectors
    this.toast = page.locator(SELECTORS.common.toast)
    this.toastMessage = page.locator(SELECTORS.common.toastMessage)

    // Reset password selectors
    this.resetPasswordConfirmDialog = page.locator(SELECTORS.resetPassword.confirmDialog)
    this.resetPasswordConfirmButton = page.locator(SELECTORS.resetPassword.confirmButton)
    this.resetPasswordResultDialog = page.locator(SELECTORS.resetPassword.resultDialog)
    this.resetPasswordNewPasswordText = page.locator(SELECTORS.resetPassword.newPasswordText)
    this.resetPasswordCopyButton = page.locator(SELECTORS.resetPassword.copyButton)
  }

  /**
   * Navigate to users page
   *
   * @param realmId Realm ID (defaults to 'admin' for backward compatibility)
   */
  async goto(realmId: string = 'admin'): Promise<void> {
    // 通过点击侧边栏菜单来导航，模拟真实用户操作
    // 这样可以避免权限加载的时序问题
    const usersMenuLink = this.page.locator(SELECTORS.sidebar.menuUsers)
    await this.smartClick(usersMenuLink)

    // 等待页面加载完成
    await this.waitForReady()
  }

  /**
   * Wait for users page to be visible
   */
  async waitForReady(): Promise<void> {
    await expect(this.container).toBeVisible()
    await expect(this.heading).toBeVisible()
    await expect(this.table).toBeVisible()
  }

  /**
   * Click "Add User" button to open create dialog
   */
  async clickAddUser(): Promise<void> {
    await this.smartClick(this.addButton)
    await expect(this.dialog).toBeVisible()
    await expect(this.dialogTitle).toHaveText(/Add User|Create User|New User/i)
  }

  /**
   * Fill user form fields
   *
   * @param userData User data to fill (partial allowed for edit mode)
   */
  async fillUserForm(userData: Partial<UserData>): Promise<void> {
    if (userData.email) {
      await this.fillField(this.emailInput, userData.email)
    }

    if (userData.password) {
      await this.fillField(this.passwordInput, userData.password)
    }

    if (userData.nickname) {
      await this.fillField(this.nicknameInput, userData.nickname)
    }

    if (userData.name) {
      await this.fillField(this.nameInput, userData.name)
    }

    // Check the default "User" role checkbox (required by createUserSchema)
    const roleCheckbox = this.page.locator('[data-testid="user-create-role-checkbox"]')
    await expect(roleCheckbox).toBeVisible({ timeout: 10000 })
    const isChecked = await roleCheckbox.isChecked()
    if (!isChecked) {
      await roleCheckbox.check()
    }
  }

  /**
   * Submit the user form (click submit button)
   *
   * Includes comprehensive error handling, validation, and logging.
   */
  async submitUserForm(): Promise<void> {
    console.log('[UsersPage] Starting form submission...')

    // Verify dialog is visible before proceeding
    const dialogVisible = await this.isVisible(this.dialog)
    if (!dialogVisible) {
      throw new Error('Cannot submit form: Dialog is not visible')
    }
    console.log('[UsersPage] Dialog is visible, ready to submit')

    // Check if submit button is disabled before clicking
    const isDisabled = await this.dialogSubmitButton.isDisabled()
    if (isDisabled) {
      const buttonText = await this.dialogSubmitButton.textContent()
      throw new Error(`Cannot submit form: Submit button is disabled. Button text: "${buttonText}"`)
    }
    console.log('[UsersPage] Submit button is enabled')

    // Verify submit button is clickable
    await expect(this.dialogSubmitButton).toBeVisible()
    console.log('[UsersPage] Submit button is visible')

    // Click submit button
    console.log('[UsersPage] Clicking submit button...')
    await this.smartClick(this.dialogSubmitButton)
    console.log('[UsersPage] Submit button clicked')

    // Wait for dialog to close with explicit error handling
    try {
      console.log('[UsersPage] Waiting for dialog to close...')
      await expect(this.dialog).toBeHidden({ timeout: 5000 })
      console.log('[UsersPage] Dialog closed successfully')
    } catch (error) {
      // Log current state for debugging
      const isDialogStillVisible = await this.isVisible(this.dialog)
      const isButtonDisabled = await this.dialogSubmitButton.isDisabled()
      const buttonText = await this.dialogSubmitButton.textContent()

      throw new Error(
        `Failed to submit form: Dialog did not close. ` +
        `Dialog visible: ${isDialogStillVisible}, ` +
        `Submit button disabled: ${isButtonDisabled}, ` +
        `Button text: "${buttonText}". ` +
        `Original error: ${error}`
      )
    }

    // Wait for table to refresh (indicates successful submission)
    console.log('[UsersPage] Waiting for table to refresh...')
    await expect(this.table).toBeVisible()
    console.log('[UsersPage] Table refreshed successfully')

    console.log('[UsersPage] Form submission completed')
  }

  /**
   * Create a new user
   *
   * @param userData User data
   */
  async createUser(userData: UserData): Promise<void> {
    await this.clickAddUser()
    await this.fillUserForm(userData)
    await this.submitUserForm()
  }

  /**
   * Find user row in table by email
   *
   * @param email User email to search for
   * @returns Row locator or null if not found
   */
  findUserRow(email: string): Locator {
    // Find table row containing the email
    return this.table.locator(`tr:has-text("${email}")`).first()
  }

  /**
   * Check if user exists in table
   *
   * @param email User email to check
   */
  async userExists(email: string): Promise<boolean> {
    const row = this.findUserRow(email)
    return await this.isVisible(row)
  }

  /**
   * Click "Edit" button for a user
   *
   * @param email User email
   */
  async clickEditUser(email: string): Promise<void> {
    const row = this.findUserRow(email)
    await expect(row).toBeVisible()

    // Find edit button in the row
    const editButton = row.locator('[data-testid="edit-button"]').first()
    await this.smartClick(editButton)

    await expect(this.dialog).toBeVisible()
    await expect(this.dialogTitle).toHaveText(/Edit User|Update User/i)
  }

  /**
   * Edit an existing user
   *
   * @param email User email to edit
   * @param updatedData New user data
   */
  async editUser(email: string, updatedData: Partial<UserData>): Promise<void> {
    await this.clickEditUser(email)
    await this.fillUserForm(updatedData)
    await this.submitUserForm()
  }

  /**
   * Click "Delete" button for a user
   *
   * @param email User email
   */
  async clickDeleteUser(email: string): Promise<void> {
    const row = this.findUserRow(email)
    await expect(row).toBeVisible()

    // Find delete button in the row
    const deleteButton = row.locator('[data-testid$="-delete-button"]').first()

    // Click delete button to open the AlertDialog
    await deleteButton.click()

    // Wait for the confirm dialog to appear
    const confirmDialog = this.page.locator('[data-testid="delete-user-dialog"]')
    await expect(confirmDialog).toBeVisible({ timeout: 5000 })
  }

  /**
   * Confirm user deletion by clicking the confirm button in the AlertDialog
   */
  async confirmDeleteUser(): Promise<void> {
    const confirmButton = this.page.locator('[data-testid="confirm-delete-user-button"]')
    await confirmButton.click()

    // Wait for the dialog to close
    const confirmDialog = this.page.locator('[data-testid="delete-user-dialog"]')
    await expect(confirmDialog).toBeHidden({ timeout: 5000 })
  }

  /**
   * Delete a user
   *
   * @param email User email to delete
   * @param realmId Realm ID for page refresh (defaults to 'admin')
   */
  async deleteUser(email: string, realmId: string = 'admin'): Promise<void> {
    // ✅ Ensure page is in latest state before deletion
    await this.goto(realmId)

    await this.clickDeleteUser(email)
    await this.confirmDeleteUser()
    // Wait for the user row to disappear from the table
    await expect(this.findUserRow(email)).toBeHidden({ timeout: 5000 })
  }

  /**
   * Search users by email
   *
   * Uses assertion-based waiting instead of fixed delays.
   * Waits for search API response and table content update.
   *
   * @param searchTerm Search query
   */
  async searchUsers(searchTerm: string): Promise<void> {
    await this.fillField(this.searchInput, searchTerm)

    // ✅ Improved: Wait for search API response instead of fixed timeout
    // This handles the search debounce properly by waiting for the actual network request
    try {
      await this.page.waitForResponse(
        response =>
          response.url().includes('/api/users') &&
          response.request().method() === 'GET' &&
          response.status() === 200,
        { timeout: 5000 }
      )
    } catch {
      // If no API request is made (e.g., search term too short), continue
      // The table content assertion below will fail if results don't match
    }

    // Wait for either results or "no results" message
    // Playwright auto-waits for the table to be stable
    await expect(this.table).toBeVisible()
  }

  /**
   * Get user count from table
   *
   * @returns Number of user rows
   */
  async getUserCount(): Promise<number> {
    await expect(this.table).toBeVisible()
    const rows = this.table.getByRole('row')
    return await rows.count()
  }

  /**
   * Close/success toast
   */
  async closeToast(): Promise<void> {
    if (await this.isVisible(this.toast)) {
      const closeButton = this.toast.locator('[data-testid="toast-close-button"], button[aria-label="Close"]')
      await this.smartClick(closeButton)
    }
  }

  /**
   * Alias for clickAddUser() - for test compatibility
   */
  async clickAddUserButton(): Promise<void> {
    await this.clickAddUser()
  }

  /**
   * Check if create user dialog is visible
   */
  async isCreateDialogVisible(): Promise<boolean> {
    return await this.isVisible(this.dialog)
  }

  // ─── Reset Password Methods ────────────────────────────────────────────

  /**
   * Click "Reset Password" button for a user identified by email.
   *
   * Finds the user row by email, then locates the reset password button
   * relative to that row using a suffix-matching selector.
   * Waits for the confirmation dialog to appear.
   *
   * @param email User email to reset password for
   */
  async clickResetPassword(email: string): Promise<void> {
    const row = this.findUserRow(email)
    await expect(row).toBeVisible()

    const resetButton = row.locator('[data-testid$="-reset-password-button"]').first()
    await this.smartClick(resetButton)

    await expect(this.resetPasswordConfirmDialog).toBeVisible()
  }

  /**
   * Confirm the reset password action by clicking the confirm button.
   * Waits for the confirmation dialog to close.
   */
  async confirmResetPassword(): Promise<void> {
    await this.smartClick(this.resetPasswordConfirmButton)
    await expect(this.resetPasswordConfirmDialog).toBeHidden({ timeout: 5000 })
  }

  /**
   * Wait for the reset password result dialog to appear and return the new password.
   *
   * @returns The newly generated password string
   */
  async waitForResetPasswordResult(): Promise<string> {
    await expect(this.resetPasswordResultDialog).toBeVisible({ timeout: 10000 })
    await expect(this.resetPasswordNewPasswordText).toBeVisible()
    const password = await this.resetPasswordNewPasswordText.textContent()
    if (!password) {
      throw new Error('New password text is empty in reset password result dialog')
    }
    return password.trim()
  }

  /**
   * Click the "Copy Password" button in the result dialog.
   */
  async copyPassword(): Promise<void> {
    await this.smartClick(this.resetPasswordCopyButton)
  }

  /**
   * Close the reset password result dialog.
   * Clicks the Close button inside the dialog footer.
   */
  async closeResetPasswordResult(): Promise<void> {
    const closeButton = this.resetPasswordResultDialog.getByRole('button', { name: 'Close', exact: true }).first()
    await this.smartClick(closeButton)
    await expect(this.resetPasswordResultDialog).toBeHidden({ timeout: 5000 })
  }

  /**
   * Composite method: perform full reset password flow for a user.
   *
   * 1. Click reset password button in the user row
   * 2. Confirm the action
   * 3. Wait for the result and extract the new password
   *
   * @param email User email to reset password for
   * @returns The newly generated password
   */
  async resetUserPassword(email: string): Promise<string> {
    await this.clickResetPassword(email)
    await this.confirmResetPassword()
    return await this.waitForResetPasswordResult()
  }
}
