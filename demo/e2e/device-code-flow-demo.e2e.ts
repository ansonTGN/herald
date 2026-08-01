/**
 * Device Code Flow Demo Tests
 *
 * Test Coverage:
 * - Test 1: Complete Device Code Authorization Flow (US-DC-001 + US-DC-002 + US-DC-003)
 *   - CLI initiates device authorization request
 *   - User enters user_code on verification page and authorizes
 *   - CLI polls for access token and succeeds
 * - Test 2: Device Code Flow via verification_uri_complete (US-DC-002 scenario 2)
 *   - User accesses verification_uri_complete URL directly
 *   - Page auto-fills code and auto-verifies
 *   - User authorizes and CLI polls for token
 *
 * API Exception: deviceAuthorize and deviceTokenPoll are CLI-side API endpoints
 * with no browser UI. Direct API calls are intentional as the caller in the
 * real flow is a CLI tool, not a browser user.
 *
 * @see docs/user-stories/auth/device-code.md
 */

import { test, expect, cleanupTestData } from './fixtures/demo-page.fixtures'
import { DeviceVerificationPage } from './pages/device-page'
import { seedDeviceCodeClientApp, deviceAuthorize, deviceTokenPoll } from './helpers/device-api'
import { verifyTestEnvironment } from './helpers/environment-setup'
import { DEMO_ADMIN, createBearerApiContext } from './helpers/auth'

test.describe('[Device Code] Device Code Flow Demo Tests', () => {
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

  test('Complete Device Code Authorization Flow', async ({ page, loginPage, demoLogger }) => {
    const realmId = DEMO_ADMIN.realmId

    let clientId: string

    await test.step('Given: 管理员已登录', async () => {
      await loginPage.loginAsAdmin(DEMO_ADMIN.email, DEMO_ADMIN.password, realmId)
      console.log('✓ Admin logged in')
    })

    await test.step('And: 准备启用了 Device Code Grant 的 Client App', async () => {
      const adminApiContext = await createBearerApiContext(loginPage.getAccessToken())
      const result = await seedDeviceCodeClientApp(adminApiContext, realmId)
      clientId = result.clientId
      console.log(`✓ Seeded device code client app (clientId=${clientId})`)
    })

    let deviceCode: string
    let userCode: string
    let verificationUri: string
    let verificationUriComplete: string

    await test.step('When: CLI 发起设备授权请求 (US-DC-001 scenario 1)', async () => {
      const response = await deviceAuthorize(undefined, realmId, clientId)
      expect(response.device_code).toBeDefined()
      expect(response.user_code).toMatch(/^[BCDFGHJKMNPQRSTVWXYZ0-9]{4}-[BCDFGHJKMNPQRSTVWXYZ0-9]{4}$/)
      expect(response.verification_uri).toContain(realmId)
      expect(response.verification_uri_complete).toContain(realmId)
      expect(response.expires_in).toBe(900)
      expect(response.interval).toBe(5)
      deviceCode = response.device_code
      userCode = response.user_code
      verificationUri = response.verification_uri
      verificationUriComplete = response.verification_uri_complete
      console.log(`✓ Device authorization requested (user_code=${userCode})`)
    })

    const devicePage = new DeviceVerificationPage(page, demoLogger)

    await test.step('And: 用户在验证页面输入 user_code (US-DC-002 scenario 1)', async () => {
      await devicePage.goto(realmId)
      await devicePage.enterCode(userCode)
      await devicePage.waitForVerified()
      console.log('✓ User code entered and verified')
    })

    await test.step('When: 用户点击授权', async () => {
      await devicePage.authorize()
      console.log('✓ User authorized the device')
    })

    await test.step('Then: 验证授权成功', async () => {
      const resultText = await devicePage.getResultText()
      expect(resultText.toLowerCase()).toContain('successful')
      console.log('✓ Authorization result confirmed')
    })

    await test.step('Then: CLI 轮询获取 access token (US-DC-003 scenario 1)', async () => {
      const tokenResponse = await deviceTokenPoll(undefined, realmId, deviceCode)
      if ('access_token' in tokenResponse) {
        expect(tokenResponse.access_token).toBeDefined()
        expect(tokenResponse.token_type).toBe('Bearer')
        expect(tokenResponse.expires_in).toBeDefined()
        console.log('✓ CLI obtained access token')
      } else {
        throw new Error(`Token poll failed: ${tokenResponse.error} - ${tokenResponse.error_description}`)
      }
    })
  })

  test('Device Code Flow via verification_uri_complete', async ({ page, loginPage, demoLogger }) => {
    const realmId = DEMO_ADMIN.realmId

    let clientId: string

    await test.step('Given: 管理员已登录', async () => {
      await loginPage.loginAsAdmin(DEMO_ADMIN.email, DEMO_ADMIN.password, realmId)
      console.log('✓ Admin logged in')
    })

    await test.step('And: 准备启用了 Device Code Grant 的 Client App', async () => {
      const adminApiContext = await createBearerApiContext(loginPage.getAccessToken())
      const result = await seedDeviceCodeClientApp(adminApiContext, realmId)
      clientId = result.clientId
      console.log(`✓ Seeded device code client app (clientId=${clientId})`)
    })

    let deviceCode: string
    let userCode: string

    await test.step('When: CLI 发起设备授权请求', async () => {
      const response = await deviceAuthorize(undefined, realmId, clientId)
      expect(response.device_code).toBeDefined()
      expect(response.user_code).toBeDefined()
      expect(response.verification_uri_complete).toContain(realmId)
      deviceCode = response.device_code
      userCode = response.user_code
      console.log(`✓ Device authorization requested (user_code=${userCode})`)
    })

    const devicePage = new DeviceVerificationPage(page, demoLogger)

    await test.step('And: 用户通过 verification_uri_complete 访问 (US-DC-002 scenario 2)', async () => {
      // gotoWithCode takes the raw code without hyphen (frontend handles formatting)
      const rawCode = userCode.replace('-', '')
      await devicePage.gotoWithCode(realmId, rawCode)
      // Page auto-submits verify on mount
      await devicePage.waitForVerified()
      console.log('✓ Auto-verified via verification_uri_complete')
    })

    await test.step('When: 用户点击授权', async () => {
      await devicePage.authorize()
      console.log('✓ User authorized the device')
    })

    await test.step('Then: 验证授权成功', async () => {
      const resultText = await devicePage.getResultText()
      expect(resultText.toLowerCase()).toContain('successful')
      console.log('✓ Authorization result confirmed')
    })

    await test.step('Then: CLI 轮询获取 access token', async () => {
      const tokenResponse = await deviceTokenPoll(undefined, realmId, deviceCode)
      if ('access_token' in tokenResponse) {
        expect(tokenResponse.access_token).toBeDefined()
        expect(tokenResponse.token_type).toBe('Bearer')
        expect(tokenResponse.expires_in).toBeDefined()
        console.log('✓ CLI obtained access token')
      } else {
        throw new Error(`Token poll failed: ${tokenResponse.error} - ${tokenResponse.error_description}`)
      }
    })
  })
})
