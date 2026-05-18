/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { InvoiceUserPage } from '../invoice-user-page'
import type { InvoiceResponse, InvoiceListResponse } from '@/lib/api-generated'
import { server } from '@/test/mocks/server'

// ==================== Test Helpers ====================

const REALM_ID = 'test-realm'
const BASE_URL = 'http://localhost:3000'

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  })
}

function renderWithProviders(ui: React.ReactElement, queryClient?: QueryClient) {
  const qc = queryClient ?? createTestQueryClient()
  return render(<QueryClientProvider client={qc}>{ui}</QueryClientProvider>)
}

function makeInvoice(overrides: Partial<InvoiceResponse> = {}): InvoiceResponse {
  return {
    id: 'inv-1',
    invoiceNumber: 'INV-001',
    accountId: 'acc-1',
    billingName: 'Test Buyer',
    source: 'user_application',
    status: 'issued',
    total: 9900,
    currency: 'CNY',
    dueDate: '2025-06-01T00:00:00Z',
    createdAt: '2025-05-01T00:00:00Z',
    ...overrides,
  }
}

function makeListResponse(
  invoices: InvoiceResponse[],
  overrides: Partial<InvoiceListResponse> = {}
): InvoiceListResponse {
  return {
    data: invoices,
    page: 0,
    pageSize: 20,
    total: invoices.length,
    ...overrides,
  }
}

// ==================== Tests ====================

describe('InvoiceUserPage', () => {
  const defaultInvoices: InvoiceResponse[] = [
    makeInvoice({
      id: 'inv-1',
      invoiceNumber: 'INV-001',
      status: 'issued',
      total: 9900,
      currency: 'CNY',
      dueDate: '2025-06-01T00:00:00Z',
      createdAt: '2025-05-01T00:00:00Z',
    }),
    makeInvoice({
      id: 'inv-2',
      invoiceNumber: 'INV-002',
      status: 'paid',
      total: 15000,
      currency: 'USD',
      dueDate: '2025-06-15T00:00:00Z',
      createdAt: '2025-05-05T00:00:00Z',
    }),
    makeInvoice({
      id: 'inv-3',
      invoiceNumber: 'INV-003',
      status: 'overdue',
      total: 20000,
      currency: 'CNY',
      dueDate: '2025-05-01T00:00:00Z',
      createdAt: '2025-04-01T00:00:00Z',
    }),
    makeInvoice({
      id: 'inv-4',
      invoiceNumber: 'INV-004',
      status: 'draft',
      total: 5000,
      currency: 'CNY',
      dueDate: '2025-07-01T00:00:00Z',
      createdAt: '2025-06-01T00:00:00Z',
    }),
    makeInvoice({
      id: 'inv-5',
      invoiceNumber: 'INV-005',
      status: 'void',
      total: 3000,
      currency: 'CNY',
      dueDate: '2025-01-15T00:00:00Z',
      createdAt: '2025-01-01T00:00:00Z',
    }),
  ]

  function setupDefaultHandlers() {
    server.use(
      http.get(`${BASE_URL}/api/bill/${REALM_ID}/my/invoices`, () => {
        return HttpResponse.json(makeListResponse(defaultInvoices))
      })
    )
  }

  beforeEach(() => {
    setupDefaultHandlers()
  })

  // ==================== PDF Download Visibility ====================

  describe('PDF download button visibility', () => {
    it('shows PDF download button for issued invoice', async () => {
      const invoice = makeInvoice({ id: 'inv-issued', status: 'issued' })
      server.use(
        http.get(`${BASE_URL}/api/bill/${REALM_ID}/my/invoices`, () => {
          return HttpResponse.json(makeListResponse([invoice]))
        })
      )

      renderWithProviders(<InvoiceUserPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByTestId('invoice-download-pdf-inv-issued')).toBeInTheDocument()
      })
    })

    it('shows PDF download button for paid invoice', async () => {
      const invoice = makeInvoice({ id: 'inv-paid', status: 'paid' })
      server.use(
        http.get(`${BASE_URL}/api/bill/${REALM_ID}/my/invoices`, () => {
          return HttpResponse.json(makeListResponse([invoice]))
        })
      )

      renderWithProviders(<InvoiceUserPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByTestId('invoice-download-pdf-inv-paid')).toBeInTheDocument()
      })
    })

    it('shows PDF download button for overdue invoice', async () => {
      const invoice = makeInvoice({ id: 'inv-overdue', status: 'overdue' })
      server.use(
        http.get(`${BASE_URL}/api/bill/${REALM_ID}/my/invoices`, () => {
          return HttpResponse.json(makeListResponse([invoice]))
        })
      )

      renderWithProviders(<InvoiceUserPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByTestId('invoice-download-pdf-inv-overdue')).toBeInTheDocument()
      })
    })

    it('hides PDF download button for draft invoice', async () => {
      const invoice = makeInvoice({ id: 'inv-draft', status: 'draft' })
      server.use(
        http.get(`${BASE_URL}/api/bill/${REALM_ID}/my/invoices`, () => {
          return HttpResponse.json(makeListResponse([invoice]))
        })
      )

      renderWithProviders(<InvoiceUserPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByTestId('invoice-user-table')).toBeInTheDocument()
      })

      expect(screen.queryByTestId('invoice-download-pdf-inv-draft')).not.toBeInTheDocument()
    })

    it('hides PDF download button for void invoice', async () => {
      const invoice = makeInvoice({ id: 'inv-void', status: 'void' })
      server.use(
        http.get(`${BASE_URL}/api/bill/${REALM_ID}/my/invoices`, () => {
          return HttpResponse.json(makeListResponse([invoice]))
        })
      )

      renderWithProviders(<InvoiceUserPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByTestId('invoice-user-table')).toBeInTheDocument()
      })

      expect(screen.queryByTestId('invoice-download-pdf-inv-void')).not.toBeInTheDocument()
    })
  })

  // ==================== Apply Dialog ====================

  describe('apply invoice dialog', () => {
    it('clicking "Apply for Invoice" opens the apply dialog', async () => {
      const user = userEvent.setup()
      renderWithProviders(<InvoiceUserPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByTestId('invoice-user-table')).toBeInTheDocument()
      })

      const applyButton = screen.getByTestId('apply-invoice-button')
      await user.click(applyButton)

      await waitFor(() => {
        expect(screen.getByTestId('apply-invoice-dialog')).toBeInTheDocument()
      })

      // The dialog title "Apply for Invoice" appears alongside the button text,
      // so query by role to target the heading specifically.
      expect(screen.getByRole('heading', { name: 'Apply for Invoice' })).toBeInTheDocument()
    })

    it('submit without billingName shows validation error', async () => {
      const user = userEvent.setup()

      // Provide an apply endpoint so the mutation can succeed
      server.use(
        http.post(`${BASE_URL}/api/bill/${REALM_ID}/my/invoices`, () => {
          return HttpResponse.json({ id: 'inv-new' })
        })
      )

      renderWithProviders(<InvoiceUserPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByTestId('invoice-user-table')).toBeInTheDocument()
      })

      // Open dialog
      await user.click(screen.getByTestId('apply-invoice-button'))

      await waitFor(() => {
        expect(screen.getByTestId('apply-invoice-dialog')).toBeInTheDocument()
      })

      // Fill payment attempt ID so we pass the refine check
      const paymentInput = screen.getByTestId('apply-payment-attempt-id-input')
      await user.type(paymentInput, 'pay-123')

      // Submit with empty billingName
      const submitButton = screen.getByTestId('apply-invoice-submit-button')
      await user.click(submitButton)

      // Should show validation error for billingName
      await waitFor(() => {
        expect(screen.getByText('Billing name is required')).toBeInTheDocument()
      })
    })

    it('submit without payment/subscription shows validation error', async () => {
      const user = userEvent.setup()

      renderWithProviders(<InvoiceUserPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByTestId('invoice-user-table')).toBeInTheDocument()
      })

      // Open dialog
      await user.click(screen.getByTestId('apply-invoice-button'))

      await waitFor(() => {
        expect(screen.getByTestId('apply-invoice-dialog')).toBeInTheDocument()
      })

      // Fill only billingName (no payment/subscription)
      const billingNameInput = screen.getByTestId('apply-billing-name-input')
      await user.type(billingNameInput, 'Test Buyer')

      // Submit
      const submitButton = screen.getByTestId('apply-invoice-submit-button')
      await user.click(submitButton)

      // Should show validation error about payment attempt or subscription
      await waitFor(() => {
        expect(
          screen.getByText(/Either payment attempt or subscription is required/)
        ).toBeInTheDocument()
      })
    })

    it('valid submission calls apply mutation with correct payload', async () => {
      const user = userEvent.setup()
      let capturedBody: unknown = null

      server.use(
        http.post(`${BASE_URL}/api/bill/${REALM_ID}/my/invoices`, async ({ request }) => {
          capturedBody = await request.json()
          return HttpResponse.json({ id: 'inv-new' })
        })
      )

      renderWithProviders(<InvoiceUserPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByTestId('invoice-user-table')).toBeInTheDocument()
      })

      // Open dialog
      await user.click(screen.getByTestId('apply-invoice-button'))

      await waitFor(() => {
        expect(screen.getByTestId('apply-invoice-dialog')).toBeInTheDocument()
      })

      // Fill payment attempt ID
      const paymentInput = screen.getByTestId('apply-payment-attempt-id-input')
      await user.type(paymentInput, 'pay-abc-123')

      // Fill billing name
      const billingNameInput = screen.getByTestId('apply-billing-name-input')
      await user.type(billingNameInput, 'John Doe')

      // Fill billing email
      const billingEmailInput = screen.getByTestId('apply-billing-email-input')
      await user.type(billingEmailInput, 'john@example.com')

      // Fill billing address (required)
      const billingAddressInput = screen.getByTestId('apply-billing-address-input')
      await user.type(billingAddressInput, '123 Billing St')

      // Fill due date (required)
      const dueDateInput = screen.getByTestId('apply-due-date-input')
      await user.type(dueDateInput, '2025-08-01')

      // Submit
      const submitButton = screen.getByTestId('apply-invoice-submit-button')
      await user.click(submitButton)

      // Wait for mutation to be called
      await waitFor(() => {
        expect(capturedBody).not.toBeNull()
      })

      expect(capturedBody).toMatchObject({
        paymentAttemptId: 'pay-abc-123',
        billingName: 'John Doe',
        billingEmail: 'john@example.com',
        billingAddress: '123 Billing St',
        dueDate: '2025-08-01',
        currency: 'CNY',
      })
    })
  })

  // ==================== Pagination ====================

  describe('pagination', () => {
    const MANY_INVOICES: InvoiceResponse[] = Array.from({ length: 45 }, (_, i) =>
      makeInvoice({
        id: `inv-p${i}`,
        invoiceNumber: `INV-P${String(i + 1).padStart(3, '0')}`,
        status: 'issued',
      })
    )

    it('clicking next/prev controls change page', async () => {
      const user = userEvent.setup()
      let capturedPage: number | null = null

      server.use(
        http.get(`${BASE_URL}/api/bill/${REALM_ID}/my/invoices`, ({ request }) => {
          const url = new URL(request.url)
          capturedPage = parseInt(url.searchParams.get('page') ?? '0', 10)

          const start = capturedPage * 20
          const end = Math.min(start + 20, MANY_INVOICES.length)
          const pageInvoices = MANY_INVOICES.slice(start, end)

          return HttpResponse.json(
            makeListResponse(pageInvoices, {
              page: capturedPage,
              total: MANY_INVOICES.length,
            })
          )
        })
      )

      renderWithProviders(<InvoiceUserPage realmId={REALM_ID} />)

      // Wait for initial load (page 0)
      await waitFor(() => {
        expect(capturedPage).toBe(0)
      })

      await waitFor(() => {
        expect(screen.getByText('INV-P001')).toBeInTheDocument()
      })

      // Click next
      const nextButton = screen.getByTestId('invoice-user-pagination-next')
      await user.click(nextButton)

      await waitFor(() => {
        expect(capturedPage).toBe(1)
      })

      // Page 0 items should be gone, page 1 items should appear
      await waitFor(() => {
        expect(screen.getByText('INV-P021')).toBeInTheDocument()
      })

      // Click previous to go back to page 0
      const prevButton = screen.getByTestId('invoice-user-pagination-previous')
      await user.click(prevButton)

      // Verify we're back on page 0 by checking page 0 content appears
      await waitFor(() => {
        expect(screen.getByText('INV-P001')).toBeInTheDocument()
      })
    })
  })
})
