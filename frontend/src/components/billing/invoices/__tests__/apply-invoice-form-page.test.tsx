/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { ApplyInvoiceFormPage } from '../apply-invoice-form-page'
import { server } from '@/test/mocks/server'
import { renderWithProviders } from '@/test/utils/render'

// ==================== Router Mock ====================

const mockNavigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

// ==================== Test Helpers ====================

const REALM_ID = 'test-realm'
const BASE_URL = 'http://localhost:3000'

function sellerConfigHandler() {
  return http.get(`${BASE_URL}/api/bill/${REALM_ID}/invoice-seller-config`, () => {
    return HttpResponse.json({
      sellerName: 'Seller Corp',
      sellerAddress: '789 Seller Ave',
      sellerEmail: 'seller@test.com',
      sellerPhone: null,
      sellerTaxId: 'TAX999',
      defaultPaymentTerms: 'Net 30',
      createdAt: '2025-01-01T00:00:00Z',
      updatedAt: '2025-01-01T00:00:00Z',
    })
  })
}

function applyInvoiceHandler() {
  return http.post(`${BASE_URL}/api/bill/${REALM_ID}/my/invoices`, async () => {
    return HttpResponse.json({ id: 'inv-new' }, { status: 201 })
  })
}

// ==================== Tests ====================

describe('ApplyInvoiceFormPage', () => {
  beforeEach(() => {
    mockNavigate.mockClear()
    server.use(sellerConfigHandler(), applyInvoiceHandler())
  })

  // ==================== Rendering ====================

  describe('rendering', () => {
    it('renders the apply invoice form with all sections', async () => {
      renderWithProviders(<ApplyInvoiceFormPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByTestId('apply-form-page')).toBeInTheDocument()
      })

      // Verify all three section cards
      expect(screen.getByTestId('apply-form-reference-section')).toBeInTheDocument()
      expect(screen.getByTestId('apply-form-billing-section')).toBeInTheDocument()
      expect(screen.getByTestId('apply-form-details-section')).toBeInTheDocument()

      // Verify key form fields are present
      expect(screen.getByTestId('apply-payment-attempt-id-input')).toBeInTheDocument()
      expect(screen.getByTestId('apply-subscription-id-input')).toBeInTheDocument()
      expect(screen.getByTestId('apply-billing-name-input')).toBeInTheDocument()
      expect(screen.getByTestId('apply-billing-email-input')).toBeInTheDocument()
      expect(screen.getByTestId('apply-billing-address-input')).toBeInTheDocument()
      expect(screen.getByTestId('apply-billing-phone-input')).toBeInTheDocument()
      expect(screen.getByTestId('apply-due-date-input')).toBeInTheDocument()
      expect(screen.getByTestId('apply-notes-input')).toBeInTheDocument()

      // Verify action buttons
      expect(screen.getByTestId('apply-invoice-submit-button')).toBeInTheDocument()
      expect(screen.getByTestId('apply-invoice-cancel-button')).toBeInTheDocument()
      expect(screen.getByTestId('apply-invoice-back-button')).toBeInTheDocument()
    })
  })

  // ==================== Validation ====================

  describe('validation', () => {
    it('submit without billingName shows validation error', async () => {
      const user = userEvent.setup()
      renderWithProviders(<ApplyInvoiceFormPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByTestId('apply-form-page')).toBeInTheDocument()
      })

      // Fill payment attempt ID so we pass the refine check
      const paymentInput = screen.getByTestId('apply-payment-attempt-id-input')
      await user.type(paymentInput, 'pay-123')

      // Submit with empty billingName (and empty billingAddress, dueDate)
      const submitButton = screen.getByTestId('apply-invoice-submit-button')
      await user.click(submitButton)

      // Should show validation error for billingName
      await waitFor(() => {
        expect(screen.getByText('Billing name is required')).toBeInTheDocument()
      })
    })

    it('submit without payment/subscription shows validation error', async () => {
      const user = userEvent.setup()
      renderWithProviders(<ApplyInvoiceFormPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByTestId('apply-form-page')).toBeInTheDocument()
      })

      // Fill billingName only (no payment/subscription)
      const billingNameInput = screen.getByTestId('apply-billing-name-input')
      await user.type(billingNameInput, 'Test Buyer')

      // Fill billingAddress (required)
      await user.type(screen.getByTestId('apply-billing-address-input'), '123 Billing St')

      // Fill dueDate (required)
      await user.type(screen.getByTestId('apply-due-date-input'), '2025-08-01')

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
  })

  // ==================== Submission ====================

  describe('submission', () => {
    it('valid submission calls apply mutation with correct payload', async () => {
      let capturedBody: unknown = null

      server.use(
        sellerConfigHandler(),
        http.post(`${BASE_URL}/api/bill/${REALM_ID}/my/invoices`, async ({ request }) => {
          capturedBody = await request.json()
          return HttpResponse.json({ id: 'inv-new' }, { status: 201 })
        })
      )

      const user = userEvent.setup()
      renderWithProviders(<ApplyInvoiceFormPage realmId={REALM_ID} />)

      // Wait for seller config to load and form to initialize
      await waitFor(() => {
        expect(screen.getByTestId('apply-form-page')).toBeInTheDocument()
      })

      // Fill payment attempt ID
      await user.type(screen.getByTestId('apply-payment-attempt-id-input'), 'pay-abc-123')

      // Fill billing name
      await user.type(screen.getByTestId('apply-billing-name-input'), 'John Doe')

      // Fill billing email
      await user.type(screen.getByTestId('apply-billing-email-input'), 'john@example.com')

      // Fill billing address (required)
      await user.type(screen.getByTestId('apply-billing-address-input'), '123 Billing St')

      // dueDate is auto-populated from sellerConfig (Net 30 terms), no need to fill

      // Submit
      await user.click(screen.getByTestId('apply-invoice-submit-button'))

      // Wait for mutation to be called
      await waitFor(() => {
        expect(capturedBody).not.toBeNull()
      })

      expect(capturedBody).toMatchObject({
        paymentAttemptId: 'pay-abc-123',
        billingName: 'John Doe',
        billingEmail: 'john@example.com',
        billingAddress: '123 Billing St',
        currency: 'CNY',
      })

      // Should navigate back after successful submit
      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/$realmId/user/invoices',
        params: { realmId: REALM_ID },
      })
    })
  })

  // ==================== Navigation ====================

  describe('cancel navigation', () => {
    it('cancel button navigates back', async () => {
      const user = userEvent.setup()
      renderWithProviders(<ApplyInvoiceFormPage realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByTestId('apply-form-page')).toBeInTheDocument()
      })

      // Click the back button in the header
      await user.click(screen.getByTestId('apply-invoice-back-button'))

      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/$realmId/user/invoices',
        params: { realmId: REALM_ID },
      })
    })
  })
})
