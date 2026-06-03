/**
 * Reset Password Demo Tests
 *
 * User Story: docs/prd/core/users.md
 * - Admin can reset a user's password from the user list
 * - After reset, the new password is displayed in a result dialog
 * - Admin can copy the new password and close the dialog
 * - Cancel path dismisses the confirmation dialog without resetting
 *
 * Selector calibration: All selectors verified against
 *   frontend/src/components/users/user-table.tsx (row-level reset button)
 *   frontend/src/routes/$realmId/manage/users.tsx (confirm dialog)
 *   frontend/src/components/users/reset-password-result-dialog.tsx (result dialog)
 */

import { test, expect, cleanupTestData } from '../fixtures/demo-page.fixtures'

test.describe('[ResetPassword] Admin resets user password', () => {
  const testUserEmail = `reset-pw-test@demo.com`

  test.afterEach(async ({ usersPage, testStartTime }) => {
    await cleanupTestData(usersPage.page, 'admin', {
      timestamp: testStartTime,
      keepUsers: [],
    })
  })

  test('should reset password and show new password in result dialog', async ({ usersPage }) => {
    // Given: a user exists in the admin realm
    await test.step('Given a test user exists', async () => {
      await usersPage.createUser({
        email: testUserEmail,
        password: 'TestPass123!',
        nickname: 'resetpw',
      })
      await expect(usersPage.findUserRow(testUserEmail)).toBeVisible()
    })

    // When: admin clicks Reset Password button
    let newPassword = ''
    await test.step('When admin clicks Reset Password and confirms', async () => {
      await usersPage.clickResetPassword(testUserEmail)

      // Then: confirmation dialog appears
      await expect(usersPage.resetPasswordConfirmDialog).toBeVisible()

      // Click confirm
      await usersPage.confirmResetPassword()
    })

    // Then: result dialog appears with a new password
    await test.step('Then result dialog shows new password', async () => {
      await expect(usersPage.resetPasswordResultDialog).toBeVisible()

      newPassword = await usersPage.waitForResetPasswordResult()

      // Password should be non-empty and at least 16 chars (backend generates 16-char passwords)
      expect(newPassword.length).toBeGreaterThanOrEqual(16)
      expect(newPassword).toMatch(/[A-Z]/)     // has uppercase
      expect(newPassword).toMatch(/[a-z]/)     // has lowercase
      expect(newPassword).toMatch(/[0-9]/)     // has digit
    })

    // And: admin can copy the password
    await test.step('And admin can copy the password', async () => {
      await usersPage.copyPassword()

      // Verify button text changed to "Copied!" -- this is a stable DOM assertion, not a toast
      await expect(usersPage.resetPasswordCopyButton).toHaveText(/Copied/)
    })

    // And: admin can close the result dialog
    await test.step('And admin can close the result dialog', async () => {
      await usersPage.closeResetPasswordResult()
      await expect(usersPage.resetPasswordResultDialog).toBeHidden()
    })
  })

  test('should cancel reset password without changing password', async ({ usersPage }) => {
    // Given: a user exists
    await test.step('Given a test user exists', async () => {
      await usersPage.createUser({
        email: testUserEmail,
        password: 'TestPass123!',
        nickname: 'cancel-reset',
      })
      await expect(usersPage.findUserRow(testUserEmail)).toBeVisible()
    })

    // When: admin clicks Reset Password then cancels
    await test.step('When admin clicks Reset Password then cancels', async () => {
      await usersPage.clickResetPassword(testUserEmail)
      await expect(usersPage.resetPasswordConfirmDialog).toBeVisible()

      // Cancel by pressing Escape (AlertDialog supports Escape to dismiss)
      await usersPage.page.keyboard.press('Escape')
    })

    // Then: dialog closes, no result dialog appears
    await test.step('Then confirmation dialog closes without result dialog', async () => {
      await expect(usersPage.resetPasswordConfirmDialog).toBeHidden()
      // Result dialog should NOT appear
      await expect(usersPage.resetPasswordResultDialog).toBeHidden()
    })
  })
})
