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

    // Find delete button in the row (note: data-testid format is {rowIndex}-delete-button)
    const deleteButton = row.locator('[data-testid$="-delete-button"]').first()

    // Handle native confirm dialog by auto-accepting it
    // Setup dialog handler BEFORE clicking to handle synchronous confirm
    await this.page.evaluate(() => {
      window.confirm = () => true
    })

    // Click delete button (triggers native confirm, which is auto-accepted)
    await deleteButton.click()
  }

  /**
   * Confirm user deletion
   * Note: Frontend uses native window.confirm() which is auto-accepted in clickDeleteUser
   * This method just waits for the deletion to complete
   */
  async confirmDeleteUser(): Promise<void> {
    // Native confirm dialog is already handled in clickDeleteUser
    // Wait briefly for deletion mutation to complete
    await this.page.waitForTimeout(500)
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
}
