/**
 * Grant Points Demo Helpers
 *
 * Shared helper functions for the Grant Points feature demo tests.
 * Covers two user stories:
 * - US-PO-08: Admin grants points to a user via UI dialog
 * - US-TP-017: SDK grants points via ext API
 *
 * Selectors are imported from `../selectors` (single source of truth).
 *
 * @see docs/user-stories/billing/points-admin.md (Story 8)
 * @see docs/user-stories/integration/sdk.md (Story 4)
 */

import { Page, expect } from '@playwright/test'
import { SELECTORS } from '../selectors'
import { makeExtApiRequest } from './ext-api-helper'

// ============================================================================
// Types
// ============================================================================

export interface GrantFormOptions {
  email: string
  amount: number
  reason: string
  validityDays?: number
  permanent?: boolean
}

export interface GrantPointsExtApiBody {
  userId: string
  amount: number
  reason: string
  validityDays?: number
}

export interface ApiKeyWithPermission {
  apiKey: string
  clientId: string
}

// ============================================================================
// UI Flow Helpers (US-PO-08)
// ============================================================================

/**
 * Open the Grant Points dialog from the Points Wallets page.
 *
 * Navigates to `/{realmId}/manage/points/wallets`, waits for the page to load,
 * clicks the "Grant Points" button, and waits for the form dialog to be visible.
 */
export async function openGrantDialog(
  page: Page,
  realmId: string,
): Promise<void> {
  const gp = SELECTORS.grantPoints
  const walletsUrl = `/${realmId}/manage/points/wallets`

  await page.goto(walletsUrl)
  await expect(page.locator('[data-testid="points-wallets-page"]')).toBeVisible()

  await page.locator(gp.grantPointsButton).click()
  await expect(page.locator(gp.formDialog)).toBeVisible()
}

/**
 * Fill the Grant Points form fields.
 *
 * Steps:
 * 1. Types the email into the user search input
 * 2. Waits for search results to appear and clicks the first matching user
 * 3. Fills the amount field
 * 4. Handles validity: if `permanent` is true, enables the permanent toggle;
 *    otherwise fills `validityDays` (defaulting to 30 if not specified)
 * 5. Fills the reason textarea
 */
export async function fillGrantForm(
  page: Page,
  options: GrantFormOptions,
): Promise<void> {
  const gp = SELECTORS.grantPoints

  // 1. Search for user by email
  const searchInput = page.locator(gp.userSearchInput)
  await searchInput.fill(options.email)

  // Wait for user dropdown to appear with matching results
  const firstUserButton = page.locator(
    `${SELECTORS.grantPoints.formDialog} button:has-text("${options.email}")`,
  )
  await expect(firstUserButton).toBeVisible({ timeout: 10000 })
  await firstUserButton.click()

  // 2. Fill amount
  const amountInput = page.locator(gp.amountInput)
  await amountInput.clear()
  await amountInput.fill(options.amount.toString())

  // 3. Handle validity / permanent toggle
  if (options.permanent) {
    // Ensure the permanent toggle is checked
    const toggle = page.locator(gp.permanentToggle)
    const isChecked = await toggle.getAttribute('data-state')
    if (isChecked !== 'checked') {
      await toggle.click()
    }
  } else {
    // Dialog defaults to permanent mode (validityDays=null, toggle checked, input disabled).
    // Uncheck the toggle to enable the validity days input.
    const validityInput = page.locator(gp.validityDaysInput)
    if (await validityInput.isDisabled()) {
      await page.locator(gp.permanentToggle).click()
      await expect(validityInput).toBeEnabled({ timeout: 5000 })
    }
    // Fill validity days (default 30 — already set by unchecking permanent)
    const days = options.validityDays ?? 30
    const currentValue = await validityInput.inputValue()
    if (currentValue !== days.toString()) {
      await validityInput.clear()
      await validityInput.fill(days.toString())
    }
  }

  // 4. Fill reason
  const reasonInput = page.locator(gp.reasonInput)
  await reasonInput.clear()
  await reasonInput.fill(options.reason)
}

/**
 * Confirm the Grant Points dialog.
 *
 * Clicks "Review Grant" submit button, waits for the confirmation dialog,
 * clicks "Confirm Grant", and waits for the success toast.
 */
export async function confirmGrantDialog(page: Page): Promise<void> {
  const gp = SELECTORS.grantPoints

  // Click "Review Grant" to open confirmation dialog
  await page.locator(gp.submitButton).click()

  // Wait for confirmation dialog
  await expect(page.locator(gp.confirmDialog)).toBeVisible({ timeout: 10000 })

  // Click "Confirm Grant"
  await page.locator(gp.confirmButton).click()

  // Wait for success toast
  await expect(page.locator(SELECTORS.common.toast)).toBeVisible({ timeout: 10000 })
}

/**
 * Full UI orchestration for granting points.
 *
 * Opens the dialog, fills the form, and confirms the grant.
 * Convenience wrapper combining `openGrantDialog`, `fillGrantForm`, and `confirmGrantDialog`.
 */
export async function grantPointsViaUI(
  page: Page,
  realmId: string,
  options: GrantFormOptions,
): Promise<void> {
  await openGrantDialog(page, realmId)
  await fillGrantForm(page, options)
  await confirmGrantDialog(page)
}

// ============================================================================
// Ext API Helper (US-TP-017)
// ============================================================================

/**
 * Grant points via the External API.
 *
 * Calls `POST /points/{realmId}/grant` using API Key authentication.
 *
 * @see backend/api-ext/src/points.rs
 */
export async function grantPointsViaExtApi(
  apiKey: string,
  realmId: string,
  body: GrantPointsExtApiBody,
): Promise<{ status: number; responseBody: unknown }> {
  const { status, body: responseBody } = await makeExtApiRequest({
    apiKey,
    method: 'POST',
    path: `/points/${realmId}/grant`,
    body,
  })

  return { status, responseBody }
}

// ============================================================================
// API Key Setup Helper (for DE-D03 SDK tests)
// ============================================================================

/**
 * Create a test Client App and API Key via the admin HTTP API.
 *
 * Uses Playwright's request context to call the backend directly.
 * Shares cookies with the browser context for authentication.
 *
 * When `permission` is "points.manage", this creates a custom role with that
 * permission and assigns it to the API key. Built-in roles cannot be assigned
 * to API keys (backend rejects with 400), so a dedicated test role is necessary.
 * For any other permission value, no role is assigned (key has no permissions).
 *
 * @param page - Playwright Page (must be logged in as admin)
 * @param permission - If "points.manage", creates and assigns a role with that permission
 * @param testStartTime - Unique suffix for resource names (use Date.now())
 */
export async function createTestApiKeyWithPermission(
  page: Page,
  permission: string,
  testStartTime: number,
): Promise<ApiKeyWithPermission> {
  const suffix = testStartTime
  const clientAppName = `grant-test-app-${suffix}`
  const apiKeyName = `grant-test-key-${suffix}`
  const realmId = 'admin'

  // Determine backend URL (same logic as ext-api-helper)
  const backendUrl =
    process.env.API_BASE_URL ||
    process.env.BASE_URL?.replace(/:\d+/, ':8080') ||
    'http://localhost:8080'

  // 1. Create Client App via admin API
  const clientAppResponse = await page.context().request.post(
    `${backendUrl}/api/client/${realmId}`,
    {
      data: {
        clientId: clientAppName,
        name: clientAppName,
        redirectUris: ['https://example.com/callback'],
        enabled: true,
        sessionTtlSeconds: 1800,
      },
    },
  )

  if (!clientAppResponse.ok()) {
    const text = await clientAppResponse.text()
    throw new Error(
      `Failed to create client app "${clientAppName}": ${clientAppResponse.status()} ${text}`,
    )
  }

  const clientAppBody = await clientAppResponse.json()
  const clientId = clientAppBody.id ?? clientAppName

  // 2. Create API Key bound to the new Client App
  const apiKeyResponse = await page.context().request.post(
    `${backendUrl}/api/api-keys/${realmId}`,
    {
      data: {
        name: apiKeyName,
        clientAppId: clientId,
      },
    },
  )

  if (!apiKeyResponse.ok()) {
    const text = await apiKeyResponse.text()
    throw new Error(
      `Failed to create API key "${apiKeyName}": ${apiKeyResponse.status()} ${text}`,
    )
  }

  const apiKeyBody = await apiKeyResponse.json()
  const apiKey = apiKeyBody.key ?? ''
  const apiKeyId = apiKeyBody.id ?? ''

  // 3. Assign permission via a custom role (built-in roles cannot be assigned to API keys).
  if (apiKeyId) {
    await assignPermissionRoleToApiKey(page, backendUrl, realmId, apiKeyId, suffix, permission)
  }

  return { apiKey, clientId }
}

/**
 * Create a custom role with the given permission and assign it to the API key.
 *
 * Steps:
 * 1. Find the permission ID via GET /api/permission/{realmId}/define
 * 2. Create a custom role via POST /api/roles/{realmId}/define
 * 3. Assign the permission to the role via POST /api/roles/{realmId}/define/{roleId}/permissions
 * 4. Assign the role to the API key via PUT /api/api-keys/{realmId}/{apiKeyId}/roles
 */
async function assignPermissionRoleToApiKey(
  page: Page,
  backendUrl: string,
  realmId: string,
  apiKeyId: string,
  suffix: number,
  permission: string,
): Promise<void> {
  const request = page.context().request

  // 3a. Find the permission ID
  const permListResponse = await request.get(
    `${backendUrl}/api/permission/${realmId}/define`,
  )
  if (!permListResponse.ok()) {
    throw new Error(
      `Failed to list permissions: ${permListResponse.status()} ${await permListResponse.text()}`,
    )
  }
  const permListBody = await permListResponse.json()
  // The response is ApiResult<Vec<PermissionResponse>>, serialized as bare array
  const permissions: { id: string; name: string }[] = Array.isArray(permListBody)
    ? permListBody
    : permListBody.items ?? []
  const targetPerm = permissions.find((p) => p.name === permission)
  if (!targetPerm) {
    throw new Error(
      `${permission} permission not found in realm ${realmId}. ` +
        `Available: ${permissions.map((p) => p.name).join(', ')}`,
    )
  }

  // 3b. Create a custom role (is_builtin=false, required for API key assignment)
  const roleName = `test-${permission.replace('.', '-')}-${suffix}`
  const roleCreateResponse = await request.post(
    `${backendUrl}/api/roles/${realmId}/define`,
    {
      data: {
        name: roleName,
        description: `Auto-created test role with ${permission} permission`,
        clientId: 'admin-web-console',
      },
    },
  )
  if (!roleCreateResponse.ok()) {
    throw new Error(
      `Failed to create role "${roleName}": ${roleCreateResponse.status()} ${await roleCreateResponse.text()}`,
    )
  }
  const roleBody = await roleCreateResponse.json()
  const roleId = roleBody.id

  // 3c. Assign the permission to the new role
  const assignPermResponse = await request.post(
    `${backendUrl}/api/roles/${realmId}/define/${roleId}/permissions`,
    {
      data: { permissionId: targetPerm.id },
    },
  )
  if (!assignPermResponse.ok()) {
    throw new Error(
      `Failed to assign ${permission} to role ${roleId}: ${assignPermResponse.status()} ${await assignPermResponse.text()}`,
    )
  }

  // 3d. Assign the role to the API key
  const assignRoleResponse = await request.put(
    `${backendUrl}/api/api-keys/${realmId}/${apiKeyId}/roles`,
    {
      data: { roleIds: [roleId] },
    },
  )
  if (!assignRoleResponse.ok()) {
    throw new Error(
      `Failed to assign role to API key ${apiKeyId}: ${assignRoleResponse.status()} ${await assignRoleResponse.text()}`,
    )
  }
}
