/**
 * Audit Auto-Recording Demo Test
 *
 * Test Coverage:
 * - US-AU-005: System Auto-Records Core Operations
 *   - Scenario 1: User create -> audit event recorded (category=user_management, action=user.create)
 *   - Scenario 2: RBAC change -> audit event recorded (category=rbac, action=role.create)
 *   - Scenario 4: Auth event -> audit event recorded (category=auth, action=auth.login)
 * - US-AU-004: Admin Realm Platform Audit
 *   - Scenario 1: Admin Realm admin sees platform-level audit logs
 *   - Scenario 2: Only admin realm audit events visible (realm isolation)
 *
 * Strategy:
 * - Test 1 creates a user, then navigates to audit page to verify user.create event
 * - Test 2 creates a role, then navigates to audit page to verify role.create event
 * - Test 3 verifies auth events from the fixture login
 * - Test 4 verifies platform-level categories and realm isolation
 *
 * Uses fixtures from demo-page.fixtures for auto-login and navigation.
 * Cross-page navigation is done via sidebar clicks (AuditPage.goto).
 *
 * @see ../../../docs/user-stories/14-audit-user-stories.md
 */

import { test, expect, cleanupTestData } from '../fixtures/demo-page.fixtures'
import { AuditPage } from '../pages/audit-page'

test.describe('US-AU-005: Auto-Recording of Core Operations', () => {
  test.afterEach(async ({ page, testStartTime }) => {
    await cleanupTestData(page, 'admin', {
      timestamp: testStartTime,
    })
  })

  test('S1: User create operation is recorded as audit event', async ({ usersPage, page, testStartTime, demoLogger }) => {
    const testEmail = `audit-user-${testStartTime}@example.com`
    const testPassword = 'TestPassword123!'

    const auditPage = new AuditPage(page, demoLogger)

    await test.step('Create a test user via UI', async () => {
      await usersPage.createUser({
        email: testEmail,
        password: testPassword,
      })
      const userExists = await usersPage.userExists(testEmail)
      expect(userExists).toBe(true)
    })

    await test.step('Navigate to audit page via sidebar', async () => {
      await auditPage.goto()
      await expect(auditPage.table).toBeVisible()
    })

    await test.step('Filter by category "user_management"', async () => {
      await auditPage.filterByCategory('user_management')
      await expect(auditPage.table).toBeVisible()
    })

    await test.step('Verify at least one user_management event exists', async () => {
      const rowCount = await auditPage.getRowCount()
      expect(rowCount).toBeGreaterThan(0)
    })

    await test.step('Verify a row contains "user.create" action text', async () => {
      const found = await auditPage.hasRowWithColumnText(3, ['user.create'])
      expect(found).toBe(true)
    })

    await test.step('Clean up: navigate back to users page and delete the test user', async () => {
      await usersPage.deleteUser(testEmail, 'admin')
    })
  })

  test('S2: Role create operation is recorded as audit event', async ({ rolesPage, page, testStartTime, demoLogger }) => {
    const roleName = `audit-role-${testStartTime}`

    const auditPage = new AuditPage(page, demoLogger)

    await test.step('Create a test role via UI', async () => {
      await rolesPage.createRole({
        name: roleName,
        description: `Audit test role created at ${testStartTime}`,
      })
      const exists = await rolesPage.roleExists(roleName)
      expect(exists).toBe(true)
    })

    await test.step('Navigate to audit page via sidebar', async () => {
      await auditPage.goto()
      await expect(auditPage.table).toBeVisible()
    })

    await test.step('Filter by category "rbac"', async () => {
      await auditPage.filterByCategory('rbac')
      await expect(auditPage.table).toBeVisible()
    })

    await test.step('Verify at least one rbac event exists', async () => {
      const rowCount = await auditPage.getRowCount()
      expect(rowCount).toBeGreaterThan(0)
    })

    await test.step('Verify a row contains "role.create" action text', async () => {
      const found = await auditPage.hasRowWithColumnText(3, ['role.create'])
      expect(found).toBe(true)
    })

    await test.step('Clean up: navigate back to roles page and delete the test role', async () => {
      await rolesPage.goto()
      await rolesPage.deleteRole(roleName)
    })
  })

  test('S4: Auth login event is recorded as audit event', async ({ auditPage }) => {
    await test.step('Audit page is loaded (login was performed by fixture)', async () => {
      await expect(auditPage.container).toBeVisible()
      await expect(auditPage.table).toBeVisible()
    })

    await test.step('Filter by category "auth"', async () => {
      await auditPage.filterByCategory('auth')
      await expect(auditPage.table).toBeVisible()
    })

    await test.step('Verify at least one auth event exists', async () => {
      const rowCount = await auditPage.getRowCount()
      // The fixture login generates an auth.login event
      expect(rowCount).toBeGreaterThan(0)
    })

    await test.step('Verify a row contains "auth.login" action text', async () => {
      const found = await auditPage.hasRowWithColumnText(3, ['auth.login'])
      expect(found).toBe(true)
    })

    await test.step('Verify result column shows "success" for login event', async () => {
      const rowTexts = await auditPage.getRowTexts(0)
      const resultText = rowTexts[5].toLowerCase()
      expect(resultText).toContain('success')
    })
  })
})

test.describe('US-AU-004: Admin Realm Platform Audit', () => {
  test.afterEach(async ({ page, testStartTime }) => {
    await cleanupTestData(page, 'admin', {
      timestamp: testStartTime,
    })
  })

  test('Admin realm shows platform-level audit logs with multiple categories', async ({ auditPage }) => {
    await test.step('Audit page is loaded with admin realm data', async () => {
      await expect(auditPage.container).toBeVisible()
      await expect(auditPage.heading).toBeVisible()
      await expect(auditPage.table).toBeVisible()
    })

    await test.step('Unfiltered view shows audit events', async () => {
      const rowCount = await auditPage.getRowCount()
      expect(rowCount).toBeGreaterThan(0)
    })

    await test.step('Filter by "realm_management" category shows events', async () => {
      await auditPage.filterByCategory('realm_management')
      await expect(auditPage.table).toBeVisible()

      const rowCount = await auditPage.getRowCount()
      // Admin realm seed data should include realm creation / RBAC init events
      expect(rowCount).toBeGreaterThan(0)
    })

    await test.step('Verify realm_management rows contain expected action types', async () => {
      const found = await auditPage.hasRowWithColumnText(3, [
        'realm.create',
        'realm.rbac_init',
      ])
      expect(found).toBe(true)
    })

    await test.step('Clear filters and verify general events visible again', async () => {
      await auditPage.clearFilters()
      await expect(auditPage.table).toBeVisible()

      const rowCount = await auditPage.getRowCount()
      expect(rowCount).toBeGreaterThan(0)
    })

    await test.step('Verify only admin realm events are shown (realm isolation)', async () => {
      const rowCount = await auditPage.getRowCount()
      expect(rowCount).toBeGreaterThan(0)

      const validCategories = [
        'user management',
        'rbac',
        'realm management',
        'auth',
      ]
      for (let i = 0; i < Math.min(rowCount, 5); i++) {
        const rowTexts = await auditPage.getRowTexts(i)
        const category = rowTexts[2].toLowerCase().trim()
        const isValid = validCategories.some((vc) => category.includes(vc))
        expect(isValid).toBe(true)
      }
    })
  })
})
