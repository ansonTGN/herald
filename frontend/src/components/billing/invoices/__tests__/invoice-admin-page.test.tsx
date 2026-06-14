/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor, within, fireEvent } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { InvoiceAdminPage } from '../invoice-admin-page'
import type { InvoiceResponse, InvoiceListResponse } from '@/lib/api-generated'
import { server } from '@/test/mocks/server'

// TanStack Router's <Link> requires a router context; in component tests we
// only care about its rendered href/children, so render a plain <a>. This
// mirrors the pattern used in profile-sidebar.test.tsx.
vi.mock('@tanstack/react-router', () => ({
  Link: ({
    to,
    search,
    children,
    ...props
  }: {
    to: string
    search?: Record<string, unknown>
    children?: React.ReactNode
  }) => {
    const query =
      search && Object.keys(search).length
        ? '?' + new URLSearchParams(search as Record<string, string>).toString()
        : ''
    return (
      <a href={`${to}${query}`} {...props}>
        {children}
      </a>
    )
  },
}))

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
    source: 'admin_manual',
    status: 'draft',
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

/**
 * Default feature-availability payload: an eligible realm (policy set, seller
 * configured) so the Create Invoice button stays enabled in happy-path tests.
 */
function makeFeatureAvailabilityEligible() {
  return {
    admin: {
      billingVisible: true,
      billingConfigVisible: true,
      entitlementMappingsVisible: true,
      invoicesVisible: true,
      subscriptionHistoryVisible: true,
      pointsVisible: true,
    },
    user: {
      pointsVisible: true,
      pointsPurchaseVisible: true,
      subscriptionVisible: true,
      invoicesVisible: true,
    },
    facts: {
      hasPaymentProviders: true,
      hasEntitlementMappings: true,
      hasEnabledMappings: true,
      hasOneTimeMappings: true,
      hasInvoiceSellerConfig: true,
      hasInvoices: true,
      hasSubscriptionHistory: true,
    },
    invoiceEligibility: {
      hasSellerConfig: true,
      policy: 'provider_first',
      canCreateManualInvoice: true,
      canApplyInvoice: true,
      reason: null,
    },
  }
}

describe('InvoiceAdminPage', () => {
  const defaultHandlers = [
    http.get(`${BASE_URL}/api/bill/${REALM_ID}/invoices`, ({ request }) => {
      const url = new URL(request.url)
      const status = url.searchParams.get('status')
      const source = url.searchParams.get('source')
      const page = parseInt(url.searchParams.get('page') ?? '0', 10)

      let invoices = [
        makeInvoice({
          id: 'inv-1',
          invoiceNumber: 'INV-001',
          status: 'draft',
          billingName: 'Buyer A',
          total: 9900,
          dueDate: '2025-06-01T00:00:00Z',
          createdAt: '2025-05-01T00:00:00Z',
        }),
        makeInvoice({
          id: 'inv-2',
          invoiceNumber: 'INV-002',
          status: 'issued',
          billingName: 'Buyer B',
          total: 15000,
          source: 'user_application',
          dueDate: '2025-06-15T00:00:00Z',
          createdAt: '2025-05-05T00:00:00Z',
        }),
        makeInvoice({
          id: 'inv-3',
          invoiceNumber: 'INV-003',
          status: 'paid',
          billingName: 'Buyer C',
          total: 20000,
          dueDate: '2025-05-01T00:00:00Z',
          createdAt: '2025-04-01T00:00:00Z',
        }),
        makeInvoice({
          id: 'inv-4',
          invoiceNumber: 'INV-004',
          status: 'overdue',
          billingName: 'Buyer D',
          total: 5000,
          dueDate: '2025-03-01T00:00:00Z',
          createdAt: '2025-02-01T00:00:00Z',
        }),
        makeInvoice({
          id: 'inv-5',
          invoiceNumber: 'INV-005',
          status: 'void',
          billingName: 'Buyer E',
          total: 3000,
          dueDate: '2025-01-15T00:00:00Z',
          createdAt: '2025-01-01T00:00:00Z',
        }),
      ]

      if (status) {
        invoices = invoices.filter((inv) => inv.status === status)
      }
      if (source) {
        invoices = invoices.filter((inv) => inv.source === source)
      }

      return HttpResponse.json(makeListResponse(invoices, { page, total: invoices.length }))
    }),
    http.get(`${BASE_URL}/api/realms/${REALM_ID}/feature-availability`, () => {
      return HttpResponse.json(makeFeatureAvailabilityEligible())
    }),
  ]

  beforeEach(() => {
    server.use(...defaultHandlers)
  })

  // ==================== Action Menu ====================

  describe('action menu per status', () => {
    const ALL_ACTIONS = ['View', 'Edit', 'Issue', 'Void', 'Mark Paid', 'Download PDF']

    async function openActionMenu(invoiceId: string) {
      const user = userEvent.setup()
      const trigger = screen.getByTestId(`invoice-actions-menu-${invoiceId}`)
      await user.click(trigger)
    }

    async function getVisibleActions() {
      // Wait for the dropdown menu content to appear
      const menuContent = await screen.findByRole('menu')
      const items = within(menuContent).getAllByRole('menuitem')
      return items.map((item) => item.textContent?.trim())
    }

    it('draft invoice: View, Edit, Issue, Void enabled; no Mark Paid or Download PDF', async () => {
      const invoice = makeInvoice({ id: 'inv-draft', status: 'draft' })

      server.use(
        http.get(`${BASE_URL}/api/bill/${REALM_ID}/invoices`, () => {
          return HttpResponse.json(makeListResponse([invoice]))
        })
      )

      renderWithProviders(<InvoiceAdminPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByText('INV-001')).toBeInTheDocument()
      })

      await openActionMenu('inv-draft')
      const actions = await getVisibleActions()

      expect(actions).toContain('View')
      expect(actions).toContain('Edit')
      expect(actions).toContain('Issue')
      expect(actions).toContain('Void')
      expect(actions).not.toContain('Mark Paid')
      expect(actions).not.toContain('Download PDF')
    })

    it('issued invoice: View, Void, Mark Paid, Download PDF enabled; no Edit', async () => {
      const invoice = makeInvoice({ id: 'inv-issued', status: 'issued' })

      server.use(
        http.get(`${BASE_URL}/api/bill/${REALM_ID}/invoices`, () => {
          return HttpResponse.json(makeListResponse([invoice]))
        })
      )

      renderWithProviders(<InvoiceAdminPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByText('INV-001')).toBeInTheDocument()
      })

      await openActionMenu('inv-issued')
      const actions = await getVisibleActions()

      expect(actions).toContain('View')
      expect(actions).not.toContain('Edit')
      expect(actions).not.toContain('Issue')
      expect(actions).toContain('Void')
      expect(actions).toContain('Mark Paid')
      expect(actions).toContain('Download PDF')
    })

    it('paid invoice: View, Download PDF enabled; others disabled', async () => {
      const invoice = makeInvoice({ id: 'inv-paid', status: 'paid' })

      server.use(
        http.get(`${BASE_URL}/api/bill/${REALM_ID}/invoices`, () => {
          return HttpResponse.json(makeListResponse([invoice]))
        })
      )

      renderWithProviders(<InvoiceAdminPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByText('INV-001')).toBeInTheDocument()
      })

      await openActionMenu('inv-paid')
      const actions = await getVisibleActions()

      expect(actions).toContain('View')
      expect(actions).not.toContain('Edit')
      expect(actions).not.toContain('Issue')
      expect(actions).not.toContain('Void')
      expect(actions).not.toContain('Mark Paid')
      expect(actions).toContain('Download PDF')
    })

    it('overdue invoice: View, Void, Mark Paid, Download PDF enabled; no Edit', async () => {
      const invoice = makeInvoice({ id: 'inv-overdue', status: 'overdue' })

      server.use(
        http.get(`${BASE_URL}/api/bill/${REALM_ID}/invoices`, () => {
          return HttpResponse.json(makeListResponse([invoice]))
        })
      )

      renderWithProviders(<InvoiceAdminPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByText('INV-001')).toBeInTheDocument()
      })

      await openActionMenu('inv-overdue')
      const actions = await getVisibleActions()

      expect(actions).toContain('View')
      expect(actions).not.toContain('Edit')
      expect(actions).not.toContain('Issue')
      expect(actions).toContain('Void')
      expect(actions).toContain('Mark Paid')
      expect(actions).toContain('Download PDF')
    })

    it('void invoice: only View enabled', async () => {
      const invoice = makeInvoice({ id: 'inv-void', status: 'void' })

      server.use(
        http.get(`${BASE_URL}/api/bill/${REALM_ID}/invoices`, () => {
          return HttpResponse.json(makeListResponse([invoice]))
        })
      )

      renderWithProviders(<InvoiceAdminPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByText('INV-001')).toBeInTheDocument()
      })

      await openActionMenu('inv-void')
      const actions = await getVisibleActions()

      expect(actions).toContain('View')
      expect(actions).toHaveLength(1)
    })
  })

  // ==================== Pagination ====================

  describe('pagination', () => {
    const MANY_INVOICES: InvoiceResponse[] = Array.from({ length: 45 }, (_, i) =>
      makeInvoice({
        id: `inv-p${i}`,
        invoiceNumber: `INV-P${String(i + 1).padStart(3, '0')}`,
        status: 'draft',
        billingName: `Buyer ${i + 1}`,
      })
    )

    it('clicking next page triggers re-fetch with updated page param', async () => {
      let capturedPage: number | null = null

      server.use(
        http.get(`${BASE_URL}/api/bill/${REALM_ID}/invoices`, ({ request }) => {
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

      renderWithProviders(<InvoiceAdminPage realmId={REALM_ID} />)

      // Wait for initial load (page 0)
      await waitFor(() => {
        expect(capturedPage).toBe(0)
      })

      // Verify page 0 data
      await waitFor(() => {
        expect(screen.getByText('INV-P001')).toBeInTheDocument()
      })

      // Click next — PaginationNext renders as <a> without href;
      // fireEvent.click is more reliable than userEvent for this case.
      const nextButton = screen.getByTestId('invoice-pagination-next')
      fireEvent.click(nextButton)

      await waitFor(
        () => {
          expect(capturedPage).toBe(1)
        },
        { timeout: 5000 }
      )

      // Page 0 items should be gone, page 1 items should appear
      await waitFor(() => {
        expect(screen.getByText('INV-P021')).toBeInTheDocument()
      })
    })
  })

  // ==================== Callbacks ====================

  describe('action callbacks', () => {
    it('calls onIssueInvoice when Issue is clicked in draft action menu', async () => {
      const user = userEvent.setup()
      const onIssueInvoice = vi.fn()
      const invoice = makeInvoice({ id: 'inv-cb', status: 'draft' })

      server.use(
        http.get(`${BASE_URL}/api/bill/${REALM_ID}/invoices`, () => {
          return HttpResponse.json(makeListResponse([invoice]))
        })
      )

      renderWithProviders(<InvoiceAdminPage realmId={REALM_ID} onIssueInvoice={onIssueInvoice} />)

      await waitFor(() => {
        expect(screen.getByText('INV-001')).toBeInTheDocument()
      })

      // Open action menu
      await user.click(screen.getByTestId('invoice-actions-menu-inv-cb'))

      // Click Issue
      const issueItem = await screen.findByTestId('invoice-issue-inv-cb')
      await user.click(issueItem)

      expect(onIssueInvoice).toHaveBeenCalledWith(
        expect.objectContaining({ id: 'inv-cb', status: 'draft' })
      )
    })
  })

  // ==================== Create Invoice Eligibility Gating ====================

  describe('Create Invoice eligibility gating', () => {
    function overrideFeatureAvailability(overrides: Record<string, unknown>) {
      server.use(
        http.get(`${BASE_URL}/api/realms/${REALM_ID}/feature-availability`, () => {
          return HttpResponse.json({
            ...makeFeatureAvailabilityEligible(),
            invoiceEligibility: {
              ...makeFeatureAvailabilityEligible().invoiceEligibility,
              ...overrides,
            },
          })
        })
      )
    }

    it('keeps Create Invoice enabled when realm is eligible', async () => {
      const onCreateInvoice = vi.fn()
      renderWithProviders(<InvoiceAdminPage realmId={REALM_ID} onCreateInvoice={onCreateInvoice} />)

      await waitFor(() => {
        expect(screen.getByTestId('create-invoice-button')).toBeEnabled()
      })

      expect(screen.queryByTestId('create-invoice-disabled-reason')).not.toBeInTheDocument()
    })

    it('disables Create Invoice and shows no-policy reason when policy === none', async () => {
      overrideFeatureAvailability({
        policy: 'none',
        canCreateManualInvoice: false,
        canApplyInvoice: false,
      })

      const onCreateInvoice = vi.fn()
      renderWithProviders(<InvoiceAdminPage realmId={REALM_ID} onCreateInvoice={onCreateInvoice} />)

      // The reason span only renders once eligibility resolves and disables the button;
      // waiting on it avoids racing the feature-availability query.
      const reason = await screen.findByTestId('create-invoice-disabled-reason')
      expect(reason).toHaveTextContent('This realm does not issue Herald invoices.')

      expect(screen.getByTestId('create-invoice-button')).toBeDisabled()
      // No configure link in the no-policy branch
      expect(screen.queryByTestId('create-invoice-configure-link')).not.toBeInTheDocument()
    })

    it('disables Create Invoice and shows seller-config reason + Configure link when seller missing', async () => {
      overrideFeatureAvailability({ hasSellerConfig: false })

      const onCreateInvoice = vi.fn()
      renderWithProviders(<InvoiceAdminPage realmId={REALM_ID} onCreateInvoice={onCreateInvoice} />)

      const reason = await screen.findByTestId('create-invoice-disabled-reason')
      expect(reason).toHaveTextContent('Configure seller information first.')

      expect(screen.getByTestId('create-invoice-button')).toBeDisabled()

      const link = screen.getByTestId('create-invoice-configure-link')
      expect(link).toHaveTextContent('Configure')
      // Deep-link to the invoices route with the seller-config open param.
      expect(link).toHaveAttribute('href', '/test-realm/manage/billing/invoices?open=seller')
    })
  })
})
