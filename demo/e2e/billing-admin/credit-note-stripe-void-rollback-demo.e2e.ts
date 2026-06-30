/**
 * Stripe Credit Note Void Rollback Demo Test
 *
 * User Story:
 * - US-IF-011: Stripe Credit Note void rollback via mock webhook.
 *
 * Covers:
 * - Seeding a paid Stripe external invoice.
 * - Delivering a mock `credit_note.created` webhook and verifying refund summary
 *   and active credit note row.
 * - Delivering a mock `credit_note.voided` webhook and verifying the refund
 *   summary disappears, the credit note row shows a voided test id, and the
 *   refund chip is removed from the admin table.
 */

import { execSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { test, cleanupTestData, expect } from '../fixtures/demo-page.fixtures'
import { DEMO_ADMIN, loginAsAdmin } from '../helpers/auth'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { navigateToInvoiceAdminPage } from './helpers/invoice-helpers'
import {
  getInvoiceIdByNumber,
  openInvoiceDetailDialogByNumber,
  seedPaidExternalInvoice,
  verifyRefundChipInAdminTable,
} from './helpers/credit-note-helpers'
import {
  buildStripeCreditNoteCreatedPayload,
  buildStripeCreditNoteVoidedPayload,
  deliverStripeCreditNoteWebhook,
} from './helpers/credit-note-webhook-helpers'

const POSTGRES_CONTAINER = 'cas-demo-postgres'

/**
 * Query the UUID of a local Stripe credit note by its external Stripe id.
 */
function getCreditNoteIdByExternalId(realmId: string, externalCreditNoteId: string): string {
  const id = execSync(
    `docker exec -i ${POSTGRES_CONTAINER} psql -U postgres -d herald_demo -t -A --set ON_ERROR_STOP=on`,
    {
      input: `SELECT id FROM credit_note WHERE realm_id = '${realmId}' AND external_credit_note_id = '${externalCreditNoteId}'`,
      encoding: 'utf-8',
      timeout: 10000,
    },
  ).trim()

  if (!id) {
    throw new Error(`Credit note not found: ${externalCreditNoteId} in realm ${realmId}`)
  }

  return id
}

test.describe('[Billing Admin] Stripe Credit Note Void Rollback Demo Tests', () => {
  test.beforeEach(async ({ page }) => {
    expect(DEMO_ADMIN.realmId, 'DEMO_ADMIN.realmId must be configured').toBeTruthy()
    expect(DEMO_ADMIN.email, 'DEMO_ADMIN.email must be configured').toBeTruthy()

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

  test('US-IF-011: Stripe credit note created then voided rolls back refund UI', async ({
    page,
    request,
    demoLogger,
  }) => {
    const externalInvoiceId = 'in_demo_credit_note_void_rollback'
    const eventIdCreated = `evt_${randomUUID()}`
    const creditNoteId = `cn_${randomUUID()}`

    let invoiceNumber: string
    let invoiceId: string

    await test.step('Given: admin is logged in and on invoice admin page', async () => {
      await loginAsAdmin(page, { realmId: DEMO_ADMIN.realmId, waitNavigation: true })
      await navigateToInvoiceAdminPage(page, DEMO_ADMIN.realmId)
    })

    await test.step('When: seed a paid Stripe external invoice for $100.00', async () => {
      const result = seedPaidExternalInvoice(DEMO_ADMIN.realmId, {
        provider: 'stripe',
        total: 10000,
        externalInvoiceId,
      })
      invoiceNumber = result.invoiceNumber
      invoiceId = getInvoiceIdByNumber(DEMO_ADMIN.realmId, invoiceNumber)
      console.log(`[Test] Paid Stripe invoice seeded: ${invoiceNumber} (${invoiceId})`)
    })

    await test.step('Then: detail dialog shows no refund summary or Stripe credit notes initially', async () => {
      await openInvoiceDetailDialogByNumber(page, invoiceNumber)
      await expect(page.getByTestId('invoice-refund-summary')).not.toBeVisible()
      await expect(page.getByTestId('credit-note-list-stripe')).not.toBeVisible()
    })

    await test.step('When: deliver credit_note.created webhook for $50.00', async () => {
      const payload = buildStripeCreditNoteCreatedPayload({
        eventId: eventIdCreated,
        creditNoteId,
        invoiceId: externalInvoiceId,
        total: 5000,
        currency: 'USD',
      })
      const result = await deliverStripeCreditNoteWebhook(request, DEMO_ADMIN.realmId, payload)
      expect(result.status, `credit_note.created webhook failed: ${result.body}`).toBe(200)
    })

    await test.step('Then: refund summary and active Stripe credit note are shown', async () => {
      await page.reload({ waitUntil: 'networkidle' })
      await openInvoiceDetailDialogByNumber(page, invoiceNumber)

      await expect(page.getByTestId('invoice-refund-summary')).toBeVisible({ timeout: 10000 })
      await expect(page.getByTestId('invoice-refunded-amount')).toHaveText('-$50.00')
      await expect(page.getByTestId('invoice-remaining-amount')).toHaveText('$50.00')

      const stripeList = page.getByTestId('credit-note-list-stripe')
      await expect(stripeList).toBeVisible({ timeout: 10000 })
      await expect(stripeList.locator('[data-testid^="credit-note-voided-"]')).toHaveCount(0)
    })

    await test.step('And: refund chip is visible in the admin table', async () => {
      await verifyRefundChipInAdminTable(page, invoiceNumber)
    })

    await test.step('When: deliver credit_note.voided webhook for the same credit note', async () => {
      const payload = buildStripeCreditNoteVoidedPayload({
        eventId: `evt_${randomUUID()}`,
        creditNoteId,
        invoiceId: externalInvoiceId,
        total: 5000,
      })
      const result = await deliverStripeCreditNoteWebhook(request, DEMO_ADMIN.realmId, payload)
      expect(result.status, `credit_note.voided webhook failed: ${result.body}`).toBe(200)
    })

    await test.step('Then: refund summary disappears and the credit note row is voided', async () => {
      await page.reload({ waitUntil: 'networkidle' })
      await openInvoiceDetailDialogByNumber(page, invoiceNumber)

      await expect(page.getByTestId('invoice-refund-summary')).not.toBeVisible()

      const noteId = getCreditNoteIdByExternalId(DEMO_ADMIN.realmId, creditNoteId)
      await expect(page.getByTestId(`credit-note-voided-${noteId}`)).toBeVisible({ timeout: 10000 })
    })

    await test.step('And: refund chip is no longer visible in the admin table', async () => {
      await expect(page.getByTestId(`invoice-refund-chip-${invoiceId}`)).not.toBeVisible()
    })

    await demoLogger.testCode.log('Stripe credit note void rollback UI verified')
  })
})
