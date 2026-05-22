/**
 * OAuth PKCE Error and Edge Case Demo Tests
 *
 * Test Coverage:
 * - Test 1: Authorization Code Replay (US-TP-006 scenario 1)
 * - Test 2: PKCE Verification Failure (US-TP-006 scenario 2)
 * - Test 3: redirect_uri Not in Whitelist (US-TP-008)
 * - Test 4: Disabled Client App (US-TP-006 scenario 3)
 * - Test 5: Invalid Authorization Code (US-TP-006 scenario 4)
 * - Test 6: Login with Mismatched State (US-TP-006 scenario 5)
 * - Test 7: Partial OAuth Params Display Error (US-RU-010 scenario 2)
 *
 * Each scenario is a separate test() because error tests require
 * different setup state and must not cascade failures.
 *
 * @see docs/user-stories/oauth-third-party-integration.md
 */

import { test, expect, cleanupTestData } from './fixtures/demo-page.fixtures'
import {
  BASE_URL,
  generatePKCEPair,
  oauthAuthorize,
  oauthTokenExchange,
  seedOAuthClientApp,
  completeOAuthLoginAndGetAuthCode,
  blockExternalCallback,
  isLoginApiResponse,
} from './helpers/oauth-helpers'
import { verifyTestEnvironment } from './helpers/environment-setup'
import { DEMO_ADMIN } from './helpers/auth'
import { ClientAppsPage } from './pages/client-apps-page'
import * as crypto from 'node:crypto'

test.describe('[OAuth PKCE] Error and Edge Case Demo Tests', () => {
  let testStartTime: number

  test.beforeEach(async ({ page, testStartTime: startTime }) => {
    testStartTime = startTime

    await verifyTestEnvironment(page, {
      requiredRealms: ['admin'],
      requiredUsers: ['admin@cas.com'],
    })
  })

  test.afterEach(async ({ page }) => {
    await cleanupTestData(page, DEMO_ADMIN.realmId, {
      timestamp: testStartTime,
    })
  })

  // ---------------------------------------------------------------------------
  // Test 1: Authorization Code Replay
  // ---------------------------------------------------------------------------

  test('Authorization code replay fails on second exchange', async ({
    page,
    loginPage,
  }) => {
    const realmId = DEMO_ADMIN.realmId
    const redirectUri = 'https://example.com/oauth/callback'
    const state = crypto.randomUUID()
    const appName = `Replay Test ${Date.now()}`

    let clientId: string

    await test.step('Given: Admin is logged in and OAuth client app is seeded', async () => {
      await loginPage.loginAsAdmin(DEMO_ADMIN.email, DEMO_ADMIN.password, realmId)
      const result = await seedOAuthClientApp(page.request, realmId, {
        appName,
        redirectUris: [redirectUri],
      })
      clientId = result.clientId
    })

    let authCode: string
    let pkce: ReturnType<typeof generatePKCEPair>

    await test.step('And: Full PKCE flow completes producing an auth code', async () => {
      const flowResult = await completeOAuthLoginAndGetAuthCode(
        page, BASE_URL, realmId, clientId, redirectUri, state,
        { email: DEMO_ADMIN.email, password: DEMO_ADMIN.password },
      )
      authCode = flowResult.authCode
      pkce = flowResult.pkce
    })

    await test.step('When: First token exchange succeeds', async () => {
      const firstResult = await oauthTokenExchange(BASE_URL, realmId, {
        grant_type: 'authorization_code',
        code: authCode,
        redirect_uri: redirectUri,
        client_id: clientId,
        code_verifier: pkce.code_verifier,
      })

      expect('access_token' in firstResult).toBe(true)
      const tokenResp = firstResult as { access_token: string }
      expect(tokenResp.access_token).toBeTruthy()
    })

    await test.step('Then: Second token exchange with same code fails', async () => {
      const secondResult = await oauthTokenExchange(BASE_URL, realmId, {
        grant_type: 'authorization_code',
        code: authCode,
        redirect_uri: redirectUri,
        client_id: clientId,
        code_verifier: pkce.code_verifier,
      })

      const errorResp = secondResult as unknown as Record<string, unknown>
      expect('message' in errorResp || 'error' in errorResp).toBe(true)
      const message = (errorResp.message as string) || (errorResp.error as string) || ''
      expect(message).toContain('Invalid or expired authorization code')
    })
  })

  // ---------------------------------------------------------------------------
  // Test 2: PKCE Verification Failure
  // ---------------------------------------------------------------------------

  test('Wrong code_verifier produces PKCE verification failure', async ({
    page,
    loginPage,
  }) => {
    const realmId = DEMO_ADMIN.realmId
    const redirectUri = 'https://example.com/oauth/callback'
    const state = crypto.randomUUID()
    const appName = `PKCE Mismatch Test ${Date.now()}`

    let clientId: string

    await test.step('Given: Admin is logged in and OAuth client app is seeded', async () => {
      await loginPage.loginAsAdmin(DEMO_ADMIN.email, DEMO_ADMIN.password, realmId)
      const result = await seedOAuthClientApp(page.request, realmId, {
        appName,
        redirectUris: [redirectUri],
      })
      clientId = result.clientId
    })

    let authCode: string

    await test.step('And: Full authorize + login flow produces an auth code', async () => {
      const flowResult = await completeOAuthLoginAndGetAuthCode(
        page, BASE_URL, realmId, clientId, redirectUri, state,
        { email: DEMO_ADMIN.email, password: DEMO_ADMIN.password },
      )
      authCode = flowResult.authCode
    })

    await test.step('When: Token exchange uses a wrong code_verifier', async () => {
      const wrongPkce = generatePKCEPair()

      const result = await oauthTokenExchange(BASE_URL, realmId, {
        grant_type: 'authorization_code',
        code: authCode,
        redirect_uri: redirectUri,
        client_id: clientId,
        code_verifier: wrongPkce.code_verifier,
      })

      await test.step('Then: Response contains PKCE verification failure', async () => {
        const errorResp = result as unknown as Record<string, unknown>
        const message = (errorResp.message as string) || (errorResp.error as string) || ''
        expect(message).toContain('PKCE verification failed')
      })
    })
  })

  // ---------------------------------------------------------------------------
  // Test 3: redirect_uri Not in Whitelist
  // ---------------------------------------------------------------------------

  test('Non-whitelisted redirect_uri is rejected at authorize', async ({
    page,
    loginPage,
  }) => {
    const realmId = DEMO_ADMIN.realmId
    const whitelistedUri = 'https://example.com/oauth/callback'
    const evilUri = 'https://evil.com/callback'
    const state = crypto.randomUUID()
    const appName = `Whitelist Test ${Date.now()}`

    let clientId: string

    await test.step('Given: OAuth client app is seeded with whitelist containing only example.com', async () => {
      await loginPage.loginAsAdmin(DEMO_ADMIN.email, DEMO_ADMIN.password, realmId)
      const result = await seedOAuthClientApp(page.request, realmId, {
        appName,
        redirectUris: [whitelistedUri],
      })
      clientId = result.clientId
    })

    await test.step('When: Authorize is called with non-whitelisted redirect_uri', async () => {
      const pkce = generatePKCEPair()

      const result = await oauthAuthorize(BASE_URL, realmId, {
        client_id: clientId,
        redirect_uri: evilUri,
        state,
        code_challenge: pkce.code_challenge,
      })

      await test.step('Then: Response returns 400 with whitelist error', async () => {
        expect(result.status).toBe(400)
        expect(result.errorBody).toBeTruthy()
        expect(result.errorBody!).toContain('not in the whitelist')
      })
    })
  })

  // ---------------------------------------------------------------------------
  // Test 4: Disabled Client App
  // ---------------------------------------------------------------------------

  test('Disabled client app returns 403 at authorize', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    const realmId = DEMO_ADMIN.realmId
    const redirectUri = 'https://example.com/oauth/callback'
    const state = crypto.randomUUID()
    const appName = `Disabled Client Test ${Date.now()}`

    let clientId: string

    await test.step('Given: Admin is logged in and OAuth client app is seeded', async () => {
      await loginPage.loginAsAdmin(DEMO_ADMIN.email, DEMO_ADMIN.password, realmId)
      const result = await seedOAuthClientApp(page.request, realmId, {
        appName,
        redirectUris: [redirectUri],
      })
      clientId = result.clientId
    })

    await test.step('And: Client app is disabled via admin UI', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto(realmId)
      await clientAppsPage.editClientApp(appName, { enabled: false }, realmId)
    })

    await test.step('When: Authorize is called with the disabled client_id', async () => {
      const pkce = generatePKCEPair()

      const result = await oauthAuthorize(BASE_URL, realmId, {
        client_id: clientId,
        redirect_uri: redirectUri,
        state,
        code_challenge: pkce.code_challenge,
      })

      await test.step('Then: Response returns 403 with disabled error', async () => {
        expect(result.status).toBe(403)
        expect(result.errorBody).toBeTruthy()
        expect(result.errorBody!).toContain('disabled')
      })
    })
  })

  // ---------------------------------------------------------------------------
  // Test 5: Invalid Authorization Code
  // ---------------------------------------------------------------------------

  test('Fabricated authorization code produces 400 at token exchange', async () => {
    const realmId = DEMO_ADMIN.realmId

    await test.step('When: Token exchange is called with a fabricated code', async () => {
      const result = await oauthTokenExchange(BASE_URL, realmId, {
        grant_type: 'authorization_code',
        code: 'ac_nonexistent_fabricated_code',
        redirect_uri: 'https://example.com/oauth/callback',
        client_id: 'some-client-id',
        code_verifier: 'fabricated_verifier_that_does_not_matter',
      })

      await test.step('Then: Response contains invalid code error', async () => {
        const errorResp = result as unknown as Record<string, unknown>
        const message = (errorResp.message as string) || (errorResp.error as string) || ''
        expect(message).toContain('Invalid or expired authorization code')
      })
    })
  })

  // ---------------------------------------------------------------------------
  // Test 6: Login with Mismatched State (state never stored in Redis)
  // ---------------------------------------------------------------------------

  test('Login with fabricated state returns 400 from login API', async ({
    page,
  }) => {
    const realmId = DEMO_ADMIN.realmId
    const fabricatedState = `fabricated-state-${crypto.randomUUID()}`
    const fabricatedClientId = `fabricated-client-${Date.now()}`

    await test.step('When: User navigates to login page with fabricated OAuth params (state never in Redis)', async () => {
      // Do NOT call authorize first -- the fabricated state was never stored in Redis.
      const loginUrl =
        `${BASE_URL}/${realmId}/auth/login?` +
        `oauthClientId=${encodeURIComponent(fabricatedClientId)}` +
        `&redirectUri=${encodeURIComponent('https://example.com/oauth/callback')}` +
        `&state=${encodeURIComponent(fabricatedState)}`

      await page.goto(loginUrl, { waitUntil: 'domcontentloaded' })
      await expect(page.getByTestId('login-card')).toBeVisible({ timeout: 10000 })
      await blockExternalCallback(page)
    })

    await test.step('And: User submits credentials', async () => {
      const loginResponsePromise = page.waitForResponse(isLoginApiResponse, { timeout: 15000 })

      await page.getByTestId('email-input').fill(DEMO_ADMIN.email)
      await page.getByTestId('password-input').fill(DEMO_ADMIN.password)
      await page.getByTestId('login-submit-button').click()

      const loginResponse = await loginResponsePromise

      await test.step('Then: Login API returns 400 with state error', async () => {
        expect(loginResponse.status()).toBe(400)
        const body = await loginResponse.text()
        expect(body).toContain('state')
      })
    })
  })

  // ---------------------------------------------------------------------------
  // Test 7: Partial OAuth Params Display Error
  // ---------------------------------------------------------------------------

  test('Partial OAuth params show error message and disable submit', async ({ page }) => {
    const realmId = DEMO_ADMIN.realmId

    await test.step('When: User navigates to login with only oauthClientId (partial params)', async () => {
      const url =
        `${BASE_URL}/${realmId}/auth/login?` +
        `oauthClientId=${encodeURIComponent('test-client')}`

      await page.goto(url, { waitUntil: 'domcontentloaded' })
    })

    await test.step('Then: OAuth incomplete error is visible', async () => {
      await expect(page.getByTestId('oauth-incomplete-error')).toBeVisible({ timeout: 10000 })
    })

    await test.step('And: Submit button is disabled', async () => {
      const submitButton = page.getByTestId('login-submit-button')
      await expect(submitButton).toBeVisible()
      await expect(submitButton).toBeDisabled()
    })
  })
})
