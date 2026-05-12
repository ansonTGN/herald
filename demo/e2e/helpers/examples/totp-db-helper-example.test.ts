/**
 * Example: Using TOTP Database Helper in Playwright Tests
 *
 * This example demonstrates how to integrate the TOTP database helper
 * with Playwright E2E tests for setup, cleanup, and debugging.
 *
 * To run this example:
 *   pwsh -File scripts/run-test-quiet.ps1 demo/e2e/helpers/examples/totp-db-helper-example.test.ts
 */

import { test, expect } from '@playwright/test'
import {
  disableUserTOTP,
  disableRealmTOTP,
  resetRealmTOTP,
  getUserTOTPConfig,
  getRealmTOTPSettings,
  listUsersWithTOTPEnabled,
  ensureAdminNoTOTP,
  closePool,
} from '../totp-db-helper'

test.describe('TOTP Database Helper Examples', () => {
  // Cleanup after all tests
  test.afterAll(async () => {
    await closePool()
  })

  test('Example 1: Ensure admin does not have TOTP', async () => {
    // This is useful for test setup - ensures admin is in a clean state
    const wasDisabled = await ensureAdminNoTOTP('admin@cas.com', 'admin')
    console.log(`Admin TOTP was disabled: ${wasDisabled}`)

    // Verify admin does not have TOTP enabled
    const config = await getUserTOTPConfig('admin-id-placeholder', 'admin')
    if (config) {
      expect(config.enabled).toBe(false)
    }
  })

  test('Example 2: Reset realm TOTP for clean test state', async () => {
    // Use in beforeEach to ensure clean state before each test
    await resetRealmTOTP('admin')

    // Verify realm settings are disabled
    const settings = await getRealmTOTPSettings('admin')
    expect(settings?.enabled).toBe(false)
    expect(settings?.force_enabled).toBe(false)

    // Verify no users have TOTP enabled
    const userIds = await listUsersWithTOTPEnabled('admin')
    expect(userIds.length).toBe(0)
  })

  test('Example 3: Disable specific user TOTP after test', async () => {
    // Simulate a test that enables TOTP for a user
    const testUserId = 'test-user-id-placeholder'

    // After test completes, cleanup TOTP
    await disableUserTOTP(testUserId, 'admin')

    // Verify TOTP is disabled
    const config = await getUserTOTPConfig(testUserId, 'admin')
    if (config) {
      expect(config.enabled).toBe(false)
    }
  })

  test('Example 4: Debug TOTP state', async ({ page }) => {
    // Use in combination with UI tests to debug state
    const realmId = 'admin'

    // Check realm TOTP settings
    const settings = await getRealmTOTPSettings(realmId)
    console.log(`Realm TOTP Settings:`, settings)

    // List all users with TOTP enabled
    const userIds = await listUsersWithTOTPEnabled(realmId)
    console.log(`Users with TOTP enabled (${userIds.length}):`, userIds)

    // For each user, get detailed configuration
    for (const userId of userIds) {
      const config = await getUserTOTPConfig(userId, realmId)
      console.log(`User ${userId}:`, {
        enabled: config?.enabled,
        verified_at: config?.verified_at,
        last_used_at: config?.last_used_at,
      })
    }
  })

  test('Example 5: Comprehensive test with TOTP setup and cleanup', async ({
    page,
  }) => {
    const realmId = 'admin'

    // Setup: Ensure clean state
    await resetRealmTOTP(realmId)

    // Test body would go here...
    // - Navigate to UI
    // - Enable TOTP
    // - Verify TOTP works
    // - Disable TOTP
    // await page.goto('http://localhost:3000/admin/security')
    // ... test code ...

    // Cleanup: Reset TOTP state
    await resetRealmTOTP(realmId)
  })

  test('Example 6: Conditional TOTP disabling', async () => {
    const userId = 'test-user-id-placeholder'

    // Only disable if currently enabled
    const config = await getUserTOTPConfig(userId, 'admin')
    if (config && config.enabled) {
      console.log(`Disabling TOTP for user ${userId}`)
      await disableUserTOTP(userId, 'admin')
    } else {
      console.log(`User ${userId} does not have TOTP enabled`)
    }
  })

  test('Example 7: Batch disable TOTP for multiple users', async () => {
    const realmId = 'admin'

    // Get all users with TOTP enabled
    const userIds = await listUsersWithTOTPEnabled(realmId)
    console.log(`Found ${userIds.length} users with TOTP enabled`)

    // Disable TOTP for all of them
    for (const userId of userIds) {
      await disableUserTOTP(userId, realmId)
      console.log(`Disabled TOTP for user ${userId}`)
    }

    // Verify all are disabled
    const remaining = await listUsersWithTOTPEnabled(realmId)
    expect(remaining.length).toBe(0)
  })

  test('Example 8: Realm-level TOTP management', async () => {
    const realmId = 'admin'

    // Disable realm TOTP settings (force_enabled and enabled flags)
    await disableRealmTOTP(realmId)

    // Verify settings are disabled
    const settings = await getRealmTOTPSettings(realmId)
    expect(settings?.enabled).toBe(false)
    expect(settings?.force_enabled).toBe(false)
  })
})

// ============================================================================
// Test Suite with beforeEach/afterEach Hooks
// ============================================================================

test.describe('TOTP Tests with Automatic Cleanup', () => {
  const realmId = 'admin'

  // Setup: Ensure clean state before each test
  test.beforeEach(async () => {
    await resetRealmTOTP(realmId)
  })

  // Cleanup after all tests
  test.afterAll(async () => {
    await closePool()
  })

  test('Test 1: TOTP functionality', async ({ page }) => {
    // Test body - state is guaranteed to be clean
    // await page.goto('http://localhost:3000/admin/security')
    // ... test code ...
  })

  test('Test 2: Another TOTP test', async ({ page }) => {
    // Test body - state is guaranteed to be clean
    // await page.goto('http://localhost:3000/admin/security')
    // ... test code ...
  })
})

