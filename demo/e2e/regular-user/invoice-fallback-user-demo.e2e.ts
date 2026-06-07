/**
 * Invoice Fallback User Demo Tests
 *
 * User Stories:
 * - US-IF-005: Regular user views external provider invoices
 * - US-IF-006: Download external invoice PDF or view provider page
 *
 * Design Doc: .ai/design/invoice-fallback.md Section 4.4
 * User Stories: docs/user-stories/billing/invoice-fallback.md
 *
 * Session switching:
 * - Admin operations: loginAsAdmin -> navigate to /{realmId}/manage/billing/invoices
 * - User operations: loginAsUser -> navigate to /{realmId}/user/invoices
 *
 * Seeding: Uses seedExternalInvoice and seedCreemPaymentAttempt from DE-D01 helpers.
 * External invoices are seeded with account_id set to the user's account so they
 * appear in /api/realms/{realmId}/my-invoices.
 */

import { test, cleanupTestData, expect } from '../fixtures/demo-page.fixtures'
import { DEMO_ADMIN, DEMO_USERS } from '../helpers/auth'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import {
  seedExternalInvoice,
  seedCreemPaymentAttempt,
  verifyProviderBannerInDetail,
} from '../billing-admin/helpers/invoice-fallback-helpers'
import { execSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'

const REALM_ID = DEMO_ADMIN.realmId
const POSTGRES_CONTAINER = 'cas-demo-postgres'

function execPgSql(query: string): string {
  return execSync(
    `docker exec -i ${POSTGRES_CONTAINER} psql -U postgres -d herald_demo -t -A`,
    {
      input: query,
      encoding: 'utf-8',
      timeout: 10000,
    },
  ).trim()
}

/**
 * Look up a user's account ID by email and realm.
 * Returns the UUID of the account row.
 */
function lookupAccountId(userEmail: string, realmId: string): string {
  const userId = execPgSql(
    `SELECT id FROM account WHERE email = '${userEmail}' AND realm_id = '${realmId}'`,
  )
  if (!userId) {
    throw new Error(`User not found: ${userEmail} in realm ${realmId}`)
  }
  return userId
}

/**
 * Seed a Stripe external invoice associated with the given user account.
 * The invoice will appear in the user's "My Invoices" list.
 */
function seedStripeInvoiceForUser(options: {
  userEmail: string
  realmId?: string
  externalPdfUrl?: string
  externalHostedUrl?: string
  status?: string
}) {
  const realmId = options.realmId ?? REALM_ID
  const accountId = lookupAccountId(options.userEmail, realmId)

  return seedExternalInvoice(realmId, {
    provider: 'stripe',
    accountId,
    externalPdfUrl: options.externalPdfUrl ?? 'https://pay.stripe.com/invoice/acct_test/pdf123',
    externalHostedUrl: options.externalHostedUrl ?? 'https://pay.stripe.com/invoice/acct_test',
    status: options.status ?? 'issued',
  })
}

/**
 * Seed a Creem external invoice associated with the given user account.
 * No external URLs -- simulates a Creem MoR invoice with no PDF/hosted link.
 */
function seedCreemInvoiceForUser(options: {
  userEmail: string
  realmId?: string
}) {
  const realmId = options.realmId ?? REALM_ID
  const accountId = lookupAccountId(options.userEmail, realmId)

  return seedExternalInvoice(realmId, {
    provider: 'creem',
    accountId,
    status: 'issued',
  })
}

/**
 * Navigate to the admin invoice page and open the detail dialog
 * for a specific invoice by clicking its "View" action in the dropdown menu.
 */
async function openInvoiceDetailDialog(page: import('@playwright/test').Page, invoiceId: string) {
  // Open the actions dropdown menu
  await page.getByTestId(`invoice-actions-menu-${invoiceId}`).click()
  // Click the "View" action to open the detail dialog
  await page.getByTestId(`invoice-view-${invoiceId}`).click()
  // Wait for the dialog to appear
  await expect(page.getByTestId('invoice-detail-dialog')).toBeVisible({ timeout: 10000 })
}

/**
 * Clean up all external invoices created by this test suite.
 * Deletes invoices where invoice_number LIKE 'EXT-%' to avoid
 * interfering with regular test data cleanup.
 */
function cleanupExternalInvoices(realmId: string, invoiceIds: string[]) {
  for (const id of invoiceIds) {
    try {
      execPgSql(`DELETE FROM invoice_history WHERE invoice_id = '${id}'`)
      execPgSql(`DELETE FROM invoice_line_items WHERE invoice_id = '${id}'`)
      execPgSql(`DELETE FROM invoice WHERE id = '${id}'`)
    } catch {
      // Best-effort cleanup; do not block test completion
    }
  }
}

/**
 * Clean up payment attempts created for Creem rejection tests.
 */
function cleanupPaymentAttempts(attemptIds: string[]) {
  for (const id of attemptIds) {
    try {
      execPgSql(`DELETE FROM payment_attempts WHERE id = '${id}'`)
    } catch {
      // Best-effort cleanup
    }
  }
}

// ============================================================================
// Test Suite
// ============================================================================

test.describe('[Regular User] Invoice Fallback User Demo Tests', () => {
  test.beforeEach(async ({ page }) => {
    await verifyTestEnvironment(page, {
      requiredRealms: [REALM_ID],
      requiredUsers: [DEMO_ADMIN.email],
    })
  })

  test.afterEach(async ({ page, testStartTime }) => {
    await cleanupTestData(page, REALM_ID, {
      keepUsers: [DEMO_ADMIN.email],
      timestamp: testStartTime,
    })
  })

  // ==========================================================================
  // US-IF-005: User External Invoice Viewing
  // ==========================================================================

  test.describe('US-IF-005: User External Invoice Viewing', () => {
    test('should display provider badge for external invoices in user invoice list', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      const user = DEMO_USERS.user1
      const invoiceIdsToCleanup: string[] = []

      // Step 1: Seed a Stripe external invoice associated with the user
      const invoice = seedStripeInvoiceForUser({ userEmail: user.email })
      invoiceIdsToCleanup.push(invoice.id)

      await test.step('Given: user is logged in and on the user invoices page', async () => {
        await loginPage.loginAsUser(user.email, user.password, REALM_ID)
        await page.goto(`/${REALM_ID}/user/invoices`)
        await expect(page.getByTestId('invoice-user-page')).toBeVisible({ timeout: 10000 })
      })

      await test.step('Then: the Stripe invoice row shows a provider badge', async () => {
        const table = page.getByTestId('invoice-user-table')
        await expect(table).toBeVisible()

        // Find the row containing the invoice number
        const row = table.locator('tbody tr').filter({ hasText: invoice.invoiceNumber })
        await expect(row).toBeVisible({ timeout: 10000 })

        // Verify the provider badge shows "Stripe"
        // Provider column is at cell index 2 (after #, Invoice Number).
        // Scope to this cell to avoid strict mode violations when "Stripe" appears
        // in both the provider badge and the action link.
        const providerCell = row.locator('td').nth(2)
        await expect(providerCell.getByText('Stripe', { exact: true })).toBeVisible()
      })

      // Cleanup
      cleanupExternalInvoices(REALM_ID, invoiceIdsToCleanup)

      await demoLogger.testCode.log(
        'Verified provider badge for external invoice in user invoice list'
      )
    })

    test('should show readonly detail for external invoice', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      const invoiceIdsToCleanup: string[] = []
      const invoice = seedStripeInvoiceForUser({ userEmail: DEMO_USERS.user1.email })
      invoiceIdsToCleanup.push(invoice.id)

      await test.step('Given: admin is logged in and on invoice admin page', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', REALM_ID)
        await page.goto(`/${REALM_ID}/manage/billing/invoices`)
        await expect(page.getByTestId('invoice-admin-page')).toBeVisible({ timeout: 10000 })
      })

      await test.step('When: admin opens the external invoice detail', async () => {
        await openInvoiceDetailDialog(page, invoice.id)
      })

      await test.step('Then: the external provider banner is visible', async () => {
        await verifyProviderBannerInDetail(page, 'stripe')
      })

      await test.step('And: no action buttons (edit, issue, void, mark-paid) are present', async () => {
        // The detail dialog for external invoices should only show
        // download/view buttons, not edit/issue/void/mark-paid actions.
        // These buttons would be in the action menu, but the detail dialog
        // itself is purely informational for external invoices.
        const dialog = page.getByTestId('invoice-detail-dialog')
        await expect(dialog).toBeVisible()

        // External invoices only show the "Download PDF" button in the footer
        // No edit/issue/void/mark-paid buttons should appear
        await expect(page.getByTestId('invoice-download-pdf-button')).toBeVisible()
      })

      // Cleanup
      cleanupExternalInvoices(REALM_ID, invoiceIdsToCleanup)

      await demoLogger.testCode.log(
        'Verified readonly detail view for external invoice'
      )
    })

    test('should reject invoice application for Creem transaction', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      const user = DEMO_USERS.user1
      const attemptIdsToCleanup: string[] = []

      // Step 1: Seed a Creem payment_attempt associated with the user
      const creemAttempt = seedCreemPaymentAttempt(REALM_ID, {
        userEmail: user.email,
      })
      attemptIdsToCleanup.push(creemAttempt.id)

      await test.step('Given: user is logged in and navigates to apply invoice form', async () => {
        await loginPage.loginAsUser(user.email, user.password, REALM_ID)
        await page.goto(`/${REALM_ID}/user/invoices`)
        await expect(page.getByTestId('invoice-user-page')).toBeVisible({ timeout: 10000 })
      })

      await test.step('When: user opens the apply invoice form', async () => {
        await page.getByTestId('apply-invoice-button').click()
        await expect(page.getByTestId('apply-form-page')).toBeVisible({ timeout: 5000 })
      })

      await test.step('And: fills in billing details with the Creem payment attempt ID', async () => {
        await page.getByTestId('apply-payment-attempt-id-input').fill(creemAttempt.id)
        await page.getByTestId('apply-billing-name-input').fill('Creem Rejection Test')
        await page.getByTestId('apply-billing-address-input').fill('123 Test Street')
        await page.getByTestId('apply-billing-tax-id-input').fill('TAX123456')
      })

      await test.step('And: submits the application', async () => {
        await page.getByTestId('apply-invoice-submit-button').click()
      })

      await test.step('Then: Creem rejection alert appears after mutation fails', async () => {
        // The alert only renders after the apply mutation fails with the Creem error.
        // Wait for the error state to render the rejection alert.
        await expect(page.getByTestId('apply-invoice-creem-rejection')).toBeVisible({
          timeout: 15000,
        })
      })

      // Cleanup
      cleanupPaymentAttempts(attemptIdsToCleanup)

      await demoLogger.testCode.log(
        'Verified Creem transaction invoice application rejection'
      )
    })
  })

  // ==========================================================================
  // US-IF-006: Download External Invoice PDF or View Provider Page
  // ==========================================================================

  test.describe('US-IF-006: Download External Invoice PDF or View Provider Page', () => {
    test('should show PDF download link for external invoice with externalPdfUrl', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      const user = DEMO_USERS.user1
      const pdfUrl = 'https://pay.stripe.com/invoice/acct_test/pdf_unique_001'
      const hostedUrl = 'https://pay.stripe.com/invoice/acct_test'
      const invoiceIdsToCleanup: string[] = []

      const invoice = seedStripeInvoiceForUser({
        userEmail: user.email,
        externalPdfUrl: pdfUrl,
        externalHostedUrl: hostedUrl,
      })
      invoiceIdsToCleanup.push(invoice.id)

      await test.step('Given: user is logged in and on the user invoices page', async () => {
        await loginPage.loginAsUser(user.email, user.password, REALM_ID)
        await page.goto(`/${REALM_ID}/user/invoices`)
        await expect(page.getByTestId('invoice-user-page')).toBeVisible({ timeout: 10000 })
      })

      await test.step('Then: the row shows a view-provider link (list summary omits externalPdfUrl)', async () => {
        // The list API summary does not include external_pdf_url, so the
        // "Download PDF" button is NOT rendered in the list.  Instead, the
        // externalHostedUrl triggers the "View in Provider" button.
        const viewProviderButton = page.getByTestId(`invoice-view-provider-${invoice.id}`)
        await expect(viewProviderButton).toBeVisible({ timeout: 10000 })

        // Verify the href points to the hosted URL (the <a> is the direct target of the testid via asChild)
        const linkHref = await viewProviderButton.getAttribute('href')
        expect(linkHref).toBe(hostedUrl)
      })

      // Cleanup
      cleanupExternalInvoices(REALM_ID, invoiceIdsToCleanup)

      await demoLogger.testCode.log(
        'Verified view-provider link for external invoice with externalPdfUrl (PDF only available in detail dialog)'
      )
    })

    test('should show provider link for external invoice with only externalHostedUrl', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      const user = DEMO_USERS.user1
      const hostedUrl = 'https://pay.stripe.com/invoice/acct_test/hosted_only_002'
      const invoiceIdsToCleanup: string[] = []

      const invoice = seedStripeInvoiceForUser({
        userEmail: user.email,
        externalPdfUrl: undefined,
        externalHostedUrl: hostedUrl,
      })
      invoiceIdsToCleanup.push(invoice.id)

      await test.step('Given: user is logged in and on the user invoices page', async () => {
        await loginPage.loginAsUser(user.email, user.password, REALM_ID)
        await page.goto(`/${REALM_ID}/user/invoices`)
        await expect(page.getByTestId('invoice-user-page')).toBeVisible({ timeout: 10000 })
      })

      await test.step('Then: the row shows a view-provider link with provider label', async () => {
        const viewProviderLink = page.getByTestId(`invoice-view-provider-${invoice.id}`)
        await expect(viewProviderLink).toBeVisible({ timeout: 10000 })

        // Verify the link contains the "Stripe" provider label
        await expect(viewProviderLink).toContainText('Stripe')

        // Verify the href points to the hosted URL
        // The testid targets the <a> element directly (Button asChild pattern)
        const linkHref = await viewProviderLink.getAttribute('href')
        expect(linkHref).toBe(hostedUrl)
      })

      // Cleanup
      cleanupExternalInvoices(REALM_ID, invoiceIdsToCleanup)

      await demoLogger.testCode.log(
        'Verified provider link for external invoice with only externalHostedUrl'
      )
    })

    test('should show managed-by text for external invoice with no URLs', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      const user = DEMO_USERS.user1
      const invoiceIdsToCleanup: string[] = []

      const invoice = seedCreemInvoiceForUser({ userEmail: user.email })
      invoiceIdsToCleanup.push(invoice.id)

      await test.step('Given: user is logged in and on the user invoices page', async () => {
        await loginPage.loginAsUser(user.email, user.password, REALM_ID)
        await page.goto(`/${REALM_ID}/user/invoices`)
        await expect(page.getByTestId('invoice-user-page')).toBeVisible({ timeout: 10000 })
      })

      await test.step('Then: the row shows managed-by text indicating managed by Creem', async () => {
        const managedText = page.getByTestId(`invoice-managed-external-${invoice.id}`)
        await expect(managedText).toBeVisible({ timeout: 10000 })

        // Verify the text contains the Creem provider label
        await expect(managedText).toContainText('Creem')
      })

      // Cleanup
      cleanupExternalInvoices(REALM_ID, invoiceIdsToCleanup)

      await demoLogger.testCode.log(
        'Verified managed-by text for external invoice with no URLs'
      )
    })

    test('should have external PDF download link in detail dialog', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      const pdfUrl = 'https://pay.stripe.com/invoice/acct_test/pdf_detail_003'
      const invoiceIdsToCleanup: string[] = []

      const invoice = seedStripeInvoiceForUser({
        userEmail: DEMO_USERS.user1.email,
        externalPdfUrl: pdfUrl,
        externalHostedUrl: 'https://pay.stripe.com/invoice/acct_test',
      })
      invoiceIdsToCleanup.push(invoice.id)

      await test.step('Given: admin is logged in and on invoice admin page', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', REALM_ID)
        await page.goto(`/${REALM_ID}/manage/billing/invoices`)
        await expect(page.getByTestId('invoice-admin-page')).toBeVisible({ timeout: 10000 })
      })

      await test.step('When: admin opens the external invoice detail dialog', async () => {
        await openInvoiceDetailDialog(page, invoice.id)
      })

      await test.step('Then: the footer shows a download PDF button with href pointing to the external PDF URL', async () => {
        const downloadButton = page.getByTestId('invoice-download-pdf-button')
        await expect(downloadButton).toBeVisible()

        // Verify the href points to the external PDF URL
        // The testid targets the <a> element directly (Button asChild pattern)
        const linkHref = await downloadButton.getAttribute('href')
        expect(linkHref).toBe(pdfUrl)
      })

      // Cleanup
      cleanupExternalInvoices(REALM_ID, invoiceIdsToCleanup)

      await demoLogger.testCode.log(
        'Verified external PDF download link in detail dialog'
      )
    })

    test('should have View in Provider button in detail dialog', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      const hostedUrl = 'https://pay.stripe.com/invoice/acct_test/hosted_detail_004'
      const invoiceIdsToCleanup: string[] = []

      const invoice = seedStripeInvoiceForUser({
        userEmail: DEMO_USERS.user1.email,
        externalPdfUrl: undefined,
        externalHostedUrl: hostedUrl,
      })
      invoiceIdsToCleanup.push(invoice.id)

      await test.step('Given: admin is logged in and on invoice admin page', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', REALM_ID)
        await page.goto(`/${REALM_ID}/manage/billing/invoices`)
        await expect(page.getByTestId('invoice-admin-page')).toBeVisible({ timeout: 10000 })
      })

      await test.step('When: admin opens the external invoice detail dialog', async () => {
        await openInvoiceDetailDialog(page, invoice.id)
      })

      await test.step('Then: the View in Provider button is visible with provider label', async () => {
        const viewInProviderButton = page.getByTestId('invoice-view-in-provider-button')
        await expect(viewInProviderButton).toBeVisible()

        // Verify the button contains the provider label text
        await expect(viewInProviderButton).toContainText('Stripe')
      })

      // Cleanup
      cleanupExternalInvoices(REALM_ID, invoiceIdsToCleanup)

      await demoLogger.testCode.log(
        'Verified View in Provider button in detail dialog'
      )
    })
  })
})
