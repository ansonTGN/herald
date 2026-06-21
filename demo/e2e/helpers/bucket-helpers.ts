/**
 * Credit Bucket Demo Helpers
 *
 * Shared helper functions for the Credit Bucket feature demo tests.
 * User stories covered (across DE-D02..D05):
 * - US-CB-001: admin CRUD on the Credit Bucket directory
 * - US-CB-002: bind a Client App coverage set to a bucket (≥1 required)
 * - US-CB-003: assign provider entitlement mappings to a bucket
 *
 * Scope:
 * - UI helpers drive the Master-Detail editor at
 *   `/{realmId}/manage/billing/credit-buckets`. They perform NO business
 *   assertions — they only orchestrate clicks/fills and surface the resulting
 *   state (created id, error code, conflict presence). Assertion responsibility
 *   lives in the `.e2e.ts` flow tests (DE-D02 et al.).
 * - API helpers mirror `_http_json` in `scripts/lib/demo_seed.py` so other
 *   items can provision buckets in `beforeAll` without exercising the UI.
 *
 * Selectors are imported from `../selectors` (single source of truth). Bucket
 * keys/names come from `./bucket-seed-ids`.
 *
 * @see .ai/design/credit-bucket.md §4.2 (API + error codes), §4.3.2 (schema)
 * @see docs/user-stories/billing/credit-bucket.md
 */

import { Page, expect, type APIResponse } from '@playwright/test'
import { SELECTORS } from '../selectors'

// ============================================================================
// Types
// ============================================================================

/** Options accepted by `createBucketViaUI` (matches POST /credit-buckets). */
export interface CreateBucketOptions {
  bucketKey: string
  name: string
  description?: string
  displayOrder?: number
  enabled?: boolean
  /** Client App UUIDs to bind as the coverage set (≥1 required by the API). */
  clientAppIds: string[]
  /** Provider entitlement mapping UUIDs to assign to this bucket. */
  mappingIds?: string[]
  /**
   * Mark this bucket as the realm's registration pool. Per realm at most one
   * bucket may carry this flag (`uq_credit_buckets_registration_pool`).
   * Setting it on a realm that already has one yields 409
   * `registration_pool_conflict`.
   */
  receivesRegistrationCredits?: boolean
}

/** Partial fields for `editBucketViaUI`. `undefined` fields are left as-is. */
export interface EditBucketFields {
  name?: string
  description?: string
  displayOrder?: number
  enabled?: boolean
  receivesRegistrationCredits?: boolean
}

/** Result of attempting a destructive bucket delete via the confirm dialog. */
export interface DeleteBucketResult {
  /** Whether the dialog closed after confirm (delete accepted). */
  success: boolean
  /**
   * Error code surfaced in `delete-bucket-error-message` when the backend
   * refused the delete. Expected values: `bucket_in_use` (active subscriptions
   * or holders with balance remain). `undefined` on success.
   */
  errorCode?: string
  /** Raw error message text (for diagnostics / soft assertions). */
  errorMessage?: string
}

/**
 * API payload for `POST /api/realms/{realmId}/billing/credit-buckets`.
 * Mirrors `_http_json` in `scripts/lib/demo_seed.py`.
 */
export interface CreateBucketApiPayload {
  bucketKey: string
  name: string
  description?: string
  displayOrder?: number
  enabled?: boolean
  clientAppIds: string[]
  entitlementMappingIds?: string[]
  receivesRegistrationCredits?: boolean
}

/** Bucket list item shape returned by `GET /api/realms/{realmId}/billing/credit-buckets`. */
export interface CreditBucketListItem {
  id: string
  bucketKey: string
  name: string
  displayOrder: number
  enabled: boolean
  receivesRegistrationCredits: boolean
  coveredClientAppCount: number
  entitlementMappingCount: number
}

// ============================================================================
// UI Helpers — directory navigation / selection / editor
// ============================================================================

/**
 * Parse a bucket id from a `credit-bucket-list-item-${id}` testid attribute.
 *
 * Used by `createBucketViaUI` to return the created id after the directory
 * reloads. The list item wrapper carries the full testid on its root element;
 * Playwright exposes it via `getAttribute('data-testid')`.
 */
export async function parseBucketIdFromListItem(
  page: Page,
  bucketKey: string,
): Promise<string> {
  // The directory list renders one `[data-testid^="credit-bucket-list-item-"]`
  // wrapper per bucket. The bucket_key is not in the testid, so locate the
  // matching item by its visible bucket-key text and read the testid suffix.
  const item = page.locator('[data-testid^="credit-bucket-list-item-"]').filter({
    hasText: bucketKey,
  })
  await expect(item.first()).toBeVisible({ timeout: 10000 })
  const testid = await item.first().getAttribute('data-testid')
  if (!testid) {
    throw new Error(`Could not read data-testid from list item for bucket ${bucketKey}`)
  }
  // Strip the static prefix; the remainder is the bucket id.
  return testid.replace('credit-bucket-list-item-', '')
}

/**
 * Create a Credit Bucket via the directory UI editor.
 *
 * Assumes the caller is already authenticated as a realm admin and on (or able
 * to navigate to) the Credit Bucket directory. Returns the created bucket's id
 * by reading the list-item testid after the directory re-renders.
 *
 * NOTE: this helper performs no assertions on success semantics — it only
 * orchestrates the create flow and resolves the id. The calling test asserts
 * the persistent state (list item visible, registration badge, etc.).
 */
export async function createBucketViaUI(
  page: Page,
  realmId: string,
  options: CreateBucketOptions,
): Promise<string> {
  const cb = SELECTORS.creditBucket

  await page.goto(`/${realmId}/manage/billing/credit-buckets`)
  await expect(page.locator(cb.directoryPage)).toBeVisible()

  await page.locator(cb.newButton).click()
  await expect(page.locator(cb.editor)).toBeVisible()

  await page.locator(cb.editorBucketKey).fill(options.bucketKey)
  await page.locator(cb.editorName).fill(options.name)
  if (options.description !== undefined) {
    await page.locator(cb.editorDescription).fill(options.description)
  }

  // Coverage set — bind client apps via the multiselect. Each click on
  // `${prefix}-item-${id}` toggles selection; the dropdown opens on input focus.
  await bindCoverageSetViaUI(page, options.clientAppIds)

  // Mapping assignment is optional on create.
  if (options.mappingIds && options.mappingIds.length > 0) {
    await assignMappingsViaUI(page, options.mappingIds)
  }

  // Registration pool flag — at most one per realm.
  if (options.receivesRegistrationCredits) {
    const registrationSwitch = page.locator(cb.editorRegistration)
    const state = await registrationSwitch.getAttribute('data-state')
    if (state !== 'checked') {
      await registrationSwitch.click()
      // Wait for data-state to flip so the form value persists on submit.
      await expect(registrationSwitch).toHaveAttribute('data-state', 'checked', {
        timeout: 3000,
      })
    }
  }

  // Submit and wait for the editor to close (directory re-renders the row).
  await page.locator(cb.editorSubmit).click()
  await expect(page.locator(cb.editor)).toBeHidden({ timeout: 15000 })

  return parseBucketIdFromListItem(page, options.bucketKey)
}

/**
 * Open the editor for an existing bucket (Master-Detail selection).
 *
 * Clicks the list item, which loads the editor on the right pane.
 */
export async function openBucketEditor(
  page: Page,
  bucketId: string,
): Promise<void> {
  const cb = SELECTORS.creditBucket
  await page.locator(cb.listItem(bucketId)).click()
  await expect(page.locator(cb.editor)).toBeVisible({ timeout: 10000 })
}

/**
 * Edit editable fields of an existing bucket and submit.
 *
 * Only fields present in `fields` are touched. Returns nothing — the caller
 * asserts the resulting list-item / detail state.
 */
export async function editBucketViaUI(
  page: Page,
  bucketId: string,
  fields: EditBucketFields,
): Promise<void> {
  const cb = SELECTORS.creditBucket
  await openBucketEditor(page, bucketId)

  if (fields.name !== undefined) {
    await page.locator(cb.editorName).fill(fields.name)
  }
  if (fields.description !== undefined) {
    await page.locator(cb.editorDescription).fill(fields.description)
  }
  if (fields.enabled !== undefined) {
    const enabledSwitch = page.locator(cb.editorEnabled)
    const wantChecked = fields.enabled ? 'checked' : 'unchecked'
    // Read the current state; if it doesn't match the target, click and wait
    // for Radix Switch's `data-state` to transition before continuing. The
    // transition is what makes the form's `field.state.value` flip; without
    // waiting, a fast submit can race past the React state update and persist
    // the OLD value (the badge / list refresh then reflects no change).
    let state = await enabledSwitch.getAttribute('data-state')
    if (state !== wantChecked) {
      await enabledSwitch.click()
      await expect(async () => {
        state = await enabledSwitch.getAttribute('data-state')
        expect(state).toBe(wantChecked)
      }).toPass({ timeout: 3000 })
    }
  }
  if (fields.receivesRegistrationCredits !== undefined) {
    const registrationSwitch = page.locator(cb.editorRegistration)
    const wantChecked = fields.receivesRegistrationCredits ? 'checked' : 'unchecked'
    let state = await registrationSwitch.getAttribute('data-state')
    if (state !== wantChecked) {
      await registrationSwitch.click()
      // Same race-resolve as the `enabled` switch above.
      await expect(async () => {
        state = await registrationSwitch.getAttribute('data-state')
        expect(state).toBe(wantChecked)
      }).toPass({ timeout: 3000 })
    }
  }

  await page.locator(cb.editorSubmit).click()
  // On a successful save the editor stays open but the conflict alert must
  // NOT appear; on a registration_pool_conflict 409 it does. Callers that
  // need the conflict path should use `setRegistrationPoolViaUI` instead.
  await expect(page.locator(cb.editorRegistrationConflict)).toBeHidden({
    timeout: 5000,
  })
}

/**
 * Toggle the `enabled` flag on a bucket via the editor switch.
 */
export async function toggleBucketEnabledViaUI(
  page: Page,
  bucketId: string,
): Promise<void> {
  const cb = SELECTORS.creditBucket
  await openBucketEditor(page, bucketId)
  await page.locator(cb.editorEnabled).click()
  await page.locator(cb.editorSubmit).click()
}

/**
 * Mark a bucket as the realm registration pool via the editor.
 *
 * Surfaces the 409 `registration_pool_conflict` case: if another bucket in the
 * realm already carries the flag, the backend rejects the update and the
 * frontend renders `credit-bucket-editor-registration-conflict`. This helper
 * returns the resulting conflict visibility so the caller can assert either
 * branch (success / conflict) without relying on auto-dismissing toasts.
 *
 * @returns `true` if the conflict alert rendered (another registration pool
 *          already exists), `false` if the save succeeded cleanly.
 */
export async function setRegistrationPoolViaUI(
  page: Page,
  bucketId: string,
  enable: boolean,
): Promise<boolean> {
  const cb = SELECTORS.creditBucket
  await openBucketEditor(page, bucketId)

  const registrationSwitch = page.locator(cb.editorRegistration)
  const state = await registrationSwitch.getAttribute('data-state')
  const wantChecked = enable ? 'checked' : 'unchecked'
  if (state !== wantChecked) {
    await registrationSwitch.click()
  }

  await page.locator(cb.editorSubmit).click()

  // The conflict alert renders synchronously on 409; on success it stays hidden.
  // Race-resolve both outcomes with a short timeout.
  const conflictLocator = page.locator(cb.editorRegistrationConflict)
  let conflictVisible = false
  try {
    await expect(conflictLocator).toBeVisible({ timeout: 5000 })
    conflictVisible = true
  } catch {
    conflictVisible = false
  }
  return conflictVisible
}

// ============================================================================
// UI Helpers — coverage set + mapping assignment multiselects
// ============================================================================

/**
 * Toggle client apps in the coverage multiselect so the given set is bound.
 *
 * The multiselect is a command-palette: focus the search input, then click each
 * `${prefix}-item-${id}` row to toggle membership. Idempotent in the sense that
 * re-clicking an already-selected item deselects it — callers should pass the
 * full desired set each time.
 *
 * Exposed separately so DE-D02 can drive coverage-set edits independently of
 * full bucket creation.
 */
export async function bindCoverageSetViaUI(
  page: Page,
  clientAppIds: string[],
): Promise<void> {
  const cb = SELECTORS.creditBucket
  if (clientAppIds.length === 0) {
    // Coverage set must be non-empty on create (400 from backend). Nothing to
    // toggle here — the caller is responsible for the validation assertion.
    return
  }

  // Open the command palette by clicking the combobox TRIGGER (the search
  // input lives inside Radix `PopoverContent`, which only renders after the
  // `PopoverTrigger` button is activated). Clicking the search input directly
  // times out because the input does not exist in the DOM until the popover
  // opens. The trigger carries the prefix testid `bucket-coverage-multiselect`.
  await page.locator(cb.coverageMultiselect).click()
  const search = page.locator(cb.coverageMultiselectSearch)
  await expect(search).toBeVisible({ timeout: 5000 })
  await search.click()

  for (const appId of clientAppIds) {
    const item = page.locator(cb.coverageMultiselectItem(appId))
    await expect(item).toBeVisible({ timeout: 5000 })
    await item.click()
  }

  // Close the popover via Escape. `search.blur()` does NOT close a Radix
  // Popover — without this the popover stays open and obscures / intercepts
  // the editor submit click, leading to "element detached" retries.
  await page.keyboard.press('Escape')
  await expect(search).toBeHidden({ timeout: 2000 })
}

/**
 * Assign (toggle) provider entitlement mappings onto a bucket.
 *
 * Same command-palette UX as the coverage multiselect, different prefix.
 */
export async function assignMappingsViaUI(
  page: Page,
  mappingIds: string[],
): Promise<void> {
  const cb = SELECTORS.creditBucket
  if (mappingIds.length === 0) {
    return
  }

  // Open the popover via its trigger (see `bindCoverageSetViaUI` for the
  // Radix-Popover rationale — the search input is gated behind the trigger).
  await page.locator(cb.mappingsMultiselect).click()
  const search = page.locator(cb.mappingsMultiselectSearch)
  await expect(search).toBeVisible({ timeout: 5000 })
  await search.click()

  for (const mappingId of mappingIds) {
    const item = page.locator(cb.mappingsMultiselectItem(mappingId))
    await expect(item).toBeVisible({ timeout: 5000 })
    await item.click()
  }

  await page.keyboard.press('Escape')
  await expect(search).toBeHidden({ timeout: 2000 })
}

// ============================================================================
// UI Helpers — destructive delete flow
// ============================================================================

/**
 * Open the delete confirm dialog for a bucket (does NOT confirm).
 *
 * The `credit-bucket-delete-button` is rendered in the right-pane editor area
 * (credit-bucket-directory-page.tsx) ONLY when a bucket is selected and its
 * editor is visible — NOT inside the left-column list item. This helper opens
 * the editor for the bucket first, then clicks the delete button.
 */
export async function requestDeleteBucketViaUI(
  page: Page,
  bucketId: string,
): Promise<void> {
  const cb = SELECTORS.creditBucket
  await openBucketEditor(page, bucketId)
  await page.locator(cb.deleteButton).click()
  await expect(page.locator(cb.deleteConfirmDialog)).toBeVisible({ timeout: 5000 })
}

/**
 * Confirm the bucket delete dialog and classify the outcome.
 *
 * On success the dialog closes. On 409 `bucket_in_use` the dialog stays open
 * and renders `delete-bucket-error-message`. This helper does not assert; it
 * returns the outcome so the caller can assert either branch.
 *
 * @param page Playwright page (must have `requestDeleteBucketViaUI` called first)
 * @returns `{ success: true }` or `{ success: false, errorCode, errorMessage }`
 */
export async function confirmDeleteBucket(
  page: Page,
): Promise<DeleteBucketResult> {
  const cb = SELECTORS.creditBucket
  const dialog = page.locator(cb.deleteConfirmDialog)
  await expect(dialog).toBeVisible()

  await page.locator(cb.deleteConfirmButton).click()

  // Two terminal states, raced: dialog hidden (success) or error message
  // visible (409 bucket_in_use). Resolve whichever fires within the timeout.
  const errorLocator = page.locator(cb.deleteErrorMessage)
  let outcome: DeleteBucketResult
  try {
    await Promise.race([
      expect(dialog).toBeHidden({ timeout: 10000 }),
      expect(errorLocator).toBeVisible({ timeout: 10000 }),
    ])
  } catch {
    // Neither terminal state rendered — treat as failure with no code.
    return { success: false }
  }

  const errorVisible = await errorLocator.isVisible().catch(() => false)
  if (errorVisible) {
    const rawMessage = (await errorLocator.textContent()) || ''
    return {
      success: false,
      errorCode: classifyDeleteErrorCode(rawMessage),
      errorMessage: rawMessage.trim(),
    }
  }
  return { success: true }
}

/**
 * Map a human-readable error message to its backend code.
 *
 * The frontend renders a localized message for `bucket_in_use`; the helper
 * recognizes the design-documented code substring if present, and otherwise
 * returns `undefined` (caller should assert on the raw message for stability).
 */
function classifyDeleteErrorCode(message: string): string | undefined {
  const lower = message.toLowerCase()
  if (lower.includes('bucket_in_use')) return 'bucket_in_use'
  if (lower.includes('in use') || lower.includes('active subscription')) {
    return 'bucket_in_use'
  }
  return undefined
}

// ============================================================================
// API Helpers — admin-authenticated bucket provisioning
// ============================================================================

/**
 * Resolve the backend base URL the same way other helpers do.
 *
 * Mirrors the resolution in `grant-points-helpers.ts::createTestApiKeyWithPermission`.
 */
function backendBaseUrl(): string {
  return (
    process.env.API_BASE_URL ||
    process.env.BASE_URL?.replace(/:\d+/, ':8080') ||
    'http://localhost:8080'
  )
}

/**
 * Create a Credit Bucket via the admin HTTP API.
 *
 * Uses Playwright's `page.context().request` so the browser's authenticated
 * session (the X-Auth cookie set by `loginAsAdmin`) is reused — no separate
 * auth header required. Mirrors the `_http_json` pattern in
 * `scripts/lib/demo_seed.py` (status check + parsed body).
 *
 * Route: `POST /api/realms/{realmId}/billing/credit-buckets`.
 * Auth: Realm Admin with `points.manage`.
 *
 * @throws on network failure or unexpected (non-2xx, non-409) status.
 */
export async function createBucketViaApi(
  page: Page,
  realmId: string,
  payload: CreateBucketApiPayload,
): Promise<{ status: number; body: CreditBucketListItem | Record<string, unknown> }> {
  const url = `${backendBaseUrl()}/api/realms/${realmId}/billing/credit-buckets`
  const response = await page.context().request.post(url, { data: payload })

  const body = await parseJsonBody(response)
  if (!response.ok() && response.status() !== 409) {
    throw new Error(
      `createBucketViaApi failed: status=${response.status()} body=${JSON.stringify(body)}`,
    )
  }
  return { status: response.status(), body: body as CreditBucketListItem | Record<string, unknown> }
}

/**
 * List Credit Buckets via the admin HTTP API.
 *
 * Route: `GET /api/realms/{realmId}/billing/credit-buckets`.
 * Auth: Realm Admin with `points.manage` (view).
 */
export async function listBucketsViaApi(
  page: Page,
  realmId: string,
): Promise<CreditBucketListItem[]> {
  const url = `${backendBaseUrl()}/api/realms/${realmId}/billing/credit-buckets`
  const response = await page.context().request.get(url)
  const body = await parseJsonBody(response)
  if (!response.ok()) {
    throw new Error(
      `listBucketsViaApi failed: status=${response.status()} body=${JSON.stringify(body)}`,
    )
  }
  // Backend wraps list responses either as a bare array or as { items: [...] }.
  if (Array.isArray(body)) {
    return body as CreditBucketListItem[]
  }
  if (body && Array.isArray((body as { items?: unknown }).items)) {
    return (body as { items: CreditBucketListItem[] }).items
  }
  if (body && Array.isArray((body as { data?: unknown }).data)) {
    return (body as { data: CreditBucketListItem[] }).data
  }
  throw new Error(
    `listBucketsViaApi returned unexpected body shape: ${JSON.stringify(body)}`,
  )
}

async function parseJsonBody(
  response: APIResponse,
): Promise<Record<string, unknown> | unknown[]> {
  const contentType = response.headers()['content-type'] || ''
  if (contentType.includes('application/json')) {
    try {
      return (await response.json()) as Record<string, unknown> | unknown[]
    } catch {
      return { raw: await response.text() }
    }
  }
  return { raw: await response.text() }
}
