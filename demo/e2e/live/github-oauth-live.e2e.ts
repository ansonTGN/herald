/**
 * Live GitHub OAuth Full Flow Test (Manual-Assisted)
 *
 * End-to-end test that validates the complete GitHub OAuth login flow.
 * The test automates all setup and initiates the OAuth redirect,
 * then pauses for manual GitHub login (CAPTCHA / 2FA cannot be automated),
 * and finally verifies the callback result.
 *
 * Skips gracefully when credentials are absent.
 *
 * =============================================================================
 * HOW TO RUN
 * =============================================================================
 *
 * 1. Start backend and frontend
 *
 *    # Terminal 1: backend (port 8080)
 *    cd backend && cargo run
 *
 *    # Terminal 2: frontend (port 3000)
 *    cd frontend && npm run dev
 *
 * 2. Make sure demo/.env.demo has real GitHub OAuth credentials:
 *
 *    GITHUB_CLIENT_ID=Ov23li...
 *    GITHUB_CLIENT_SECRET=9227...
 *
 *    Get these from https://github.com/settings/developers
 *    The GitHub OAuth App's "Authorization callback URL" must be:
 *      http://localhost:8080/api/oauth/{realmId}/github/callback
 *    And backend config.toml's public_base_url must match:
 *      public_base_url = "http://localhost:8080"
 *
 *    If you need GitHub to reach your local machine (e.g. remote server),
 *    use ngrok:
 *      uv run scripts/ngrok-tunnel.py --port 8080
 *    Then set public_base_url to the ngrok URL and update the GitHub
 *    OAuth App's callback URL accordingly.
 *
 * 3. Run this test in headed mode (so you can see and operate the browser):
 *
 *    cd demo
 *    npx playwright test e2e/live/github-oauth-live.e2e.ts --project=demo-fast --headed
 *
 *    Or with Playwright UI for better visibility:
 *    npx playwright test e2e/live/github-oauth-live.e2e.ts --ui
 *
 * 4. When the test pauses on the GitHub login page, manually:
 *    a) Log in with your GitHub account
 *    b) Handle CAPTCHA / 2FA if prompted
 *    c) Click "Authorize" on the consent screen
 *
 *    The test will auto-continue once the callback returns.
 *    You have up to 2 minutes for the manual step.
 *
 * =============================================================================
 */

import { test, expect } from '@playwright/test'
import { secrets, hasGitHubOAuth } from '../secrets/env'
import { seedOAuthConfig } from '../secrets/realm-seed'
import { loginAsAdmin, logout } from '../helpers/auth'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'
const REALM_ID = 'admin'

test.describe('Live: GitHub OAuth Full Flow', () => {
  test.beforeEach(async ({ page }) => {
    test.skip(!hasGitHubOAuth(), 'GitHub OAuth credentials not configured in .env.demo')

    // Step 1: Login as admin and seed GitHub OAuth config via API
    await test.step('Setup: seed GitHub OAuth config', async () => {
      await loginAsAdmin(page, { realmId: REALM_ID })
      await seedOAuthConfig(page.request, REALM_ID, {
        providerType: 'github',
        clientId: secrets.github.clientId!,
        clientSecret: secrets.github.clientSecret!,
        scopes: ['user:email'],
        enabled: true,
      })
      console.log('[setup] GitHub OAuth config seeded')

      // Logout so the OAuth flow starts from the login page
      await logout(page)
    })
  })

  test.afterEach(async ({ page }) => {
    // Cleanup: delete the GitHub OAuth config
    try {
      await loginAsAdmin(page, { realmId: REALM_ID })
      const response = await page.request.delete(
        `${BASE_URL}/api/oauth/${REALM_ID}/configs/github`,
      )
      if (response.ok()) {
        console.log('[cleanup] GitHub OAuth config deleted')
      } else {
        console.log(`[cleanup] GitHub OAuth config delete returned ${response.status()}`)
      }
    } catch (error) {
      console.error('[cleanup] Failed to clean up GitHub OAuth config:', error)
    }
  })

  test('complete GitHub OAuth login with manual authorization', async ({ page }) => {
    // Step 2: Navigate to login page and verify GitHub button exists
    await test.step('Given the login page with GitHub OAuth enabled', async () => {
      await page.goto(`${BASE_URL}/${REALM_ID}/auth/login`, {
        waitUntil: 'domcontentloaded',
      })
      await page.waitForLoadState('networkidle')

      const githubButton = page.getByTestId('oauth-login-button-github')
      await expect(githubButton).toBeVisible({ timeout: 10000 })
      console.log('[step-2] GitHub OAuth button is visible on login page')
    })

    // Step 3: Click GitHub button, validate redirect URL, then pause for manual login
    await test.step('When clicking GitHub OAuth button and authorizing on GitHub', async () => {
      // Intercept the API response that returns the auth URL
      const loginResponsePromise = page.waitForResponse(
        (resp) =>
          resp.url().includes(`/api/oauth/${REALM_ID}/github/login`) &&
          resp.request().method() === 'GET',
        { timeout: 15000 },
      )

      const githubButton = page.getByTestId('oauth-login-button-github')
      await githubButton.click()

      const loginResponse = await loginResponsePromise
      expect(loginResponse.ok()).toBeTruthy()

      const responseBody = await loginResponse.json()
      const authUrl: string = responseBody.authUrl
      const stateToken: string = responseBody.state

      // Validate the auth URL structure
      expect(authUrl).toContain('github.com/login/oauth/authorize')
      const urlObj = new URL(authUrl)
      expect(urlObj.searchParams.get('client_id')).toBe(secrets.github.clientId)
      expect(urlObj.searchParams.get('state')).toBe(stateToken)
      expect(urlObj.searchParams.get('scope')).toContain('user:email')
      console.log(`[step-3] Auth URL validated: ${authUrl}`)

      // Wait for the redirect to GitHub
      await page.waitForURL('**github.com/**', { timeout: 15000 })
      console.log(`[step-3] Redirected to: ${page.url()}`)

      // >>> MANUAL INTERVENTION REQUIRED <<<
      // The test pauses here. In the opened browser window:
      //   1. Log in to GitHub (handle CAPTCHA / 2FA if prompted)
      //   2. Click "Authorize" on the consent screen
      //   3. The test will automatically continue after the callback
      console.log('')
      console.log('========================================')
      console.log('  PLEASE COMPLETE GITHUB LOGIN')
      console.log('  in the browser window...')
      console.log('========================================')
      console.log('')
    })

    // Step 4: Wait for the OAuth callback to complete and verify the result
    await test.step('Then the OAuth callback succeeds and user is logged in', async () => {
      // Wait for navigation away from GitHub (back to our app after callback)
      // The frontend handles the callback response and sets the auth token
      await page.waitForURL(
        (url) => !url.toString().includes('github.com'),
        { timeout: 120_000 }, // 2 minutes for manual login
      )

      const currentUrl = page.url()
      console.log(`[step-4] Callback completed, current URL: ${currentUrl}`)

      // Verify we're back on our app and authenticated
      // The frontend should redirect to an authenticated page after successful OAuth
      expect(currentUrl).toContain(REALM_ID)
      expect(currentUrl).not.toContain('/auth/login')

      // Verify we have auth cookies / tokens
      const cookies = await page.context().cookies()
      const hasAuthCookie = cookies.some(
        (c) => c.name === 'X-Auth' || c.name === 'auth_token' || c.name === 'token',
      )

      if (hasAuthCookie) {
        console.log('[step-4] Auth cookie found - OAuth login successful')
      } else {
        // Check localStorage as fallback
        const hasLocalToken = await page.evaluate(() => {
          return !!(
            localStorage.getItem('auth_token') ||
            localStorage.getItem('token') ||
            localStorage.getItem('accessToken')
          )
        })
        expect(hasLocalToken).toBeTruthy()
        console.log('[step-4] Auth token in localStorage - OAuth login successful')
      }

      console.log('[step-4] Full GitHub OAuth flow completed successfully')
    })
  })
})
