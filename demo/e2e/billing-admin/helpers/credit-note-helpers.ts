/**
 * Credit Note Demo Helpers
 *
 * Shared helper functions for Credit Note demo/E2E tests.
 * Covers: manual invoice lifecycle, refund dialog interaction, admin table
 * verification, and DB seeding for credit notes / external invoices.
 *
 * All amounts are in smallest currency unit (cents) in DB/code.
 * UI amount inputs use major-currency display units (e.g. "50.00" for 5000 cents).
 */

import { execSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { Page, expect } from '@playwright/test'
import {
  createInvoice,
  issueInvoice,
  markPaidInvoice,
  type InvoiceCreateData,
} from './invoice-helpers'

const POSTGRES_CONTAINER = 'cas-demo-postgres'

export interface ManualCreditNoteOptions {
  /** Refund amount in smallest currency unit (cents). */
  amount: number
  /** ISO 4217 currency code. Defaults to 'USD'. */
  currency?: string
  /** Refund reason / memo. */
  reason?: string
  /** UUID of the admin creating the credit note. */
  createdByUserId?: string
}

export interface StripeCreditNoteOptions {
  /** Refund amount in smallest currency unit (cents). */
  amount: number
  /** ISO 4217 currency code. Defaults to 'USD'. */
  currency?: string
  /** Stripe credit note ID (cn_xxx). Defaults to generated. */
  externalCreditNoteId?: string
}

export interface PaidExternalInvoiceOptions {
  provider: 'stripe' | 'creem'
  /** External invoice ID (e.g. Stripe in_xxx). Defaults to generated. */
  externalInvoiceId?: string
  /** External order ID (e.g. Creem order ID). Defaults to undefined. */
  externalOrderId?: string
  /** External hosted page URL. Stripe invoices typically have this. */
  externalHostedUrl?: string
  /** External PDF download URL. Stripe invoices typically have this. */
  externalPdfUrl?: string
  /** Currency code. Defaults to 'USD'. */
  currency?: string
  /** Total amount in smallest currency unit. Defaults to 1000. */
  total?: number
  /** Account ID to associate the invoice with a user. */
  accountId?: string
}

export interface PaidExternalInvoiceResult {
  id: string
  invoiceNumber: string
  provider: string
  externalInvoiceId: string
}

/**
 * Execute a PostgreSQL query inside the demo Postgres container.
 *
 * `--set ON_ERROR_STOP=on` makes psql exit non-zero on any SQL error so a
 * failed INSERT/SELECT surfaces via execSync instead of being silently swallowed.
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

/** Resolve an invoice UUID by invoice number. */
export function getInvoiceIdByNumber(realmId: string, invoiceNumber: string): string {
  const id = execPgSql(
    `SELECT id FROM invoice WHERE realm_id = '${realmId}' AND invoice_number = '${invoiceNumber}'`,
  )
  if (!id) {
    throw new Error(`Invoice not found: ${invoiceNumber} in realm ${realmId}`)
  }
  return id
}

/** Resolve an account UUID by email. */
export function getAccountIdByEmail(realmId: string, email: string): string {
  const id = execPgSql(
    `SELECT id FROM account WHERE realm_id = '${realmId}' AND email = '${email}'`,
  )
  if (!id) {
    throw new Error(`Account not found: ${email} in realm ${realmId}`)
  }
  return id
}

/**
 * Create, issue, and pay a manual invoice.
 *
 * Prerequisite: user is logged in as a realm admin on the invoice admin page.
 */
export async function createAndPayManualInvoice(
  page: Page,
  realmId: string,
  data: InvoiceCreateData,
): Promise<string> {
  const invoiceNumber = await createInvoice(page, realmId, data)
  await issueInvoice(page, invoiceNumber)
  await markPaidInvoice(page, invoiceNumber)
  return invoiceNumber
}

/** Open the invoice detail dialog from the admin table by invoice number. */
export async function openInvoiceDetailDialogByNumber(
  page: Page,
  invoiceNumber: string,
): Promise<void> {
  const row = page.locator('tr').filter({ hasText: invoiceNumber }).first()
  await expect(row).toBeVisible({ timeout: 10000 })

  const menuButton = row.getByRole('button', { name: 'Open menu' })
  await menuButton.click()
  await page.getByRole('menuitem', { name: 'View' }).click()

  await expect(page.getByTestId('invoice-detail-dialog')).toBeVisible({ timeout: 10000 })
}

/**
 * Close the invoice detail dialog if it is open.
 *
 * Uses the Escape key, which triggers Radix Dialog's built-in close handler.
 */
export async function closeInvoiceDetailDialog(page: Page): Promise<void> {
  const dialog = page.getByTestId('invoice-detail-dialog')
  const isVisible = await dialog.isVisible().catch(() => false)
  if (!isVisible) {
    return
  }
  await page.keyboard.press('Escape')
  await expect(dialog).toBeHidden({ timeout: 5000 })
}

/**
 * Record a refund via the Record Refund dialog.
 *
 * Prerequisite: the invoice detail dialog is open and the record-refund button is visible.
 * `amount` is in major currency units (e.g. 50 for $50.00 = 5000 cents).
 */
export async function recordRefundViaDialog(
  page: Page,
  { amount, reason }: { amount: number; reason: string },
): Promise<void> {
  await page.getByTestId('record-refund-button').click()
  await expect(page.getByTestId('record-refund-dialog')).toBeVisible({ timeout: 5000 })

  await page.getByTestId('record-refund-amount-input').fill(amount.toFixed(2))
  await page.getByTestId('record-refund-reason-input').fill(reason)
  await page.getByTestId('record-refund-submit-button').click()

  await expect(page.getByTestId('record-refund-dialog')).toBeHidden({ timeout: 10000 })
}

/**
 * Verify the refund chip is visible for the given invoice in the admin table.
 *
 * Resolves the invoice id from the invoice number and asserts visibility of
 * `invoice-refund-chip-${invoiceId}`.
 */
export async function verifyRefundChipInAdminTable(
  page: Page,
  realmId: string,
  invoiceNumber: string,
): Promise<void> {
  const invoiceId = getInvoiceIdByNumber(realmId, invoiceNumber)
  await expect(page.getByTestId(`invoice-refund-chip-${invoiceId}`)).toBeVisible({
    timeout: 10000,
  })
}

/**
 * Verify the manual credit note list is visible and contains the expected amount and reason.
 *
 * `amount` is in smallest currency unit (cents) and is formatted as currency text
 * (e.g. "$50.00") for the assertion.
 */
export async function verifyCreditNoteInManualList(
  page: Page,
  { amount, reason }: { amount: number; reason: string },
): Promise<void> {
  const list = page.getByTestId('credit-note-list-manual')
  await expect(list).toBeVisible({ timeout: 10000 })

  const formattedAmount = formatCurrencyMajor(amount)
  await expect(list).toContainText(formattedAmount)
  await expect(list).toContainText(reason)
}

/**
 * Insert a manual credit note for an invoice and update invoice refund aggregates.
 *
 * `options.amount` is in smallest currency unit (cents).
 */
export function seedManualCreditNote(
  realmId: string,
  invoiceNumber: string,
  options: ManualCreditNoteOptions,
): void {
  const invoiceId = getInvoiceIdByNumber(realmId, invoiceNumber)
  const id = randomUUID()
  const amount = options.amount
  const currency = options.currency ?? 'USD'
  const reason = options.reason ?? 'Demo manual refund'

  execPgSql(`
    INSERT INTO credit_note (
      id, invoice_id, realm_id, amount, currency, source, status, memo, created_at
    ) VALUES (
      '${id}', '${invoiceId}'::uuid, '${realmId}', ${amount}, '${currency}', 'manual', 'active', '${reason}', NOW()
    );
    UPDATE invoice
    SET amount_refunded = amount_refunded + ${amount},
        amount_remaining = amount_remaining - ${amount},
        updated_at = NOW()
    WHERE id = '${invoiceId}'::uuid;
  `)
}

/**
 * Insert a Stripe credit note for an invoice and update invoice refund aggregates.
 *
 * `options.amount` is in smallest currency unit (cents).
 */
export function seedStripeCreditNote(
  realmId: string,
  invoiceNumber: string,
  options: StripeCreditNoteOptions,
): void {
  const invoiceId = getInvoiceIdByNumber(realmId, invoiceNumber)
  const id = randomUUID()
  const amount = options.amount
  const currency = options.currency ?? 'USD'
  const externalCreditNoteId = options.externalCreditNoteId ?? `cn_${randomUUID().slice(0, 8)}`

  execPgSql(`
    INSERT INTO credit_note (
      id, invoice_id, realm_id, amount, currency, source, status, external_credit_note_id, created_at
    ) VALUES (
      '${id}', '${invoiceId}'::uuid, '${realmId}', ${amount}, '${currency}', 'stripe', 'active', '${externalCreditNoteId}', NOW()
    );
    UPDATE invoice
    SET amount_refunded = amount_refunded + ${amount},
        amount_remaining = amount_remaining - ${amount},
        updated_at = NOW()
    WHERE id = '${invoiceId}'::uuid;
  `)
}

/**
 * Create a paid external invoice record via direct DB insert.
 *
 * Mirrors `seedExternalInvoice` but forces `status='paid'`, `amount_refunded=0`,
 * and `amount_remaining=options.total` so subsequent Credit Note webhooks can
 * pass the `amount <= amount_remaining` validation.
 */
export function seedPaidExternalInvoice(
  realmId: string,
  options: PaidExternalInvoiceOptions,
): PaidExternalInvoiceResult {
  const id = randomUUID()
  const provider = options.provider
  const externalInvoiceId = options.externalInvoiceId ?? `ext_${provider}_${id.slice(0, 8)}`
  const invoiceNumber = `EXT-${provider.toUpperCase()}-${id.slice(0, 8)}`
  const currency = options.currency ?? 'USD'
  const total = options.total ?? 1000
  const source = 'external_sync'
  const status = 'paid'

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
    'amount_refunded',
    'amount_remaining',
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
    total,
    '0',
    '0',
    '0',
    total,
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

/** Format a cent amount as major-currency text with two decimals. */
function formatCurrencyMajor(cents: number): string {
  return (cents / 100).toFixed(2)
}
