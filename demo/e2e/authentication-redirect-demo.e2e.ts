/**
 * Authentication Redirect Flow Demo Test
 *
 * Test Coverage:
 * - Scenario 1: Unauthenticated user accessing root URL
 * - Scenario 2: Unauthenticated user accessing protected route
 * - Scenario 3: Admin user login redirect
 * - Scenario 4: Regular user login redirect
 * - Scenario 5: Admin user accessing realm root
 * - Scenario 6: Regular user accessing realm root
 * - Scenario 7: Regular user accessing admin dashboard (permission denied)
 * - Scenario 8: Logout and redirect
 *
 * @see docs/user-stories/03-regular-user-user-stories.md#US-RU-009
 * @note Uses the 'admin' realm for all test scenarios
 */

import { test, expect, cleanupTestData } from './fixtures/demo-page.fixtures'
import { SELECTORS } from './selectors'
import { verifyTestEnvironment } from './helpers/environment-setup'
import { logout } from './helpers/auth'
import { UsersPage } from './pages/users-page'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'
const REALM_ID = 'admin' // Using existing admin realm as confirmed

test.describe('Authentication Redirect Flow', () => {
  let testStartTime: number

  test.beforeEach(async ({ page, demoLogger, testStartTime: startTime }) => {
    testStartTime = startTime

    // Clear cookies for clean state
    await page.context().clearCookies()

    // Verify test environment before each test
    await verifyTestEnvironment(page, {
      requiredRealms: ['admin'],
      requiredUsers: ['admin@cas.com'],
    })
  })

  test.afterEach(async ({ page, demoLogger }) => {
    // Use standard cleanup function
    await cleanupTestData(page, REALM_ID, {
      keepUsers: ['admin@cas.com'],
      timestamp: testStartTime,
    })
  })

  // Scenario 1: Unauthenticated user accessing root URL
  test('Scenario 1: Unauthenticated user accessing root URL redirects to login', async ({ page }) => {
    await test.step('Access root URL', async () => {
      await page.goto(`${BASE_URL}/`)
    })

    // Should redirect to admin realm login page
    await expect(page).toHaveURL(new RegExp(`\\/${REALM_ID}\\/auth\\/login`))

    // Verify login page is visible
    await expect(page.locator(SELECTORS.login.container)).toBeVisible()
    await expect(page.locator(SELECTORS.login.emailInput)).toBeVisible()
    await expect(page.locator(SELECTORS.login.passwordInput)).toBeVisible()
  })

  // Scenario 2: Unauthenticated user accessing protected route
  test('Scenario 2: Unauthenticated user accessing protected route redirects to login', async ({ page }) => {
    await test.step('Access protected route (manage page)', async () => {
      await page.goto(`${BASE_URL}/${REALM_ID}/manage`)
    })

    // Should redirect to login page
    await expect(page).toHaveURL(new RegExp(`\\/${REALM_ID}\\/auth\\/login`))

    // Check redirect parameter is preserved
    const url = page.url()
    expect(url).toContain('redirect=')

    // Verify the redirect parameter points to the relative path (without realm prefix)
    expect(url).toContain('redirect=%2Fmanage')

    // Verify login page is visible
    await expect(page.locator(SELECTORS.login.container)).toBeVisible()
  })

  // Scenario 3: Admin user login redirect
  test('Scenario 3: Admin user login redirects to manage dashboard', async ({ page, loginPage }) => {
    await test.step('Navigate to login page', async () => {
      await page.goto(`${BASE_URL}/${REALM_ID}/auth/login`)
    })

    await test.step('Login as admin', async () => {
      await loginPage.loginAsAdmin('admin@cas.com', 'password', REALM_ID)
    })

    // Should be redirected to manage dashboard - use assertion wait
    await expect(page).toHaveURL(new RegExp(`\\/${REALM_ID}\\/manage`), { timeout: 10000 })
  })

  // Scenario 4: Regular user login redirect
  test('Scenario 4: Regular user login redirects to user profile', async ({ page, loginPage, usersPage, demoLogger }) => {
    const regularUserEmail = `regularuser${testStartTime}@example.com`

    await test.step('Login as admin and create regular user', async () => {
      await loginPage.loginAsAdmin('admin@cas.com', 'password', REALM_ID)

      await usersPage.goto(REALM_ID)

      // Create a regular user (no admin permissions)
      await usersPage.clickAddUser()
      await usersPage.fillUserForm({
        email: regularUserEmail,
        password: 'User123456!',
        nickname: `Regular User ${testStartTime}`
      })

      // Select the "User" role (required field)
      const userRoleCheckbox = page.locator('label:text("User")').first()
      await userRoleCheckbox.check()

      await usersPage.submitUserForm()
      demoLogger.testCode.info(`Created regular user: ${regularUserEmail}`)
    })

    await test.step('Logout and login as regular user', async () => {
      await logout(page)

      await loginPage.goto(REALM_ID)
      await loginPage.login({
        email: regularUserEmail,
        password: 'User123456!'
      })
    })

    // Should be redirected to user profile page
    await expect(page).toHaveURL(new RegExp(`\\/${REALM_ID}\\/user\\/profile`), { timeout: 10000 })

    // Verify profile page is loaded
    await expect(page.locator('h3:text("Profile Information")')).toBeVisible()
  })

  // Scenario 5: Admin user accessing realm root
  test('Scenario 5: Admin user accessing realm root redirects to manage', async ({ page, loginPage }) => {
    await test.step('Login as admin', async () => {
      await loginPage.goto(REALM_ID)
      await loginPage.loginAsAdmin('admin@cas.com', 'password', REALM_ID)
    })

    await test.step('Navigate to realm root', async () => {
      await page.goto(`${BASE_URL}/${REALM_ID}`)
    })

    // Should redirect to manage dashboard
    await expect(page).toHaveURL(new RegExp(`\\/${REALM_ID}\\/manage`), { timeout: 10000 })
  })

  // Scenario 6: Regular user accessing realm root
  test('Scenario 6: Regular user accessing realm root redirects to profile', async ({ page, loginPage, usersPage, demoLogger }) => {
    const testUserEmail = `testuser${testStartTime}@example.com`

    await test.step('Login as admin and create regular user', async () => {
      await loginPage.goto(REALM_ID)
      await loginPage.loginAsAdmin('admin@cas.com', 'password', REALM_ID)

      await usersPage.goto(REALM_ID)

      // Create a regular user
      await usersPage.clickAddUser()
      await usersPage.fillUserForm({
        email: testUserEmail,
        password: 'User123456!',
        nickname: `Test User ${testStartTime}`
      })

      // Select the "User" role (required field)
      const userRoleCheckbox = page.locator('label:text("User")').first()
      await userRoleCheckbox.check()

      await usersPage.submitUserForm()
      demoLogger.testCode.info(`Created regular user: ${testUserEmail}`)
    })

    await test.step('Logout and login as regular user', async () => {
      await logout(page)

      await loginPage.goto(REALM_ID)
      await loginPage.login({
        email: testUserEmail,
        password: 'User123456!'
      })
    })

    await test.step('Navigate to realm root', async () => {
      await page.goto(`${BASE_URL}/${REALM_ID}`)
    })

    // Should redirect to user profile
    await expect(page).toHaveURL(new RegExp(`\\/${REALM_ID}\\/user\\/profile`), { timeout: 10000 })

    // Verify profile page is loaded
    await expect(page.locator('h3:text("Profile Information")')).toBeVisible()
  })

  // Scenario 6.5: Authenticated regular user accessing root URL
  test('Scenario 6.5: Authenticated regular user accessing root URL redirects to profile', async ({ page, loginPage, usersPage, demoLogger }) => {
    const testUserEmail = `rooturluser${testStartTime}@example.com`

    await test.step('Login as admin and create regular user', async () => {
      await loginPage.goto(REALM_ID)
      await loginPage.loginAsAdmin('admin@cas.com', 'password', REALM_ID)

      await usersPage.goto(REALM_ID)

      // Create a regular user
      await usersPage.clickAddUser()
      await usersPage.fillUserForm({
        email: testUserEmail,
        password: 'User123456!',
        nickname: `Root URL User ${testStartTime}`
      })

      // Select the "User" role (required field)
      const userRoleCheckbox = page.locator('label:text("User")').first()
      await userRoleCheckbox.check()

      await usersPage.submitUserForm()
      demoLogger.testCode.info(`Created regular user: ${testUserEmail}`)
    })

    await test.step('Logout and login as regular user', async () => {
      await logout(page)

      await loginPage.goto(REALM_ID)
      await loginPage.login({
        email: testUserEmail,
        password: 'User123456!'
      })
    })

    await test.step('Access root URL while authenticated', async () => {
      await page.goto(`${BASE_URL}/`)
    })

    // Should redirect to user profile (not login page)
    await expect(page).toHaveURL(new RegExp(`\\/${REALM_ID}\\/user\\/profile`), { timeout: 10000 })

    // Verify profile page is loaded
    await expect(page.locator('h3:text("Profile Information")')).toBeVisible()
  })

  // Scenario 7: Regular user accessing admin dashboard (permission denied)
  test('Scenario 7: Regular user accessing admin dashboard redirects to profile', async ({ page, loginPage, usersPage, demoLogger }) => {
    const noPermissionEmail = `nopermission${testStartTime}@example.com`

    await test.step('Login as admin and create regular user', async () => {
      await loginPage.goto(REALM_ID)
      await loginPage.loginAsAdmin('admin@cas.com', 'password', REALM_ID)

      await usersPage.goto(REALM_ID)

      // Create a regular user
      await usersPage.clickAddUser()
      await usersPage.fillUserForm({
        email: noPermissionEmail,
        password: 'User123456!',
        nickname: `No Permission User ${testStartTime}`
      })

      // Select the "User" role (required field)
      const userRoleCheckbox = page.locator('label:text("User")').first()
      await userRoleCheckbox.check()

      await usersPage.submitUserForm()
      demoLogger.testCode.info(`Created regular user: ${noPermissionEmail}`)
    })

    await test.step('Logout and login as regular user', async () => {
      await logout(page)

      await loginPage.goto(REALM_ID)
      await loginPage.login({
        email: noPermissionEmail,
        password: 'User123456!'
      })
    })

    await test.step('Try to access admin dashboard directly', async () => {
      await page.goto(`${BASE_URL}/${REALM_ID}/manage`)
    })

    // Should be redirected to user profile (permission denied)
    await expect(page).toHaveURL(new RegExp(`\\/${REALM_ID}\\/user\\/profile`), { timeout: 10000 })

    // Verify profile page is loaded
    await expect(page.locator('h3:text("Profile Information")')).toBeVisible()
  })

  // Scenario 8: Logout and redirect
  test('Scenario 8: Logout clears session and redirects to login', async ({ page, loginPage, usersPage, demoLogger }) => {
    // Clear any existing session from previous test (Scenario 7 left regular user logged in)
    await logout(page)

    const logoutTestEmail = `logoutuser${testStartTime}@example.com`

    await test.step('Login as admin and create user', async () => {
      await loginPage.goto(REALM_ID)
      await loginPage.loginAsAdmin('admin@cas.com', 'password', REALM_ID)

      await usersPage.goto(REALM_ID)

      // Create a regular user
      await usersPage.clickAddUser()
      await usersPage.fillUserForm({
        email: logoutTestEmail,
        password: 'User123456!',
        nickname: `Logout Test User ${testStartTime}`
      })

      const userRoleCheckbox = page.locator('label:text("User")').first()
      await userRoleCheckbox.check()
      await usersPage.submitUserForm()

      demoLogger.testCode.info(`Created regular user: ${logoutTestEmail}`)
    })

    await test.step('Login as regular user', async () => {
      // Logout admin user to clear session before logging in as regular user
      await logout(page)

      await loginPage.goto(REALM_ID)
      await loginPage.login({
        email: logoutTestEmail,
        password: 'User123456!'
      })
    })

    await test.step('Verify logged in state', async () => {
      await expect(page.locator('h3:text("Profile Information")')).toBeVisible()
    })

    await test.step('Logout', async () => {
      await logout(page)
    })

    await test.step('Verify redirected to login page', async () => {
      await expect(page).toHaveURL(new RegExp(`\\/${REALM_ID}\\/auth\\/login`), { timeout: 10000 })
      await expect(page.locator(SELECTORS.login.container)).toBeVisible()
    })

    await test.step('Verify cannot access protected route without login', async () => {
      await page.goto(`${BASE_URL}/${REALM_ID}/manage`)
      await expect(page).toHaveURL(new RegExp(`\\/${REALM_ID}\\/auth\\/login`), { timeout: 10000 })
    })
  })

})
