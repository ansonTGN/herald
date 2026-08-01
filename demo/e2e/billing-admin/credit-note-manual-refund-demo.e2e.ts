/**
 * Admin Manual Credit Note Refund Demo Tests
 *
 * User Story:
 * - US-IF-010: Realm Admin records an offline refund for a paid manual invoice
 *   and views refund summary / manual credit note list.
 *
 * Covers:
 * - Main refund flow: create paid manual invoice, record refund, verify chip,
 *   detail breakdown, and active credit note row.
 * - Over-remaining validation: refund amount exceeding balance is rejected.
 * - Non-paid status: Record Refund button is not rendered.
 */

import { test, cleanupTestData, expect } from '../fixtures/demo-page.fixtures'
import { DEMO_ADMIN, loginAsAdmin } from '../helpers/auth'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import {
  navigateToInvoiceAdminPage,
  createInvoice,
  issueInvoice,
} from './helpers/invoice-helpers'
import {
  closeInvoiceDetailDialog,
  createAndPayManualInvoice,
  getAccountIdByEmail,
  getInvoiceIdByNumber,
  openInvoiceDetailDialogByNumber,
  recordRefundViaDialog,
  verifyCreditNoteInManualList,
  verifyRefundChipInAdminTable,
} from './helpers/credit-note-helpers'

test.describe('[Billing Admin] Manual Credit Note Refund Demo Tests', () => {
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

  test('US-IF-010: record a manual refund and verify refund summary / credit note', async ({
    page,
    demoLogger,
  }) => {
    let accountId: string
    let invoiceNumber: string
    let invoiceId: string

    await test.step('Given: admin is logged in and on invoice admin page', async () => {
      accountId = getAccountIdByEmail(DEMO_ADMIN.realmId, DEMO_ADMIN.email)
      await loginAsAdmin(page, { realmId: DEMO_ADMIN.realmId, waitNavigation: true })
      await navigateToInvoiceAdminPage(page, DEMO_ADMIN.realmId)
    })

    await test.step('When: create and pay a manual invoice for $100.00', async () => {
      invoiceNumber = await createAndPayManualInvoice(page, DEMO_ADMIN.realmId, {
        accountId,
        billingName: 'Manual Refund Demo',
        billingEmail: 'manual-refund-demo@example.com',
        sellerName: 'Herald Demo',
        lineItems: [{ name: 'Service', quantity: '1', unitPrice: 10000 }],
        dueDate: '2026-12-31',
      })
      invoiceId = getInvoiceIdByNumber(DEMO_ADMIN.realmId, invoiceNumber)
      console.log(`[Test] Paid manual invoice created: ${invoiceNumber} (${invoiceId})`)
    })

    await test.step('Then: no refund chip is shown before refund', async () => {
      await expect(page.getByTestId(`invoice-refund-chip-${invoiceId}`)).not.toBeVisible()
    })

    await test.step('When: open invoice detail dialog', async () => {
      await openInvoiceDetailDialogByNumber(page, invoiceNumber)
    })

    await test.step('Then: Record Refund button is visible for paid manual invoice', async () => {
      await expect(page.getByTestId('record-refund-button')).toBeVisible({ timeout: 5000 })
    })

    await test.step('When: record a partial refund of $30.00', async () => {
      await recordRefundViaDialog(page, { amount: 30, reason: 'Partial refund' })
    })

    await test.step('Then: refund dialog closes', async () => {
      await expect(page.getByTestId('record-refund-dialog')).toBeHidden({ timeout: 10000 })
    })

    await test.step('When: close invoice detail dialog to return to admin table', async () => {
      await closeInvoiceDetailDialog(page)
    })

    await test.step('Then: refund chip appears in the admin table', async () => {
      await verifyRefundChipInAdminTable(page, DEMO_ADMIN.realmId, invoiceNumber)
    })

    await test.step('When: reopen invoice detail dialog', async () => {
      await openInvoiceDetailDialogByNumber(page, invoiceNumber)
    })

    await test.step('Then: refund summary shows refunded and remaining amounts', async () => {
      await expect(page.getByTestId('invoice-refund-summary')).toBeVisible({ timeout: 5000 })
      await expect(page.getByTestId('invoice-refunded-amount')).toHaveText('-CN¥30.00')
      await expect(page.getByTestId('invoice-remaining-amount')).toHaveText('CN¥70.00')
    })

    await test.step('Then: manual credit note list contains the active credit note', async () => {
      await verifyCreditNoteInManualList(page, { amount: 3000, reason: 'Partial refund' })

      const manualList = page.getByTestId('credit-note-list-manual')
      await expect(manualList.locator('[data-testid^="credit-note-voided-"]')).toHaveCount(0)
    })

    await demoLogger.testCode.log('Manual credit note refund main flow verified')
  })

  test('US-IF-010: reject refund amount exceeding remaining balance', async ({
    page,
    demoLogger,
  }) => {
    let accountId: string
    let invoiceNumber: string

    await test.step('Given: admin is logged in and on invoice admin page', async () => {
      accountId = getAccountIdByEmail(DEMO_ADMIN.realmId, DEMO_ADMIN.email)
      await loginAsAdmin(page, { realmId: DEMO_ADMIN.realmId, waitNavigation: true })
      await navigateToInvoiceAdminPage(page, DEMO_ADMIN.realmId)
    })

    await test.step('When: create and pay a manual invoice for $50.00', async () => {
      invoiceNumber = await createAndPayManualInvoice(page, DEMO_ADMIN.realmId, {
        accountId,
        billingName: 'Over Refund Validation',
        billingEmail: 'over-refund-validation@example.com',
        sellerName: 'Herald Demo',
        lineItems: [{ name: 'Service', quantity: '1', unitPrice: 5000 }],
        dueDate: '2026-12-31',
      })
    })

    await test.step('And: open invoice detail dialog', async () => {
      await openInvoiceDetailDialogByNumber(page, invoiceNumber)
      await expect(page.getByTestId('record-refund-button')).toBeVisible({ timeout: 5000 })
    })

    await test.step('When: attempt to record a $60.00 refund', async () => {
      await page.getByTestId('record-refund-button').click()
      await expect(page.getByTestId('record-refund-dialog')).toBeVisible({ timeout: 5000 })
      await page.getByTestId('record-refund-amount-input').fill('60.00')
      await page.getByTestId('record-refund-reason-input').fill('Over refund')
      await page.getByTestId('record-refund-submit-button').click()
    })

    await test.step('Then: error message is shown and dialog stays open', async () => {
      await expect(page.getByTestId('record-refund-error-message')).toBeVisible({ timeout: 5000 })
      await expect(page.getByTestId('record-refund-dialog')).toBeVisible({ timeout: 5000 })
    })

    await demoLogger.testCode.log('Over-remaining refund validation verified')
  })

  test('US-IF-010: hide Record Refund button for non-paid invoices', async ({
    page,
    demoLogger,
  }) => {
    let accountId: string
    let invoiceNumber: string

    await test.step('Given: admin is logged in and on invoice admin page', async () => {
      accountId = getAccountIdByEmail(DEMO_ADMIN.realmId, DEMO_ADMIN.email)
      await loginAsAdmin(page, { realmId: DEMO_ADMIN.realmId, waitNavigation: true })
      await navigateToInvoiceAdminPage(page, DEMO_ADMIN.realmId)
    })

    await test.step('When: create and issue a manual invoice without marking paid', async () => {
      invoiceNumber = await createInvoice(page, DEMO_ADMIN.realmId, {
        accountId,
        billingName: 'Unpaid Refund Button Check',
        billingEmail: 'unpaid-refund-button-check@example.com',
        sellerName: 'Herald Demo',
        lineItems: [{ name: 'Service', quantity: '1', unitPrice: 3000 }],
        dueDate: '2026-12-31',
      })
      await issueInvoice(page, invoiceNumber)
    })

    await test.step('Then: Record Refund button is not rendered in detail dialog', async () => {
      await openInvoiceDetailDialogByNumber(page, invoiceNumber)
      await expect(page.getByTestId('record-refund-button')).toHaveCount(0)
    })

    await demoLogger.testCode.log('Non-paid invoice hides Record Refund button')
  })
})
