/**
 * Realm Admin Comprehensive Demo Tests
 *
 * User Stories: docs/user-stories/core/realm-admin.md
 *
 * Test Coverage:
 * - US-RA-001: Realm Isolation Access
 *   - Scenario 4.1: Access own realm resources
 *   - Scenario 4.2: Cannot access other realm resources
 *   - Scenario 4.3: UI cross-realm access denied
 *
 * Note: Other realm-admin stories are covered in separate test files:
 * - realm-admin-rbac-comprehensive-demo.e2e.ts: US-RA-002 ~ US-RA-006, US-BP-001
 * - client-app-management-demo.e2e.ts: Client app management
 * - realm-admin-oauth-config-demo.e2e.ts: OAuth provider configuration
 * - realm-admin-totp-config-demo.e2e.ts: TOTP configuration
 *
 * Test Data Strategy:
 * - Uses Demo Seed created realms (realm-001, admin)
 * - Does NOT create realms through UI (violates demo testing spec)
 * - Tests cross-realm isolation using existing seeded data
 */

import { test, cleanupTestData, expect } from '../fixtures/demo-page.fixtures'
import { UsersPage } from '../pages/users-page'
import { verifyTestEnvironment } from '../helpers/environment-setup'

const ADMIN_REALM = 'admin'
const ADMIN_EMAIL = 'admin@cas.com'
// Use Demo Seed created realm-001 for cross-realm testing
const SEEDED_REALM = 'realm-001'
const SEEDED_REALM_ADMIN = 'admin@realm-001.com'

test.describe('[Realm Admin] Realm Isolation Access Demo Tests', () => {
  let testStartTime: number

  test.beforeEach(async ({ page }) => {
    testStartTime = Date.now()
    // Verify Demo Seed data exists (realm-001 and admin realms)
    await verifyTestEnvironment(page, {
      requiredRealms: [ADMIN_REALM, SEEDED_REALM],
      requiredUsers: [ADMIN_EMAIL, SEEDED_REALM_ADMIN],
    })
  })

  test.afterEach(async ({ page }) => {
    await cleanupTestData(page, ADMIN_REALM, {
      keepUsers: [ADMIN_EMAIL, SEEDED_REALM_ADMIN],
      timestamp: testStartTime,
    })
  })

  test.describe('User Story 1: Realm Isolation Access [US-RA-001]', () => {
    test('Scenario 4.1: 访问自己 Realm 的资源', async ({ page, loginPage, demoLogger }) => {
      await test.step('Given: 用户已登录为 realm-001 管理员', async () => {
        await loginPage.loginAsAdmin(SEEDED_REALM_ADMIN, 'password', SEEDED_REALM)
      })

      await test.step('When: 访问 realm-001 的用户管理页面', async () => {
        const usersPage = new UsersPage(page, demoLogger)
        await usersPage.goto(SEEDED_REALM)
        // Sidebar navigation lands on the session-scoped manage route
        // (`/manage/users`, realm resolved from the auth store); realm-scoped
        // `/realm-001/manage/users` is only entered via direct cross-realm URL.
        await expect(page).toHaveURL(/\/manage\/users$/)
      })

      await test.step('Then: 可以看到 realm-001 的用户', async () => {
        await expect(page.locator('table[data-testid="users-table"]')).toBeVisible()
      })
    })

    test('Scenario 4.2: 不能访问其他 Realm 的资源', async ({ page, loginPage, demoLogger }) => {
      await test.step('Given: 用户已登录为 realm-001 管理员', async () => {
        await loginPage.loginAsAdmin(SEEDED_REALM_ADMIN, 'password', SEEDED_REALM)
      })

      await test.step('When: 尝试直接访问 admin realm 的用户管理页面', async () => {
        // Try to access admin realm while authenticated as realm-001
        await page.goto(`/${ADMIN_REALM}/manage/users`)
      })

      await test.step('Then: 系统重定向回用户已认证的 realm', async () => {
        // Wait for redirect to complete
        await page.waitForLoadState('networkidle')

        const currentUrl = page.url()
        // User should be redirected back to their authenticated realm (realm-001)
        expect(currentUrl).toContain(`/${SEEDED_REALM}/manage/users`)
        // User should NOT be on the admin realm
        expect(currentUrl).not.toContain(`/${ADMIN_REALM}/`)
      })
    })

    test('Scenario 4.3: UI 跨 Realm 访问被拒绝', async ({ page, loginPage, demoLogger }) => {
      await test.step('Given: 用户已登录为 realm-001 管理员', async () => {
        await loginPage.loginAsAdmin(SEEDED_REALM_ADMIN, 'password', SEEDED_REALM)
      })

      await test.step('When: 尝试通过 URL 直接访问 admin realm 的用户页面', async () => {
        // Direct URL access attempt
        await page.goto(`/${ADMIN_REALM}/manage/users`)
      })

      await test.step('Then: 系统拒绝访问并重定向回用户自己的 realm', async () => {
        // Wait for redirect to complete
        await page.waitForLoadState('networkidle')

        const currentUrl = page.url()
        // Verify user is redirected back to their authenticated realm
        expect(currentUrl).toContain(`/${SEEDED_REALM}/`)
        // Verify user is NOT on the attempted realm
        expect(currentUrl).not.toContain(`/${ADMIN_REALM}/`)

        // Verify the users table for their own realm is visible
        await expect(page.locator('table[data-testid="users-table"]')).toBeVisible()
      })
    })
  })
})
