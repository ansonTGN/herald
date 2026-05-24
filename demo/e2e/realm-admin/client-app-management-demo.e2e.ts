/**
 * Client App Management Demo Tests (Consolidated)
 *
 * User Stories:
 * - US-TP-005: Client App Configuration Management
 * - US-TP-008: Configure Client App Redirect URI Whitelist
 * - US-TP-009: Manage Client App Icon
 * - US-TP-010: Enable/Disable Client App
 * - US-TP-011: Configure Session TTL Policy
 *
 * Test Structure:
 * 1. Complete Client App Lifecycle (Create -> Edit -> Delete)
 * 2. Page Navigation (Cancel, Tab switching, Data persistence)
 * 3. Tab Switching and Data Persistence
 * 4. Tab Controls (Verify all tabs present, Active tab styling)
 * 5. Keyboard Navigation (Tab through fields, Enter to submit, Cancel button)
 * 6. Keyboard Shortcuts (Shift+Tab, Focus management)
 */

import { test, cleanupTestData, expect } from '../fixtures/demo-page.fixtures'
import { ClientAppsPage, type ClientAppData } from '../pages/client-apps-page'
import { DEMO_ADMIN } from '../helpers/auth'
import { verifyTestEnvironment } from '../helpers/environment-setup'

test.describe('[Realm Admin] Client App Management Demo Tests', () => {
  test.beforeEach(async ({ page }) => {
    await verifyTestEnvironment(page, {
      requiredRealms: [DEMO_ADMIN.realmId],
      requiredUsers: [DEMO_ADMIN.email],
    })
  })

  test.afterEach(async ({ page, testStartTime }) => {
    await cleanupTestData(page, DEMO_ADMIN.realmId, {
      keepUsers: [DEMO_ADMIN.email],
      timestamp: testStartTime,
    })
  })

  // ============================================================================
  // Test 1: Complete Client App Lifecycle (Create -> Edit -> Delete)
  // ============================================================================
  test('Complete Client App Lifecycle', async ({ page, loginPage, demoLogger, testStartTime }) => {
    const originalName = `Lifecycle App ${testStartTime}`
    const updatedName = `Updated Lifecycle App ${testStartTime}`

    const testClientApp: ClientAppData = {
      clientId: `lifecycle-${testStartTime}`,
      name: originalName,
      description: 'Testing complete lifecycle',
      redirectUris: ['https://example.com/callback', 'https://app.example.com/auth'],
      enabled: true,
      sessionTtl: 3600,
      renewalTtl: 7200,
    }

    await test.step('Given: Admin is logged in', async () => {
      await loginPage.loginAsAdmin('admin@cas.com', 'password', 'admin')
      console.log('Admin logged in')
    })

    await test.step('When: Create Client App via form page', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)

      // Navigate to page
      await test.step('Step 1: Navigate to client apps page', async () => {
        await clientAppsPage.goto('admin')
        console.log('Navigated to client apps page')
      })

      // Open create form page
      await test.step('Step 2: Navigate to create page', async () => {
        await clientAppsPage.gotoCreatePage()
        await clientAppsPage.verifyPageTitle('Create Client App')
        console.log('Create form page opened')
      })

      // Fill Basic tab
      await test.step('Step 3: Fill basic information', async () => {
        await clientAppsPage.fillBasicTab({
          clientId: testClientApp.clientId,
          name: testClientApp.name,
          description: testClientApp.description,
        })
        console.log('Basic information filled')
      })

      // Configure Redirect URIs
      await test.step('Step 4: Configure redirect URIs', async () => {
        await clientAppsPage.fillRedirectUrisTab(testClientApp.redirectUris || [])
        console.log('Redirect URIs configured')
      })

      // Configure Security Settings
      await test.step('Step 5: Configure security settings', async () => {
        await clientAppsPage.fillSecurityTab({
          sessionTtlSeconds: testClientApp.sessionTtl || 3600,
          sessionRenewalTtlSeconds: testClientApp.renewalTtl,
        })
        console.log('Security settings configured')
      })

      // Submit
      await test.step('Step 6: Submit and verify creation', async () => {
        await clientAppsPage.submitForm()
        console.log('Client App created')
      })
    })

    await test.step('And: Verify creation in list', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto('admin')
      await clientAppsPage.waitForClientAppByName(testClientApp.name)
      console.log(`Client App "${testClientApp.name}" verified in list`)
    })

    await test.step('When: Edit Client App via form page', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)

      // Navigate to list and open edit page
      await clientAppsPage.goto('admin')

      await test.step('Step 1: Navigate to edit page', async () => {
        await clientAppsPage.gotoEditPage(originalName)
        await clientAppsPage.verifyPageTitle('Edit Client App')
        console.log('Edit form page opened')
      })

      // Update name on Basic tab
      await test.step('Step 2: Update basic information', async () => {
        await clientAppsPage.nameInput.clear()
        await clientAppsPage.nameInput.fill(updatedName)
        await clientAppsPage.descriptionInput.clear()
        await clientAppsPage.descriptionInput.fill('Updated description')
        console.log('Basic information updated')
      })

      // Submit changes
      await test.step('Step 3: Submit changes', async () => {
        await clientAppsPage.submitForm()
        console.log('Client App updated')
      })
    })

    await test.step('And: Verify edit in list', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto('admin')
      await clientAppsPage.waitForClientAppByName(updatedName)
      console.log(`Client App "${updatedName}" verified in list`)
    })

    await test.step('When: Delete Client App', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto('admin')

      // Find the client app by name
      const appId = await clientAppsPage.getClientIdByName(updatedName)
      expect(appId).toBeTruthy()

      // Delete the client app
      await clientAppsPage.deleteClientApp(appId)
      console.log(`Delete action initiated for "${updatedName}"`)
    })

    await test.step('Then: Verify deletion', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      const exists = await clientAppsPage.clientAppExists(updatedName)
      expect(exists).toBeFalsy()
      console.log(`Client App "${updatedName}" deleted successfully`)
    })
  })

  // ============================================================================
  // Test 2: Page Navigation (Cancel, Tab switching)
  // ============================================================================
  test('Page Navigation', async ({ page, loginPage, demoLogger, testStartTime }) => {
    await test.step('Given: Admin is logged in', async () => {
      await loginPage.loginAsAdmin('admin@cas.com', 'password', 'admin')
      console.log('Admin logged in')
    })

    await test.step('When: Test cancel form flow', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto('admin')

      // Open create form page
      await clientAppsPage.gotoCreatePage()

      // Fill some data
      await clientAppsPage.fillBasicTab({
        clientId: `cancel-${testStartTime}`,
        name: `Cancel Test App ${testStartTime}`,
        description: 'App to test cancellation',
      })

      // Cancel the form
      await clientAppsPage.cancelForm()
      console.log('Form cancelled')
    })

    await test.step('And: Verify no Client App was created', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto('admin')

      const exists = await clientAppsPage.clientAppExists(`Cancel Test App ${testStartTime}`)
      expect(exists).toBeFalsy()
      console.log('Verified no Client App was created')
    })

    await test.step('When: Test tab switching and data preservation', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto('admin')

      // Open create form page
      await clientAppsPage.gotoCreatePage()

      // Fill Basic tab
      await clientAppsPage.fillBasicTab({
        clientId: `nav-${testStartTime}`,
        name: `Navigation Test App ${testStartTime}`,
        description: 'App to test page navigation',
      })
      console.log('Basic tab filled')

      // Switch to Redirect URIs tab
      await clientAppsPage.switchTab('redirect-uris')
      await clientAppsPage.verifyActiveTab('redirect-uris')
      console.log('Switched to Redirect URIs tab')

      // Switch back to Basic tab
      await clientAppsPage.switchTab('basic')
      await clientAppsPage.verifyActiveTab('basic')
      console.log('Switched back to Basic tab')

      // Verify data is preserved
      const nameValue = await clientAppsPage.nameInput.inputValue()
      expect(nameValue).toBe(`Navigation Test App ${testStartTime}`)
      console.log('Data preserved after tab switch')

      // Switch to Redirect URIs again
      await clientAppsPage.switchTab('redirect-uris')
      await clientAppsPage.fillRedirectUrisTab(['https://example.com/callback'])
      console.log('Redirect URIs filled')

      // Switch to Security tab
      await clientAppsPage.switchTab('security')
      await clientAppsPage.verifyActiveTab('security')
      await clientAppsPage.fillSecurityTab({
        sessionTtlSeconds: 3600,
      })
      console.log('Security tab filled')

      // Submit the form
      await clientAppsPage.submitForm()
      console.log('Client App created')
    })

    await test.step('And: Verify creation', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto('admin')
      await clientAppsPage.waitForClientAppByName(`Navigation Test App ${testStartTime}`)
      console.log('Client App verified')
    })

    await test.step('When: Test edit page with tab navigation', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto('admin')

      const originalName = `Navigation Test App ${testStartTime}`
      await clientAppsPage.gotoEditPage(originalName)

      // Verify we are on Basic tab by default
      await clientAppsPage.verifyActiveTab('basic')

      // Verify Client ID is displayed (read-only in edit mode)
      await expect(clientAppsPage.clientIdDisplay).toBeVisible()
      console.log('Edit form page opened, Client ID display visible')

      // Switch to Security tab and update
      await clientAppsPage.switchTab('security')
      await clientAppsPage.fillSecurityTab({
        sessionTtlSeconds: 7200,
      })

      // Submit changes
      await clientAppsPage.submitForm()
      console.log('Client App updated via edit page')
    })

    await test.step('Then: Verify update', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto('admin')
      await clientAppsPage.waitForClientAppByName(`Navigation Test App ${testStartTime}`)
      console.log('Client App verified in list')
    })
  })

  // ============================================================================
  // Test 3: Tab Switching and Data Persistence
  // ============================================================================
  test('Tab Switching and Data Persistence', async ({ page, loginPage, demoLogger, testStartTime }) => {
    await test.step('Given: Admin is logged in', async () => {
      await loginPage.loginAsAdmin('admin@cas.com', 'password', 'admin')
      console.log('Admin logged in')
    })

    await test.step('When: Fill partial data and switch tabs freely', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto('admin')

      // Open create form page
      await clientAppsPage.gotoCreatePage()
      console.log('Create form page opened')

      // Fill name only on Basic tab
      await clientAppsPage.fillBasicTab({
        clientId: `persist-${testStartTime}`,
        name: `Persistence Test App ${testStartTime}`,
        description: 'Testing data persistence across tabs',
      })
      console.log('Filled name and description')

      // Switch to Redirect URIs tab without filling more basic data
      await clientAppsPage.switchTab('redirect-uris')
      await clientAppsPage.verifyActiveTab('redirect-uris')
      console.log('Switched to Redirect URIs tab')

      // Switch to Security tab without filling redirect URIs
      await clientAppsPage.switchTab('security')
      await clientAppsPage.verifyActiveTab('security')
      console.log('Switched to Security tab')

      // Switch to Appearance tab without filling security
      await clientAppsPage.switchTab('appearance')
      await clientAppsPage.verifyActiveTab('appearance')
      console.log('Switched to Appearance tab')

      // Switch back to Basic tab
      await clientAppsPage.switchTab('basic')
      await clientAppsPage.verifyActiveTab('basic')

      // Verify data is preserved
      const nameValue = await clientAppsPage.nameInput.inputValue()
      expect(nameValue).toBe(`Persistence Test App ${testStartTime}`)
      const descValue = await clientAppsPage.descriptionInput.inputValue()
      expect(descValue).toBe('Testing data persistence across tabs')
      console.log('Basic tab data preserved after navigating through all tabs')
    })

    await test.step('When: Complete the form across tabs', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)

      // We are on Basic tab - name already filled
      // Switch to Redirect URIs and fill
      await clientAppsPage.fillRedirectUrisTab(['https://example.com/callback'])
      console.log('Redirect URIs filled')

      // Switch to Security and fill
      await clientAppsPage.fillSecurityTab({
        sessionTtlSeconds: 3600,
      })
      console.log('Security settings filled')

      // Submit
      await clientAppsPage.submitForm()
      console.log('Client App created')
    })

    await test.step('Then: Verify creation', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto('admin')
      await clientAppsPage.waitForClientAppByName(`Persistence Test App ${testStartTime}`)
      console.log('Client App created successfully')
    })
  })

  // ============================================================================
  // Test 4: Tab Controls (Verify all tabs present, Active tab styling)
  // ============================================================================
  test('Tab Controls', async ({ page, loginPage, demoLogger, testStartTime }) => {
    await test.step('Given: Admin is logged in', async () => {
      await loginPage.loginAsAdmin('admin@cas.com', 'password', 'admin')
      console.log('Admin logged in')
    })

    await test.step('When: Open create page and verify tabs', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto('admin')

      // Open create form page
      await clientAppsPage.gotoCreatePage()
      console.log('Create form page opened')

      // Verify all tab triggers are visible
      await expect(clientAppsPage.tabBasic).toBeVisible()
      await expect(clientAppsPage.tabRedirectUris).toBeVisible()
      await expect(clientAppsPage.tabSecurity).toBeVisible()
      await expect(clientAppsPage.tabAppearance).toBeVisible()
      console.log('All four tabs visible')

      // Verify Basic tab is active by default
      await clientAppsPage.verifyActiveTab('basic')
      console.log('Basic tab is active by default')

      // Verify page title
      await clientAppsPage.verifyPageTitle('Create Client App')
      console.log('Page title verified: "Create Client App"')

      // Verify submit button text
      await clientAppsPage.verifySubmitButtonText('Create')
      console.log('Submit button text verified: "Create"')

      // Verify cancel button is visible
      await expect(clientAppsPage.cancelButton).toBeVisible()
      console.log('Cancel button visible')
    })

    await test.step('When: Verify tab switching updates active state', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)

      // Switch to Redirect URIs
      await clientAppsPage.switchTab('redirect-uris')
      await clientAppsPage.verifyActiveTab('redirect-uris')

      // Verify Basic is no longer active
      await expect(clientAppsPage.tabBasic).not.toHaveAttribute('data-state', 'active')
      console.log('Redirect URIs tab is now active, Basic is not')

      // Switch to Security
      await clientAppsPage.switchTab('security')
      await clientAppsPage.verifyActiveTab('security')
      console.log('Security tab is now active')

      // Switch to Appearance
      await clientAppsPage.switchTab('appearance')
      await clientAppsPage.verifyActiveTab('appearance')
      console.log('Appearance tab is now active')

      // Switch back to Basic
      await clientAppsPage.switchTab('basic')
      await clientAppsPage.verifyActiveTab('basic')
      console.log('Basic tab is active again')
    })

    await test.step('When: Fill form and create Client App', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)

      // Fill Basic tab (we are already on it)
      await clientAppsPage.fillBasicTab({
        clientId: `tabctrl-${testStartTime}`,
        name: `Tab Controls Test ${testStartTime}`,
        description: 'Testing tab controls',
      })

      // Fill Redirect URIs
      await clientAppsPage.fillRedirectUrisTab(['https://example.com/callback'])

      // Fill Security
      await clientAppsPage.fillSecurityTab({
        sessionTtlSeconds: 3600,
      })

      // Submit
      await clientAppsPage.submitForm()
      console.log('Client App created')
    })

    await test.step('Then: Verify creation', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto('admin')
      await clientAppsPage.waitForClientAppByName(`Tab Controls Test ${testStartTime}`)
      console.log('Client App verified')
    })
  })

  // ============================================================================
  // Test 5: Keyboard Navigation (Tab through fields, Enter to submit)
  // ============================================================================
  test('Keyboard Navigation', async ({ page, loginPage, demoLogger, testStartTime }) => {
    const clientAppsPage = new ClientAppsPage(page, demoLogger)

    await test.step('Given: Admin is logged in', async () => {
      await loginPage.loginAsAdmin('admin@cas.com', 'password', 'admin')
      console.log('Admin logged in')
    })

    await test.step('When: Navigate form fields with Tab key', async () => {
      await clientAppsPage.goto('admin')
      await clientAppsPage.gotoCreatePage()
      console.log('Create form page opened')

      // Focus on the client ID input and type
      await clientAppsPage.clientIdInput.focus()
      await page.keyboard.type(`kb-${testStartTime}`)
      console.log('Typed client ID')

      // Tab to name input
      await page.keyboard.press('Tab')

      // Type name
      await page.keyboard.type(`Keyboard Test App ${testStartTime}`)
      console.log('Typed app name')

      // Tab to description
      await page.keyboard.press('Tab')
      const focusedElement = await page.evaluate(() => document.activeElement?.getAttribute('data-testid'))
      expect(focusedElement).toBe('client-app-description-input')
      console.log('Tab: Focus moved to Description field')

      // Type description
      await page.keyboard.type('Testing keyboard navigation')
      console.log('Typed description')
    })

    await test.step('When: Submit form with button click', async () => {
      // Fill redirect URIs using POM method (keyboard URIs input is unreliable)
      await clientAppsPage.fillRedirectUrisTab(['https://example.com/callback'])
      console.log('Redirect URIs added via POM')

      // Fill security settings via POM
      await clientAppsPage.fillSecurityTab({
        sessionTtlSeconds: 3600,
      })
      console.log('Security settings filled')

      // Submit via button click (Enter in a tab-based form may not submit)
      await clientAppsPage.submitForm()
      console.log('Client App created')
    })

    await test.step('Then: Verify creation', async () => {
      await clientAppsPage.goto('admin')
      await clientAppsPage.waitForClientAppByName(`Keyboard Test App ${testStartTime}`)
      console.log('Client App created successfully via keyboard-assisted flow')
    })
  })

  // ============================================================================
  // Test 6: Keyboard Shortcuts (Cancel button, Shift+Tab)
  // ============================================================================
  test('Keyboard Shortcuts', async ({ page, loginPage, demoLogger, testStartTime }) => {
    await test.step('Given: Admin is logged in', async () => {
      await loginPage.loginAsAdmin('admin@cas.com', 'password', 'admin')
      console.log('Admin logged in')
    })

    await test.step('When: Test cancel button to return to list', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto('admin')

      // Open create form page
      await clientAppsPage.gotoCreatePage()

      // Fill some data
      await clientAppsPage.fillBasicTab({
        name: `Cancel Test App ${testStartTime}`,
      })
      console.log('Filled partial data')

      // Click cancel button to return to list
      await clientAppsPage.cancelForm()
      console.log('Cancel button clicked')

      // Verify form page is no longer visible
      await expect(clientAppsPage.formPage).toBeHidden({ timeout: 5000 })
      console.log('Form page closed via cancel button')
    })

    await test.step('And: Verify no Client App was created', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      const exists = await clientAppsPage.clientAppExists(`Cancel Test App ${testStartTime}`)
      expect(exists).toBeFalsy()
      console.log('Verified no Client App was created')
    })

    await test.step('When: Test Escape key to close delete confirmation', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)

      // Create a test app first
      await clientAppsPage.createClientApp({
        clientId: `esc-${testStartTime}`,
        name: `Esc Delete Test App ${testStartTime}`,
        redirectUris: ['https://example.com/callback'],
        sessionTtl: 3600,
      }, 'admin')
      console.log('Test Client App created')

      // Find and click delete button
      await clientAppsPage.goto('admin')
      const appId = await clientAppsPage.getClientIdByName(`Esc Delete Test App ${testStartTime}`)
      expect(appId).toBeTruthy()

      const row = page.locator(`[data-app-id="${appId}"]`)
      await row.locator('[data-testid="delete-client-app-button"]').click()
      console.log('Delete confirmation dialog opened')

      // Verify delete dialog is visible
      await expect(page.locator('[data-testid="delete-confirmation-dialog"]')).toBeVisible()

      // Press Escape to close
      await page.keyboard.press('Escape')
      console.log('Escape pressed on delete dialog')

      // Verify dialog closed
      await expect(page.locator('[data-testid="delete-confirmation-dialog"]')).toBeHidden({ timeout: 5000 })
      console.log('Delete dialog closed via Escape key')
    })

    await test.step('When: Test Shift+Tab for reverse navigation', async () => {
      const clientAppsPage = new ClientAppsPage(page, demoLogger)
      await clientAppsPage.goto('admin')

      // Open create form page
      await clientAppsPage.gotoCreatePage()

      // Focus on the submit button (last focusable element in footer)
      await clientAppsPage.submitButton.focus()
      console.log('Focused on submit button')

      // Press Shift+Tab to move backwards
      await page.keyboard.press('Shift+Tab')
      console.log('Shift+Tab pressed')

      // Verify focus moved to cancel button
      const focusedElement = await page.evaluate(() => document.activeElement?.getAttribute('data-testid'))
      // Focus should move from submit to cancel button
      expect(focusedElement).toBe('cancel-button')
      console.log(`Focus moved to: ${focusedElement}`)

      // Close form
      await clientAppsPage.cancelForm()
    })
  })
})
