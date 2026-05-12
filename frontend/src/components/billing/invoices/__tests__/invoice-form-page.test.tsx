/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { InvoiceFormPage } from '../invoice-form-page'
import type { InvoiceDetailResponse } from '@/lib/api-generated'
import { server } from '@/test/mocks/server'

// ==================== Router Mock ====================

const mockNavigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
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

function makeInvoiceDetail(overrides: Partial<InvoiceDetailResponse> = {}): InvoiceDetailResponse {
  return {
    id: 'inv-1',
    invoiceNumber: 'INV-001',
    accountId: 'acc-1',
    applicantUserId: null,
    billingName: 'Test Buyer',
    billingEmail: 'buyer@test.com',
    billingAddress: '123 Buyer St',
    billingPhone: '111-222-3333',
    sellerName: 'Test Seller',
    sellerEmail: 'seller@test.com',
    sellerAddress: '456 Seller Ave',
    sellerPhone: '444-555-6666',
    currency: 'CNY',
    source: 'admin_manual',
    status: 'draft',
    subtotal: 10000,
    discountAmount: 0,
    discountMode: null,
    discountValue: null,
    taxAmount: 0,
    taxMode: null,
    taxValue: null,
    shippingAmount: 0,
    shippingMode: null,
    shippingValue: null,
    total: 10000,
    dueDate: '2025-07-01T00:00:00Z',
    paymentTerms: 'Net 30',
    notes: 'Test notes',
    createdAt: '2025-05-01T00:00:00Z',
    updatedAt: '2025-05-01T00:00:00Z',
    lineItems: [
      {
        id: 'li-1',
        invoiceId: 'inv-1',
        name: 'Service A',
        description: 'Service description',
        quantity: '2',
        unitPrice: 5000,
        subtotal: 10000,
        sortOrder: 0,
      },
    ],
    history: [],
    subscriptionId: null,
    paymentAttemptId: null,
    issueDate: null,
    issuedAt: null,
    paidAt: null,
    voidReason: null,
    voidedAt: null,
    realmId: REALM_ID,
    ...overrides,
  }
}

// Seller config handler
function sellerConfigHandler(
  config: {
    sellerName: string
    sellerEmail?: string
    sellerAddress?: string
    sellerPhone?: string
  } | null = {
    sellerName: 'Default Seller Corp',
    sellerEmail: 'seller@default.com',
    sellerAddress: '789 Default Blvd',
    sellerPhone: '999-888-7777',
  }
) {
  return http.get(`${BASE_URL}/api/bill/${REALM_ID}/invoice-seller-config`, () => {
    if (config === null) {
      return HttpResponse.json({ message: 'Not found' }, { status: 404 })
    }
    return HttpResponse.json({
      sellerName: config.sellerName,
      sellerEmail: config.sellerEmail ?? null,
      sellerAddress: config.sellerAddress ?? null,
      sellerPhone: config.sellerPhone ?? null,
      createdAt: '2025-01-01T00:00:00Z',
      updatedAt: '2025-01-01T00:00:00Z',
    })
  })
}

// Mutation handlers
function createInvoiceHandler() {
  return http.post(`${BASE_URL}/api/bill/${REALM_ID}/invoices`, async () => {
    return HttpResponse.json(makeInvoiceDetail(), { status: 201 })
  })
}

function updateInvoiceHandler(invoiceId: string = 'inv-1') {
  return http.patch(`${BASE_URL}/api/bill/${REALM_ID}/invoices/${invoiceId}`, async () => {
    return HttpResponse.json(makeInvoiceDetail())
  })
}

// Default props for create mode
function defaultCreateProps() {
  return {
    mode: 'create' as const,
    realmId: REALM_ID,
  }
}

// Default props for edit mode
function defaultEditProps(invoice?: InvoiceDetailResponse) {
  return {
    mode: 'edit' as const,
    realmId: REALM_ID,
    invoice: invoice ?? makeInvoiceDetail(),
  }
}

// ==================== Tests ====================

describe('InvoiceFormPage', () => {
  beforeEach(() => {
    server.use(sellerConfigHandler(), createInvoiceHandler(), updateInvoiceHandler())
  })

  // ==================== Create Mode Rendering ====================

  describe('create mode', () => {
    it('renders "Create Invoice" title and empty form', async () => {
      renderWithProviders(<InvoiceFormPage {...defaultCreateProps()} />)

      await waitFor(() => {
        expect(screen.getByText('Create Invoice')).toBeInTheDocument()
      })
      expect(screen.getByText('Create a new invoice draft')).toBeInTheDocument()

      // Verify empty form fields exist
      expect(screen.getByTestId('invoice-account-id')).toHaveValue('')
      expect(screen.getByTestId('invoice-billing-name')).toHaveValue('')
      expect(screen.getByTestId('invoice-due-date')).toHaveValue('')
      expect(screen.getByTestId('invoice-form-submit-button')).toHaveTextContent('Save as Draft')
    })

    it('renders account selector field in create mode', async () => {
      renderWithProviders(<InvoiceFormPage {...defaultCreateProps()} />)

      await waitFor(() => {
        expect(screen.getByText('Create Invoice')).toBeInTheDocument()
      })

      expect(screen.getByTestId('invoice-account-id')).toBeInTheDocument()
      expect(screen.getByTestId('invoice-subscription-id')).toBeInTheDocument()
      expect(screen.getByTestId('invoice-payment-attempt-id')).toBeInTheDocument()
    })
  })

  // ==================== Edit Mode Rendering ====================

  describe('edit mode', () => {
    it('renders "Edit Invoice" title and populates fields from invoice data', async () => {
      const invoice = makeInvoiceDetail()
      renderWithProviders(<InvoiceFormPage {...defaultEditProps(invoice)} />)

      await waitFor(() => {
        expect(screen.getByText('Edit Invoice')).toBeInTheDocument()
      })
      expect(screen.getByText('Update invoice draft details')).toBeInTheDocument()

      // Verify fields are populated from invoice data
      expect(screen.getByTestId('invoice-billing-name')).toHaveValue('Test Buyer')
      expect(screen.getByTestId('invoice-billing-email')).toHaveValue('buyer@test.com')
      expect(screen.getByTestId('invoice-billing-address')).toHaveValue('123 Buyer St')
      expect(screen.getByTestId('invoice-seller-name')).toHaveValue('Test Seller')

      // Line item populated
      expect(screen.getByTestId('invoice-line-item-name-0')).toHaveValue('Service A')
      expect(screen.getByTestId('invoice-line-item-quantity-0')).toHaveValue('2')
    })

    it('does not render account selector in edit mode', async () => {
      renderWithProviders(<InvoiceFormPage {...defaultEditProps()} />)

      await waitFor(() => {
        expect(screen.getByText('Edit Invoice')).toBeInTheDocument()
      })

      expect(screen.queryByTestId('invoice-account-id')).not.toBeInTheDocument()
      expect(screen.queryByTestId('invoice-subscription-id')).not.toBeInTheDocument()
    })
  })

  // ==================== Line Item Add/Remove ====================

  describe('line item management', () => {
    it('adds a new line item row when clicking "+ Add Line Item"', async () => {
      const user = userEvent.setup()
      renderWithProviders(<InvoiceFormPage {...defaultCreateProps()} />)

      await waitFor(() => {
        expect(screen.getByText('Create Invoice')).toBeInTheDocument()
      })

      // Default: 1 line item
      expect(screen.getByTestId('invoice-line-item-name-0')).toBeInTheDocument()
      expect(screen.queryByTestId('invoice-line-item-name-1')).not.toBeInTheDocument()

      // Add line item
      await user.click(screen.getByTestId('invoice-add-line-item'))

      // Now there should be 2 line items
      expect(screen.getByTestId('invoice-line-item-name-0')).toBeInTheDocument()
      expect(screen.getByTestId('invoice-line-item-name-1')).toBeInTheDocument()
      expect(screen.getByTestId('invoice-line-item-quantity-1')).toHaveValue('1')
    })

    it('removes a line item row when clicking remove button', async () => {
      const user = userEvent.setup()
      renderWithProviders(<InvoiceFormPage {...defaultCreateProps()} />)

      await waitFor(() => {
        expect(screen.getByText('Create Invoice')).toBeInTheDocument()
      })

      // Add a second line item
      await user.click(screen.getByTestId('invoice-add-line-item'))
      expect(screen.getByTestId('invoice-line-item-name-1')).toBeInTheDocument()

      // Remove the first line item (index 0)
      await user.click(screen.getByTestId('invoice-line-item-remove-0'))

      // Only one line item should remain (the second one shifted to index 0)
      expect(screen.queryByTestId('invoice-line-item-name-1')).not.toBeInTheDocument()
    })

    it('does not allow removing the last remaining line item', async () => {
      const user = userEvent.setup()
      renderWithProviders(<InvoiceFormPage {...defaultCreateProps()} />)

      await waitFor(() => {
        expect(screen.getByText('Create Invoice')).toBeInTheDocument()
      })

      // Only one line item, remove button should be disabled
      const removeButton = screen.getByTestId('invoice-line-item-remove-0')
      expect(removeButton).toBeDisabled()
    })
  })

  // ==================== Real-time Amount Calculations ====================

  describe('amount calculations', () => {
    it('shows correct subtotal: qty=2, unitPrice=50 => subtotal ¥100.00', async () => {
      const user = userEvent.setup()
      renderWithProviders(<InvoiceFormPage {...defaultCreateProps()} />)

      await waitFor(() => {
        expect(screen.getByText('Create Invoice')).toBeInTheDocument()
      })

      // Wait for seller config to load (it triggers a form reset)
      await waitFor(() => {
        expect(screen.getByTestId('invoice-seller-name')).toHaveValue('Default Seller Corp')
      })

      // Fill quantity and unit price (decimal input)
      const qtyInput = screen.getByTestId('invoice-line-item-quantity-0')
      const priceInput = screen.getByTestId('invoice-line-item-unit-price-0')

      await user.clear(qtyInput)
      await user.type(qtyInput, '2')
      await user.clear(priceInput)
      await user.type(priceInput, '50')

      // Wait for subtotal to update in the totals preview (formatted as currency)
      await waitFor(() => {
        expect(screen.getByTestId('invoice-totals-subtotal')).toHaveTextContent('¥100.00')
      })
    })

    it('calculates discount: edit mode invoice with discountMode=percent, discountValue=10 => discount shown', async () => {
      const invoice = makeInvoiceDetail({
        subtotal: 9900,
        discountMode: 'percent',
        discountValue: '10',
        discountAmount: 990,
        taxMode: null,
        taxValue: null,
        taxAmount: 0,
        shippingMode: null,
        shippingValue: null,
        shippingAmount: 0,
        total: 8910,
        lineItems: [
          {
            id: 'li-1',
            invoiceId: 'inv-1',
            name: 'Service A',
            description: null,
            quantity: '1',
            unitPrice: 9900,
            subtotal: 9900,
            sortOrder: 0,
          },
        ],
      })
      renderWithProviders(<InvoiceFormPage {...defaultEditProps(invoice)} />)

      await waitFor(() => {
        expect(screen.getByText('Edit Invoice')).toBeInTheDocument()
      })

      // Subtotal from line item: 1 * 99.00 = ¥99.00 (9900 cents)
      expect(screen.getByTestId('invoice-totals-subtotal')).toHaveTextContent('¥99.00')

      // Discount: 9900 * 10% = 990 cents = ¥9.90
      await waitFor(() => {
        expect(screen.getByTestId('invoice-totals-discount')).toHaveTextContent('-¥9.90')
      })

      // Discount value input should be enabled and have value 10
      expect(screen.getByTestId('invoice-discount-value')).not.toBeDisabled()
    })

    it('calculates tax: edit mode invoice with taxMode=percent, taxValue=6 => tax shown', async () => {
      const invoice = makeInvoiceDetail({
        subtotal: 9900,
        discountMode: null,
        discountValue: null,
        discountAmount: 0,
        taxMode: 'percent',
        taxValue: '6',
        taxAmount: 594,
        shippingMode: null,
        shippingValue: null,
        shippingAmount: 0,
        total: 10494,
        lineItems: [
          {
            id: 'li-1',
            invoiceId: 'inv-1',
            name: 'Service A',
            description: null,
            quantity: '1',
            unitPrice: 9900,
            subtotal: 9900,
            sortOrder: 0,
          },
        ],
      })
      renderWithProviders(<InvoiceFormPage {...defaultEditProps(invoice)} />)

      await waitFor(() => {
        expect(screen.getByText('Edit Invoice')).toBeInTheDocument()
      })

      // Subtotal from line item: 1 * 99.00 = ¥99.00 (9900 cents)
      expect(screen.getByTestId('invoice-totals-subtotal')).toHaveTextContent('¥99.00')

      // Tax: 9900 * 6% = 594 cents = ¥5.94
      await waitFor(() => {
        expect(screen.getByTestId('invoice-totals-tax')).toHaveTextContent('+¥5.94')
      })
    })

    it('calculates total: subtotal - discount + tax + shipping = total displayed correctly', async () => {
      const invoice = makeInvoiceDetail({
        subtotal: 10000,
        discountMode: 'percent',
        discountValue: '10',
        discountAmount: 1000,
        taxMode: 'percent',
        taxValue: '5',
        taxAmount: 450,
        shippingMode: 'fixed',
        shippingValue: '3',
        shippingAmount: 300,
        total: 9750,
        lineItems: [
          {
            id: 'li-1',
            invoiceId: 'inv-1',
            name: 'Service A',
            description: null,
            quantity: '1',
            unitPrice: 10000,
            subtotal: 10000,
            sortOrder: 0,
          },
        ],
      })
      renderWithProviders(<InvoiceFormPage {...defaultEditProps(invoice)} />)

      await waitFor(() => {
        expect(screen.getByText('Edit Invoice')).toBeInTheDocument()
      })

      // Verify totals preview displays correctly (formatted as currency)
      await waitFor(() => {
        expect(screen.getByTestId('invoice-totals-subtotal')).toHaveTextContent('¥100.00')
      })
      expect(screen.getByTestId('invoice-totals-discount')).toHaveTextContent('-¥10.00')
      expect(screen.getByTestId('invoice-totals-tax')).toHaveTextContent('+¥4.50')
      expect(screen.getByTestId('invoice-totals-shipping')).toHaveTextContent('+¥3.00')
      expect(screen.getByTestId('invoice-totals-total')).toHaveTextContent('¥97.50')
    })
  })

  // ==================== Validation ====================

  describe('validation', () => {
    it('shows error when submitting with empty billingName', async () => {
      const user = userEvent.setup()
      renderWithProviders(<InvoiceFormPage {...defaultCreateProps()} />)

      await waitFor(() => {
        expect(screen.getByText('Create Invoice')).toBeInTheDocument()
      })

      // Fill accountId (required for create mode) but leave billingName empty
      await user.type(screen.getByTestId('invoice-account-id'), 'acc-1')

      // Fill due date (required)
      await user.type(screen.getByTestId('invoice-due-date'), '2025-07-01')

      // Fill line item name (required)
      await user.type(screen.getByTestId('invoice-line-item-name-0'), 'Item A')
      await user.clear(screen.getByTestId('invoice-line-item-quantity-0'))
      await user.type(screen.getByTestId('invoice-line-item-quantity-0'), '1')
      await user.clear(screen.getByTestId('invoice-line-item-unit-price-0'))
      await user.type(screen.getByTestId('invoice-line-item-unit-price-0'), '100')

      // Make billingName empty and blur to trigger validation
      const billingNameInput = screen.getByTestId('invoice-billing-name')
      await user.clear(billingNameInput)
      // Blur to trigger field validation display
      billingNameInput.blur()

      // Submit the form
      await user.click(screen.getByTestId('invoice-form-submit-button'))

      await waitFor(() => {
        // Should show validation error for billing name
        const errorElements = screen.getAllByText(/billing name is required/i)
        expect(errorElements.length).toBeGreaterThan(0)
      })
    })

    it('shows error when submitting with no line items (empty item name)', async () => {
      const user = userEvent.setup()
      renderWithProviders(<InvoiceFormPage {...defaultCreateProps()} />)

      await waitFor(() => {
        expect(screen.getByText('Create Invoice')).toBeInTheDocument()
      })

      // Fill required fields except line item name
      await user.type(screen.getByTestId('invoice-account-id'), 'acc-1')
      await user.type(screen.getByTestId('invoice-billing-name'), 'Buyer Name')
      await user.type(screen.getByTestId('invoice-due-date'), '2025-07-01')

      // Line item name is empty (default), blur it
      const lineItemNameInput = screen.getByTestId('invoice-line-item-name-0')
      lineItemNameInput.blur()

      // Submit
      await user.click(screen.getByTestId('invoice-form-submit-button'))

      await waitFor(() => {
        const errorElements = screen.getAllByText(/item name is required/i)
        expect(errorElements.length).toBeGreaterThan(0)
      })
    })
  })

  // ==================== Seller Auto-fill ====================

  describe('seller auto-fill', () => {
    it('populates seller fields from seller config in create mode', async () => {
      renderWithProviders(<InvoiceFormPage {...defaultCreateProps()} />)

      // Wait for the page to render
      await waitFor(() => {
        expect(screen.getByText('Create Invoice')).toBeInTheDocument()
      })

      // Wait for seller config to load and form to reset with populated values
      await waitFor(
        () => {
          expect(screen.getByTestId('invoice-seller-name')).toHaveValue('Default Seller Corp')
        },
        { timeout: 3000 }
      )

      expect(screen.getByTestId('invoice-seller-email')).toHaveValue('seller@default.com')
      expect(screen.getByTestId('invoice-seller-address')).toHaveValue('789 Default Blvd')
      expect(screen.getByTestId('invoice-seller-phone')).toHaveValue('999-888-7777')
    })

    it('leaves seller fields empty when no seller config exists', async () => {
      server.use(sellerConfigHandler(null), createInvoiceHandler())

      renderWithProviders(<InvoiceFormPage {...defaultCreateProps()} />)

      await waitFor(() => {
        expect(screen.getByText('Create Invoice')).toBeInTheDocument()
      })

      // Seller fields should remain empty (no config returned)
      expect(screen.getByTestId('invoice-seller-name')).toHaveValue('')
    })
  })

  // ==================== Successful Submit ====================

  describe('successful submit', () => {
    it('calls create mutation with correct payload on valid form submit', async () => {
      let capturedBody: unknown = null
      server.use(
        sellerConfigHandler(),
        http.post(`${BASE_URL}/api/bill/${REALM_ID}/invoices`, async ({ request }) => {
          capturedBody = await request.json()
          return HttpResponse.json(makeInvoiceDetail(), { status: 201 })
        })
      )

      const user = userEvent.setup()
      renderWithProviders(<InvoiceFormPage {...defaultCreateProps()} />)

      await waitFor(() => {
        expect(screen.getByText('Create Invoice')).toBeInTheDocument()
      })

      // Fill required fields
      await user.type(screen.getByTestId('invoice-account-id'), 'acc-test-1')
      await user.type(screen.getByTestId('invoice-billing-name'), 'Acme Corp')

      // Seller name should be auto-filled from config, ensure it has a value
      const sellerNameInput = screen.getByTestId('invoice-seller-name')
      if (sellerNameInput.getAttribute('value') === '') {
        await user.type(sellerNameInput, 'Seller Corp')
      }

      // Fill line item
      await user.type(screen.getByTestId('invoice-line-item-name-0'), 'Consulting')
      await user.clear(screen.getByTestId('invoice-line-item-quantity-0'))
      await user.type(screen.getByTestId('invoice-line-item-quantity-0'), '10')
      await user.clear(screen.getByTestId('invoice-line-item-unit-price-0'))
      await user.type(screen.getByTestId('invoice-line-item-unit-price-0'), '50')

      // Fill due date
      await user.type(screen.getByTestId('invoice-due-date'), '2025-08-01')

      // Submit
      await user.click(screen.getByTestId('invoice-form-submit-button'))

      await waitFor(() => {
        expect(capturedBody).not.toBeNull()
      })

      const body = capturedBody as Record<string, unknown>
      expect(body.accountId).toBe('acc-test-1')
      expect(body.billingName).toBe('Acme Corp')
      expect(body.dueDate).toBe('2025-08-01')

      // Verify line items — unitPrice should be converted to cents
      const lineItems = body.lineItems as Array<Record<string, unknown>>
      expect(lineItems).toHaveLength(1)
      expect(lineItems[0].name).toBe('Consulting')
      expect(lineItems[0].quantity).toBe('10')
      expect(lineItems[0].unitPrice).toBe(5000)

      // Should navigate back after successful submit
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/$realmId/manage/billing/invoices',
        params: { realmId: REALM_ID },
      })
    })

    it('calls update mutation in edit mode on valid form submit', async () => {
      let capturedBody: unknown = null
      const invoiceId = 'inv-edit-1'
      server.use(
        http.patch(
          `${BASE_URL}/api/bill/${REALM_ID}/invoices/${invoiceId}`,
          async ({ request }) => {
            capturedBody = await request.json()
            return HttpResponse.json(makeInvoiceDetail({ id: invoiceId }))
          }
        )
      )

      const user = userEvent.setup()
      const invoice = makeInvoiceDetail({ id: invoiceId })

      renderWithProviders(<InvoiceFormPage mode="edit" realmId={REALM_ID} invoice={invoice} />)

      await waitFor(() => {
        expect(screen.getByText('Edit Invoice')).toBeInTheDocument()
      })

      // Fields should already be populated from invoice data
      // Change billing name
      const billingNameInput = screen.getByTestId('invoice-billing-name')
      await user.clear(billingNameInput)
      await user.type(billingNameInput, 'Updated Buyer')

      // Submit
      await user.click(screen.getByTestId('invoice-form-submit-button'))

      await waitFor(() => {
        expect(capturedBody).not.toBeNull()
      })

      const body = capturedBody as Record<string, unknown>
      expect(body.billingName).toBe('Updated Buyer')
      // Edit mode should NOT send accountId
      expect(body).not.toHaveProperty('accountId')

      // Should navigate back after successful submit
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/$realmId/manage/billing/invoices',
        params: { realmId: REALM_ID },
      })
    })
  })

  // ==================== Cancel / Navigate Back ====================

  describe('cancel and back navigation', () => {
    it('navigates back when Cancel button is clicked', async () => {
      const user = userEvent.setup()
      renderWithProviders(<InvoiceFormPage {...defaultCreateProps()} />)

      await waitFor(() => {
        expect(screen.getByText('Create Invoice')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('invoice-form-cancel-button'))

      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/$realmId/manage/billing/invoices',
        params: { realmId: REALM_ID },
      })
    })

    it('navigates back when back arrow button is clicked', async () => {
      const user = userEvent.setup()
      renderWithProviders(<InvoiceFormPage {...defaultCreateProps()} />)

      await waitFor(() => {
        expect(screen.getByText('Create Invoice')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('invoice-form-back-button'))

      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/$realmId/manage/billing/invoices',
        params: { realmId: REALM_ID },
      })
    })
  })
})
