/**
 * Login Page Object
 *
 * Encapsulates login page operations.
 * Provides methods for user authentication.
 *
 * @see ../../../spec/demo/e2e-testing.md#page-object-model-pom-规范
 */

import { Page, Locator, expect } from '@playwright/test'
import { SELECTORS } from '../selectors'
import { BasePage } from './base-page'
import type { UnifiedLogger } from '../helpers/unified-logger'

/**
 * Login credentials interface
 */
export interface LoginCredentials {
  email: string
  password: string
}

/**
 * Login Page Object
 *
 * Represents the login page at /{realmId}/auth/login
 *
 * @example
 * ```typescript
 * const loginPage = new LoginPage(page, logger)
 * await loginPage.goto()
 * await loginPage.login({ email: 'admin@cas.com', password: 'password' })
 * ```
 */
export class LoginPage extends BasePage {
  // Selectors
  readonly container: Locator
  readonly title: Locator
  readonly emailInput: Locator
  readonly passwordInput: Locator
  readonly submitButton: Locator
  readonly errorMessage: Locator

  constructor(page: Page, logger?: UnifiedLogger) {
    super(page, logger)
    this.container = page.locator(SELECTORS.login.container)
    this.title = page.locator(SELECTORS.login.title)
    this.emailInput = page.locator(SELECTORS.login.emailInput)
    this.passwordInput = page.locator(SELECTORS.login.passwordInput)
    this.submitButton = page.locator(SELECTORS.login.submitButton)
    this.errorMessage = page.locator(SELECTORS.login.errorMessage)
  }

  /**
   * Navigate to login page
   *
   * @param realmId Realm ID (default: 'admin')
   * @param clientId Optional client ID for the login (default: undefined, uses frontend default)
   */
  async goto(realmId: string = 'admin', clientId?: string): Promise<void> {
    let url = `/${realmId}/auth/login`
    if (clientId) {
      url += `?clientId=${encodeURIComponent(clientId)}`
    }
    const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'
    await this.page.goto(`${BASE_URL}${url}`, { waitUntil: 'domcontentloaded' })

    // Wait a moment for the page to fully load and process any redirects
    await this.page.waitForTimeout(500)

    // Wait for navigation to complete (check if redirected to dashboard/profile/realm)
    const currentUrl = this.page.url()
    console.log(`[LoginPage] Current URL: ${currentUrl}`)

    // Check if we've been redirected to a dashboard/profile page (user is already logged in)
    // Support URLs with or without query parameters (e.g., /admin, /admin/, /admin?param=value)
    const isRedirectedToDashboard = currentUrl.includes(`/${realmId}/manage`)
    const isRedirectedToProfile = currentUrl.includes(`/${realmId}/user/profile`)
    const isRedirectedToRealm = currentUrl.match(new RegExp(`/${realmId}/?(\\?.*)?$`)) !== null

    console.log(`[LoginPage] Dashboard: ${isRedirectedToDashboard}, Profile: ${isRedirectedToProfile}, Realm: ${isRedirectedToRealm}`)

    if (isRedirectedToDashboard || isRedirectedToProfile || isRedirectedToRealm) {
      console.log(`[LoginPage] Already logged in, redirected to: ${currentUrl}`)
      return
    }

    // Wait for login page to load
    await expect(this.container).toBeVisible()
  }

  /**
   * Wait for login page to be visible
   */
  async waitForReady(): Promise<void> {
    await expect(this.container).toBeVisible()
    await expect(this.title).toBeVisible()
    await expect(this.emailInput).toBeVisible()
    await expect(this.passwordInput).toBeVisible()
    await expect(this.submitButton).toBeVisible()
  }

  /**
   * Fill login form
   *
   * @param credentials Login credentials
   */
  async fillLoginForm(credentials: LoginCredentials): Promise<void> {
    await this.fillField(this.emailInput, credentials.email)
    await this.fillField(this.passwordInput, credentials.password)
  }

  /**
   * Submit login form
   */
  async submit(): Promise<void> {
    await this.smartClick(this.submitButton)
    // Note: Don't wait for networkidle here - it's handled in login() method
  }

  /**
   * Login with credentials
   *
   * @param credentials Login credentials
   */
  async login(credentials: LoginCredentials): Promise<void> {
    await this.fillLoginForm(credentials)
    await this.submit()

    // Wait for the login API response so the Bearer token is stored before proceeding.
    const loginResponse = await this.page.waitForResponse(
      response => response.url().includes('/login') && response.request().method() === 'POST',
      { timeout: 10000 }
    ).catch(() => null)

    if (loginResponse && !loginResponse.ok()) {
      const errorBody = await loginResponse.text().catch(() => 'Unable to read error body')
      console.error(`[LoginPage] Login API failed: ${loginResponse.status()} - ${errorBody}`)
      throw new Error(`Login failed: API returned ${loginResponse.status()}`)
    }

    if (loginResponse) {
      console.log(`[LoginPage] Login API response status: ${loginResponse.status()}`)
    }
  }

  /**
   * Login as admin
   *
   * Always performs a fresh login by clearing cookies first.
   * This prevents stale authentication state from causing navigation failures.
   *
   * @param email Admin email (default: 'admin@cas.com')
   * @param password Admin password (default: 'password')
   * @param realmId Realm ID (default: 'admin')
   * @returns The userId from the login response
   */
  async loginAsAdmin(
    email: string = 'admin@cas.com',
    password: string = 'password',
    realmId: string = 'admin'
  ): Promise<string> {
    // Always clear cookies before login to ensure fresh authentication
    // This prevents stale auth state from causing navigation failures
    await this.page.context().clearCookies()
    console.log(`[LoginPage] Cookies cleared for fresh authentication`)

    console.log(`[LoginPage] Logging in as ${email} to realm ${realmId}`)

    // Navigate to login page and login
    await this.goto(realmId)

    // Wait for login API response to capture userId
    const loginResponsePromise = this.page.waitForResponse(
      response => response.url().includes('/login') && response.request().method() === 'POST',
      { timeout: 10000 }
    )

    await this.login({ email, password })

    // Get userId from login response
    const loginResponse = await loginResponsePromise
    if (!loginResponse.ok()) {
      const errorBody = await loginResponse.text().catch(() => 'Unable to read error body')
      console.error(`[LoginPage] Login API failed: ${loginResponse.status()} - ${errorBody}`)
      throw new Error(`Login failed: API returned ${loginResponse.status()}`)
    }

    const loginData = await loginResponse.json()
    const userId = loginData.userId
    console.log(`[LoginPage] Login successful, userId: ${userId}`)

    // Verify successful login - wait for navigation to dashboard
    // Match either /admin/ for admin realm or /{realmId} for new realms
    await this.page.waitForURL(new RegExp(`.*${realmId}`), { timeout: 15000 })

    // Verify the browser-token session was established. Since commit f3b8d48a
    // replaced the X-Auth session cookie with the browser Bearer token model,
    // there is no cookie to check; the rotating refresh token is persisted in
    // localStorage under `auth-storage` (frontend Zustand `persist` store,
    // auth-store.ts:203 / auth-constants.ts:130). Its presence is the persistent
    // proof that the frontend completed login + PKCE token exchange.
    const authStorage = await this.page.evaluate(() => window.localStorage.getItem('auth-storage'))
    if (!authStorage) {
      throw new Error(`[LoginPage] Login failed: auth-storage not found in localStorage after login`)
    }

    console.log(`[LoginPage] auth-storage persisted (length=${authStorage.length})`)

    // Verify successful login - should be redirected to admin page
    await expect(this.page).toHaveURL(new RegExp(`^http://localhost:3000/${realmId}(/|$)`))

    return userId
  }

  /**
   * Login as a regular user in the target realm.
   *
   * Unlike admin login, regular users are expected to land on user-facing routes
   * such as profile or points pages instead of admin console routes.
   *
   * @param email User email
   * @param password User password
   * @param realmId Realm ID
   * @param clientId Optional client ID (default: undefined, uses frontend default)
   */
  async loginAsUser(
    email: string,
    password: string,
    realmId: string,
    clientId?: string
  ): Promise<void> {
    await this.page.context().clearCookies()
    console.log(`[LoginPage] Cookies cleared for fresh user authentication`)

    console.log(`[LoginPage] Logging in as user ${email} to realm ${realmId}`)

    await this.goto(realmId, clientId)
    await this.login({ email, password })

    await this.page.waitForURL(
      new RegExp(`^http://localhost:3000/${realmId}(/|\\?|$)`),
      { timeout: 15000 }
    )

    // Browser Bearer token model (commit f3b8d48a): no X-Auth cookie. The
    // rotating refresh token is persisted in localStorage under `auth-storage`
    // (frontend Zustand `persist` store). Its presence proves the session was
    // established. See loginAsAdmin for the full rationale.
    const authStorage = await this.page.evaluate(() => window.localStorage.getItem('auth-storage'))
    if (!authStorage) {
      throw new Error(`[LoginPage] User login failed: auth-storage not found in localStorage after login`)
    }

    console.log(`[LoginPage] User login successful, auth-storage persisted (length=${authStorage.length})`)
  }

  /**
   * Get error message text
   *
   * @returns Error message text (empty string if no error)
   */
  async getErrorMessage(): Promise<string> {
    const visible = await this.isVisible(this.errorMessage)
    if (!visible) return ''
    return await this.getText(this.errorMessage)
  }

  /**
   * Check if error message is visible
   *
   * @returns true if error message is visible
   */
  async hasError(): Promise<boolean> {
    return await this.isVisible(this.errorMessage)
  }

  /**
   * Verify we are on the login page
   */
  async isOnLoginPage(): Promise<boolean> {
    const url = this.getUrl()
    return url.includes('/login') && await this.isVisible(this.container)
  }
}
