/**
 * Invoice Fallback Demo Helpers
 *
 * Shared helper functions for invoice-fallback E2E demo tests.
 * Covers: external invoice seeding via DB insert, policy configuration,
 * provider capability toggling, and provider-specific UI verification.
 *
 * All seed functions use execPgSql (DB insert) instead of API calls
 * because the API forces provider='manual' on creation.
 */

import { execSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { Page, expect } from '@playwright/test'

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const POSTGRES_CONTAINER = 'cas-demo-postgres'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ExternalInvoiceOptions {
  provider: 'stripe' | 'creem'
  /** External invoice ID (e.g. Stripe in_xxx). Defaults to a generated UUID. */
  externalInvoiceId?: string
  /** External order ID (e.g. Creem order ID). Defaults to undefined. */
  externalOrderId?: string
  /** External hosted page URL. Stripe invoices typically have this. */
  externalHostedUrl?: string
  /** External PDF download URL. Stripe invoices typically have this. */
  externalPdfUrl?: string
  /** Invoice status. Defaults to 'issued'. */
  status?: string
  /** Currency code. Defaults to 'USD'. */
  currency?: string
  /** Total amount in smallest currency unit. Defaults to 1000. */
  total?: number
  /** Account ID to associate the invoice with a user. Required for user-visible invoices. */
  accountId?: string
}

export interface ExternalInvoiceResult {
  id: string
  invoiceNumber: string
  provider: string
  externalInvoiceId?: string
}

export interface CreemPaymentAttemptOptions {
  /** User email to look up account_id. Required. */
  userEmail: string
  /** Target type. Defaults to 'subscription_entitlement'. */
  targetType?: string
  /** Target ID (e.g. subscription plan ID). Defaults to a generated UUID. */
  targetId?: string
  /** Amount in smallest currency unit. Defaults to 1000. */
  amount?: number
  /** Currency code. Defaults to 'USD'. */
  currency?: string
  /** Payment attempt status. Defaults to 'Succeeded'. */
  status?: string
}

export interface CreemPaymentAttemptResult {
  id: string
  paymentProvider: string
}

// ---------------------------------------------------------------------------
// DB Helpers
// ---------------------------------------------------------------------------

/**
 * Execute a PostgreSQL query inside the demo Postgres container.
 * Standalone function using execSync -- no Page object needed.
 *
 * `--set ON_ERROR_STOP=on` makes psql exit non-zero on any SQL error so a
 * failed INSERT/SELECT surfaces via execSync (which throws on non-zero exit)
 * instead of being silently swallowed. Without this, a NOT NULL / FK violation
 * would print to stderr but exit 0, returning an empty string and leaving the
 * caller with a phantom id (the root cause of the Creem fallback test timeout).
 */
function execPgSql(query: string): string {
  return execSync(
    `docker exec -i ${POSTGRES_CONTAINER} psql -U postgres -d herald_demo -t -A --set ON_ERROR_STOP=on`,
    {
      input: query,
      encoding: 'utf-8',
      timeout: 10000,
    },
  ).trim()
}

// ---------------------------------------------------------------------------
// Seed: External Invoice
// ---------------------------------------------------------------------------

/**
 * Create an external invoice record via direct DB insert.
 *
 * Uses execPgSql because the POST /invoices API endpoint forces provider='manual'.
 * Inserts directly into the `invoice` table with all required fields including
 * provider, external_hosted_url, external_pdf_url, external_invoice_id,
 * source='external_sync', and a generated invoice_number.
 *
 * For Stripe invoices, set externalHostedUrl and externalPdfUrl.
 * For Creem invoices, leave externalHostedUrl and externalPdfUrl unset.
 *
 * Returns the created invoice ID and invoice number.
 */
export function seedExternalInvoice(
  realmId: string,
  options: ExternalInvoiceOptions,
): ExternalInvoiceResult {
  const id = randomUUID()
  const provider = options.provider
  const externalInvoiceId = options.externalInvoiceId ?? `ext_${provider}_${id.slice(0, 8)}`
  const invoiceNumber = `EXT-${provider.toUpperCase()}-${id.slice(0, 8)}`
  const status = options.status ?? 'issued'
  const currency = options.currency ?? 'USD'
  const total = options.total ?? 1000
  const subtotal = total
  const source = 'external_sync'

  const columns = [
    'id',
    'realm_id',
    'invoice_number',
    'source',
    'provider',
    'status',
    'currency',
    'subtotal',
    'discount_amount',
    'tax_amount',
    'shipping_amount',
    'total',
    'due_date',
    'external_invoice_id',
    'created_at',
    'updated_at',
  ]

  const values = [
    `'${id}'`,
    `'${realmId}'`,
    `'${invoiceNumber}'`,
    `'${source}'`,
    `'${provider}'`,
    `'${status}'`,
    `'${currency}'`,
    subtotal,
    '0',
    '0',
    '0',
    total,
    'NOW()',
    `'${externalInvoiceId}'`,
    'NOW()',
    'NOW()',
  ]

  if (options.accountId) {
    columns.push('account_id')
    values.push(`'${options.accountId}'`)
  }

  if (options.externalHostedUrl) {
    columns.push('external_hosted_url')
    values.push(`'${options.externalHostedUrl}'`)
  }

  if (options.externalPdfUrl) {
    columns.push('external_pdf_url')
    values.push(`'${options.externalPdfUrl}'`)
  }

  if (options.externalOrderId) {
    columns.push('external_order_id')
    values.push(`'${options.externalOrderId}'`)
  }

  const sql = `INSERT INTO invoice (${columns.join(', ')}) VALUES (${values.join(', ')})`
  execPgSql(sql)

  return {
    id,
    invoiceNumber,
    provider,
    externalInvoiceId,
  }
}

// ---------------------------------------------------------------------------
// Seed: Creem Payment Attempt
// ---------------------------------------------------------------------------

/**
 * Create a payment_attempt row with payment_provider='creem' via direct DB insert.
 *
 * Used by DE-D03 for the Creem rejection test.
 *
 * The row must satisfy all NOT NULL columns to actually persist. Critically:
 * - `bucket_id` is `uuid NOT NULL REFERENCES credit_buckets(id)`; omitting it
 *   makes Postgres reject the INSERT. Because the previous `execPgSql` did not
 *   set `ON_ERROR_STOP=on`, that failure was silent and the test proceeded with
 *   a phantom id, causing the purchase-history row (and its Invoice button) to
 *   never render. We resolve a real `bucket_id` for the realm the same way the
 *   companion `seedPurchaseAttempt` does.
 * - `target_type` must be `'entitlement_mapping'` (the only value allowed by
 *   the current `chk_target_type` constraint, and the value the purchase-history
 *   list filter requires) and `target_id` must reference a real
 *   `provider_entitlement_mappings.id` so the list join populates product info.
 * - `status='Succeeded'` is required for the row to appear in purchase history
 *   and for the per-row Invoice button to render.
 * - `payment_provider='creem'` is what the eligibility rule
 *   (`determine_invoice_apply_route`) matches to return the disabled +
 *   "Merchant of Record" verdict.
 *
 * Returns the created payment_attempt ID.
 */
export function seedCreemPaymentAttempt(
  realmId: string,
  options: CreemPaymentAttemptOptions,
): CreemPaymentAttemptResult {
  const userId = execPgSql(
    `SELECT id FROM account WHERE email = '${options.userEmail}' AND realm_id = '${realmId}'`,
  )
  if (!userId) {
    throw new Error(`User not found: ${options.userEmail} in realm ${realmId}`)
  }

  // Resolve a credit bucket for the realm (bucket_id is NOT NULL FK).
  const bucketId = execPgSql(
    `SELECT id FROM credit_buckets WHERE realm_id = '${realmId}' ORDER BY created_at LIMIT 1`,
  )
  if (!bucketId) {
    throw new Error(`No credit_buckets row found for realm ${realmId}`)
  }

  // Resolve an enabled entitlement mapping to use as target_id. Prefer a Creem
  // one_time mapping; fall back to any enabled one_time mapping in the realm.
  // The purchase-history query LEFT JOINs provider_entitlement_mappings on
  // target_id, so a real row keeps the product-info columns populated.
  let mappingId = execPgSql(
    `SELECT id FROM provider_entitlement_mappings WHERE realm_id = '${realmId}' AND billing_type = 'one_time' AND enabled = true AND payment_provider = 'creem' ORDER BY created_at LIMIT 1`,
  )
  if (!mappingId) {
    mappingId = execPgSql(
      `SELECT id FROM provider_entitlement_mappings WHERE realm_id = '${realmId}' AND billing_type = 'one_time' AND enabled = true ORDER BY created_at LIMIT 1`,
    )
  }
  if (!mappingId) {
    throw new Error(
      `No enabled one_time provider_entitlement_mappings row found for realm ${realmId}`,
    )
  }

  const id = randomUUID()
  // target_type is constrained to 'entitlement_mapping' (single-value CHECK);
  // the purchase-history filter also requires this exact value.
  const targetType = 'entitlement_mapping'
  const amount = options.amount ?? 1000
  const currency = options.currency ?? 'USD'
  const status = options.status ?? 'Succeeded'
  const providerReference = `creem_demo_${id.slice(-3)}`
  // provider_status mirrors what a real Creem webhook would set for a paid
  // one-time purchase; completed_at marks it finished (required for the row to
  // read as a completed purchase).
  const providerStatus = 'paid'

  const sql =
    `INSERT INTO payment_attempts ` +
    `(id, realm_id, user_id, payment_provider, target_type, target_id, bucket_id, amount, currency, status, provider_reference, provider_status, metadata, expires_at, completed_at, created_at, updated_at) ` +
    `VALUES ('${id}', '${realmId}', '${userId}'::uuid, 'creem', '${targetType}', '${mappingId}'::uuid, '${bucketId}'::uuid, ${amount}, '${currency}', '${status}', '${providerReference}', '${providerStatus}', NULL, NOW() + INTERVAL '1 hour', NOW(), NOW(), NOW())`
  execPgSql(sql)

  return {
    id,
    paymentProvider: 'creem',
  }
}

// ---------------------------------------------------------------------------
// Policy Configuration
// ---------------------------------------------------------------------------

/**
 * Open the invoice policy configuration dialog.
 *
 * Prerequisite: user is on the invoice admin page.
 */
export async function openPolicyConfigDialog(page: Page): Promise<void> {
  await page.getByTestId('policy-config-button').click()
  await expect(page.getByTestId('invoice-policy-form-dialog')).toBeVisible({
    timeout: 5000,
  })
}

/**
 * Set the invoice policy via the policy config dialog.
 *
 * Opens the dialog, selects the given policy value, saves, and waits
 * for the dialog to close.
 *
 * Prerequisite: user is on the invoice admin page.
 *
 * @param policy - One of 'provider_first', 'manual_only', 'none'
 */
export async function setInvoicePolicy(
  page: Page,
  policy: 'provider_first' | 'manual_only' | 'none',
): Promise<void> {
  await openPolicyConfigDialog(page)

  // Select the policy value
  await page.getByTestId('invoice-policy-select').click()
  const optionLabel = policy === 'provider_first' ? 'Provider First' : policy === 'manual_only' ? 'Manual Only' : 'Disabled'
  await page.getByRole('option', { name: optionLabel }).click()

  // Save
  await page.getByTestId('invoice-policy-save-button').click()

  // Wait for dialog to close
  await expect(page.getByTestId('invoice-policy-form-dialog')).toBeHidden({
    timeout: 10000,
  })
}

/**
 * Toggle a provider's external invoice capability switch.
 *
 * Opens the policy config dialog, toggles the provider switch, saves,
 * and waits for the dialog to close.
 *
 * Prerequisite: user is on the invoice admin page.
 *
 * @param provider - Provider key, e.g. 'stripe' or 'creem'
 * @param enabled - Whether to enable (true) or disable (false) the capability
 */
export async function toggleProviderCapability(
  page: Page,
  provider: string,
  enabled: boolean,
): Promise<void> {
  await openPolicyConfigDialog(page)

  const switchLocator = page.getByTestId(`invoice-policy-${provider}-switch`)

  // Read current state and only toggle if needed
  const currentState = await switchLocator.isChecked()
  if (currentState !== enabled) {
    await switchLocator.click()
  }

  // Save
  await page.getByTestId('invoice-policy-save-button').click()

  // Wait for dialog to close
  await expect(page.getByTestId('invoice-policy-form-dialog')).toBeHidden({
    timeout: 10000,
  })
}

// ---------------------------------------------------------------------------
// Verification: Provider Column
// ---------------------------------------------------------------------------

/**
 * Verify that a table row displays the correct provider badge.
 *
 * The provider column shows a Badge with the provider label.
 * For 'manual' it shows "Manual", for 'stripe' it shows "Stripe", etc.
 * When provider is 'manual', no external badge is shown in the user page,
 * so this helper checks for the provider label text within the row.
 *
 * @param row - A Playwright locator for the table row (<tr>)
 * @param provider - Provider key, e.g. 'stripe', 'creem', 'manual'
 */
export async function verifyProviderColumnInRow(
  row: import('@playwright/test').Locator,
  provider: string,
): Promise<void> {
  const label = provider === 'manual' ? 'Manual' : provider.charAt(0).toUpperCase() + provider.slice(1)
  // Provider column is at cell index 4 (after #, Invoice Number, Buyer, Source).
  // Target the <td> at index 4 to avoid strict mode violations when the label
  // text (e.g. "Manual") appears in both the Source and Provider columns.
  const providerCell = row.locator('td').nth(4)
  await expect(providerCell).toBeVisible({ timeout: 10000 })
  await expect(providerCell.getByText(label, { exact: true })).toBeVisible()
}

// ---------------------------------------------------------------------------
// Verification: External Invoice Actions
// ---------------------------------------------------------------------------

/**
 * Verify that an external invoice row shows only "View" and "View in Provider"
 * actions (no Edit, Issue, Void, Mark Paid).
 *
 * Opens the action menu for the invoice row and checks that the restricted
 * actions are absent while the allowed actions are present.
 *
 * Prerequisite: user is on the invoice admin page.
 *
 * @param invoiceId - The UUID of the external invoice
 */
export async function verifyExternalInvoiceActions(
  page: Page,
  invoiceId: string,
): Promise<void> {
  // Open the actions dropdown menu
  await page.getByTestId(`invoice-actions-menu-${invoiceId}`).click()

  // "View" action should be present
  await expect(page.getByTestId(`invoice-view-${invoiceId}`)).toBeVisible()

  // "Edit" action should NOT be present for external invoices
  await expect(page.getByTestId(`invoice-edit-${invoiceId}`)).not.toBeVisible()

  // "Issue" action should NOT be present
  await expect(page.getByTestId(`invoice-issue-${invoiceId}`)).not.toBeVisible()

  // "Void" action should NOT be present
  await expect(page.getByTestId(`invoice-void-${invoiceId}`)).not.toBeVisible()

  // "Mark Paid" action should NOT be present
  await expect(page.getByTestId(`invoice-mark-paid-${invoiceId}`)).not.toBeVisible()

  // "View in Provider" may be present if the invoice has an external URL
  // We do not assert its presence since not all external invoices have URLs

  // Close the menu by pressing Escape
  await page.keyboard.press('Escape')
}

// ---------------------------------------------------------------------------
// Verification: Provider Banner in Detail Dialog
// ---------------------------------------------------------------------------

/**
 * Verify that the provider banner is visible in the invoice detail dialog.
 *
 * The banner displays "This invoice is managed by {Provider}" for external invoices.
 *
 * Prerequisite: the invoice detail dialog is open.
 *
 * @param provider - Provider key, e.g. 'stripe' or 'creem'
 */
export async function verifyProviderBannerInDetail(
  page: Page,
  provider: string,
): Promise<void> {
  const banner = page.getByTestId('invoice-external-provider-banner')
  await expect(banner).toBeVisible()
  // The banner contains the provider label text
  const label = provider.charAt(0).toUpperCase() + provider.slice(1)
  await expect(banner).toContainText(label)
}

/**
 * Verify that NO provider banner is visible in the invoice detail dialog.
 *
 * Used for manual invoices where the external provider banner should not appear.
 *
 * Prerequisite: the invoice detail dialog is open.
 */
export async function verifyNoProviderBannerInDetail(
  page: Page,
): Promise<void> {
  await expect(page.getByTestId('invoice-external-provider-banner')).not.toBeVisible()
}
