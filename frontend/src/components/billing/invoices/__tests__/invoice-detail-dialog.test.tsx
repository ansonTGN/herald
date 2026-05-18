/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor, within } from '@testing-library/react'
import { http, HttpResponse } from 'msw'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { InvoiceDetailDialog } from '../invoice-detail-dialog'
import type { InvoiceDetailResponse } from '@/lib/api-generated'
import { server } from '@/test/mocks/server'

// ==================== Mocks ====================

// Button uses Radix Slot when asChild=true. Slot.Children.only(null) throws
// in dev mode when Button renders {isLoading && <Spinner />}{children} because
// the boolean `false` counts as a child alongside the <a> element.
// Mock Button to avoid the Slot crash while keeping functional behavior.
vi.mock('@/components/ui/button', () => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const React = require('react')

  function Button(props: any) {
    const {
      children,
      asChild,
      className,
      variant,
      size,
      loading,
      disabled,
      'data-testid': dataTestId,
      ...rest
    } = props

    if (asChild && React.isValidElement(children)) {
      return React.cloneElement(children, {
        className: [className, children.props.className].filter(Boolean).join(' '),
        'data-testid': dataTestId ?? children.props['data-testid'],
        ...rest,
      })
    }

    return React.createElement(
      'button',
      {
        className,
        'data-testid': dataTestId,
        disabled: disabled || loading,
        ...rest,
      },
      loading && React.createElement('span', null, 'Loading...'),
      children
    )
  }

  return { Button, buttonVariants: () => '' }
})

// ==================== Test Helpers ====================

const REALM_ID = 'test-realm'
const INVOICE_ID = 'inv-1'
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

function makeInvoiceDetail(overrides: Partial<InvoiceDetailResponse> = {}): InvoiceDetailResponse {
  return {
    id: INVOICE_ID,
    invoiceNumber: 'INV-001',
    accountId: 'acc-1',
    applicantUserId: null,
    billingName: 'Test Buyer',
    billingEmail: 'buyer@test.com',
    billingAddress: '123 Buyer St',
    billingPhone: '111-222-3333',
    billingTaxId: 'TAX123',
    sellerName: 'Test Seller',
    sellerEmail: 'seller@test.com',
    sellerAddress: '456 Seller Ave',
    sellerPhone: '444-555-6666',
    sellerTaxId: 'TAX456',
    currency: 'CNY',
    source: 'admin_manual',
    status: 'draft',
    subtotal: 20000,
    discountAmount: 1000,
    discountMode: null,
    discountValue: null,
    taxAmount: 1500,
    taxMode: null,
    taxValue: null,
    shippingAmount: 500,
    shippingMode: null,
    shippingValue: null,
    total: 21000,
    dueDate: '2025-07-01T00:00:00Z',
    paymentTerms: 'Net 30',
    notes: 'Test notes',
    issueDate: '2025-06-01T00:00:00Z',
    issuedAt: null,
    createdAt: '2025-05-01T00:00:00Z',
    updatedAt: '2025-06-01T00:00:00Z',
    lineItems: [
      {
        id: 'li-1',
        invoiceId: INVOICE_ID,
        name: 'Service A',
        description: 'Service A description',
        quantity: '2',
        unitPrice: 5000,
        subtotal: 10000,
        sortOrder: 0,
      },
      {
        id: 'li-2',
        invoiceId: INVOICE_ID,
        name: 'Service B',
        description: null,
        quantity: '1',
        unitPrice: 10000,
        subtotal: 10000,
        sortOrder: 1,
      },
    ],
    history: [
      {
        id: 'hist-1',
        invoiceId: INVOICE_ID,
        eventType: 'created',
        actorType: 'user',
        actorUserId: 'user-1',
        changes: null,
        createdAt: '2025-05-01T10:00:00Z',
      },
      {
        id: 'hist-2',
        invoiceId: INVOICE_ID,
        eventType: 'updated',
        actorType: 'user',
        actorUserId: 'user-1',
        changes: { field: 'status', from: 'draft', to: 'review' },
        createdAt: '2025-05-15T14:00:00Z',
      },
      {
        id: 'hist-3',
        invoiceId: INVOICE_ID,
        eventType: 'issued',
        actorType: 'system',
        actorUserId: null,
        changes: null,
        createdAt: '2025-06-01T12:00:00Z',
      },
    ],
    subscriptionId: null,
    paymentAttemptId: null,
    paidAt: null,
    voidReason: null,
    voidedAt: null,
    realmId: REALM_ID,
    ...overrides,
  }
}

const defaultOnOpenChange = vi.fn()

function setupDetailDialog(
  invoiceOverrides: Partial<InvoiceDetailResponse> = {},
  props: { open?: boolean; invoiceId?: string | null } = {}
) {
  const invoice = makeInvoiceDetail(invoiceOverrides)

  server.use(
    http.get(`${BASE_URL}/api/bill/${REALM_ID}/invoices/${invoice.id}`, () => {
      return HttpResponse.json(invoice)
    })
  )

  return renderWithProviders(
    <InvoiceDetailDialog
      open={props.open ?? true}
      onOpenChange={defaultOnOpenChange}
      realmId={REALM_ID}
      invoiceId={props.invoiceId ?? invoice.id}
    />
  )
}

// ==================== Tests ====================

describe('InvoiceDetailDialog', () => {
  beforeEach(() => {
    defaultOnOpenChange.mockClear()
  })

  // ==================== Status History ====================

  describe('status history', () => {
    it('renders chronological events with timestamp and event type', async () => {
      setupDetailDialog()

      await waitFor(() => {
        expect(screen.getByTestId('invoice-status-history')).toBeInTheDocument()
      })

      expect(screen.getByText('Status History')).toBeInTheDocument()

      // Events sorted oldest-first: created, updated, issued
      expect(screen.getByText('Created')).toBeInTheDocument()
      expect(screen.getByText('Updated')).toBeInTheDocument()
      expect(screen.getByText('Issued')).toBeInTheDocument()

      // Actor labels: "User" for created and updated, "System" for issued
      const userLabels = screen.getAllByText('User')
      expect(userLabels).toHaveLength(2)
      expect(screen.getByText('System')).toBeInTheDocument()
    })

    it('renders history with change description when changes have field/from/to', async () => {
      setupDetailDialog({
        history: [
          {
            id: 'hist-change',
            invoiceId: INVOICE_ID,
            eventType: 'updated',
            actorType: 'user',
            actorUserId: 'user-1',
            changes: { field: 'status', from: 'draft', to: 'issued' },
            createdAt: '2025-06-01T11:00:00Z',
          },
        ],
      })

      await waitFor(() => {
        expect(screen.getByTestId('invoice-status-history')).toBeInTheDocument()
      })

      expect(screen.getByText('Updated')).toBeInTheDocument()
      expect(screen.getByText('status: draft -> issued')).toBeInTheDocument()
    })

    it('renders "No history events" when history is empty', async () => {
      setupDetailDialog({ history: [] })

      await waitFor(() => {
        expect(screen.getByTestId('invoice-status-history')).toBeInTheDocument()
      })

      expect(screen.getByText('No history events')).toBeInTheDocument()
    })
  })

  // ==================== PDF Download Button Visibility ====================

  describe('PDF download button per status', () => {
    it('visible for issued status', async () => {
      setupDetailDialog({ status: 'issued' })

      await waitFor(() => {
        expect(screen.getByTestId('invoice-download-pdf-button')).toBeInTheDocument()
      })

      expect(screen.getByText('Download PDF')).toBeInTheDocument()
    })

    it('visible for paid status', async () => {
      setupDetailDialog({ status: 'paid' })

      await waitFor(() => {
        expect(screen.getByTestId('invoice-download-pdf-button')).toBeInTheDocument()
      })
    })

    it('visible for overdue status', async () => {
      setupDetailDialog({ status: 'overdue' })

      await waitFor(() => {
        expect(screen.getByTestId('invoice-download-pdf-button')).toBeInTheDocument()
      })
    })

    it('NOT visible for draft status', async () => {
      setupDetailDialog({ status: 'draft' })

      await waitFor(() => {
        expect(screen.getByTestId('invoice-detail-dialog')).toBeInTheDocument()
      })

      expect(screen.queryByTestId('invoice-download-pdf-button')).not.toBeInTheDocument()
    })

    it('NOT visible for void status', async () => {
      setupDetailDialog({ status: 'void' })

      await waitFor(() => {
        expect(screen.getByTestId('invoice-detail-dialog')).toBeInTheDocument()
      })

      expect(screen.queryByTestId('invoice-download-pdf-button')).not.toBeInTheDocument()
    })

    it('PDF link href contains correct path', async () => {
      setupDetailDialog({ status: 'issued' })

      const pdfButton = await screen.findByTestId('invoice-download-pdf-button')
      const link = pdfButton.closest('a')
      expect(link?.getAttribute('href')).toBe(`/api/bill/${REALM_ID}/invoices/${INVOICE_ID}/pdf`)
    })
  })

  // ==================== Additional Info ====================

  describe('additional info section', () => {
    it('renders issue date, due date, payment terms, and notes', async () => {
      setupDetailDialog()

      await waitFor(() => {
        expect(screen.getByTestId('invoice-additional-info')).toBeInTheDocument()
      })

      const section = within(screen.getByTestId('invoice-additional-info'))

      // Labels
      expect(section.getByText('Issue Date')).toBeInTheDocument()
      expect(section.getByText('Due Date')).toBeInTheDocument()
      expect(section.getByText('Payment Terms')).toBeInTheDocument()
      expect(section.getByText('Additional Information')).toBeInTheDocument()

      // Values
      expect(section.getByText('Net 30')).toBeInTheDocument()
      expect(section.getByText('Test notes')).toBeInTheDocument()
    })
  })

  // ==================== Loading State ====================

  describe('loading state', () => {
    it('shows skeleton placeholders while fetching detail', async () => {
      server.use(
        http.get(`${BASE_URL}/api/bill/${REALM_ID}/invoices/${INVOICE_ID}`, async () => {
          await new Promise((resolve) => setTimeout(resolve, 500))
          return HttpResponse.json(makeInvoiceDetail())
        })
      )

      renderWithProviders(
        <InvoiceDetailDialog
          open
          onOpenChange={defaultOnOpenChange}
          realmId={REALM_ID}
          invoiceId={INVOICE_ID}
        />
      )

      // Dialog should be present while loading
      expect(screen.getByTestId('invoice-detail-dialog')).toBeInTheDocument()

      // Skeleton placeholders in header
      const skeletons = document.querySelectorAll('[data-slot="skeleton"]')
      expect(skeletons.length).toBeGreaterThan(0)

      // Wait for loading to complete (seller info only appears after data loads)
      await waitFor(() => {
        expect(screen.getByTestId('invoice-seller-info')).toBeInTheDocument()
      })
    })
  })

  // ==================== Not Found / Error State ====================

  describe('invoice not found', () => {
    it('shows "Invoice Not Found" when API returns 404', async () => {
      server.use(
        http.get(`${BASE_URL}/api/bill/${REALM_ID}/invoices/${INVOICE_ID}`, () => {
          return HttpResponse.json({ message: 'Invoice not found' }, { status: 404 })
        })
      )

      renderWithProviders(
        <InvoiceDetailDialog
          open
          onOpenChange={defaultOnOpenChange}
          realmId={REALM_ID}
          invoiceId={INVOICE_ID}
        />
      )

      // Wait for the error state to render (query retries once, then fails)
      await waitFor(
        () => {
          expect(screen.getByText('Invoice Not Found')).toBeInTheDocument()
        },
        { timeout: 5000 }
      )

      expect(screen.getByText('The requested invoice could not be loaded.')).toBeInTheDocument()
    })

    it('shows "Invoice Not Found" when invoiceId is null', () => {
      renderWithProviders(
        <InvoiceDetailDialog
          open
          onOpenChange={defaultOnOpenChange}
          realmId={REALM_ID}
          invoiceId={null}
        />
      )

      // When invoiceId is null, the query is disabled (enabled: open && !!invoiceId).
      // isLoading stays false and invoice is undefined -> falls into the "not found" branch.
      expect(screen.getByText('Invoice Not Found')).toBeInTheDocument()
    })
  })
})
