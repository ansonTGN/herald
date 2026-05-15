import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { ClaimSubscriptionDialog } from '../ClaimSubscriptionDialog'

describe('ClaimSubscriptionDialog', () => {
  const mockOnSubmit = vi.fn()
  const mockOnOpenChange = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('initial rendering', () => {
    it('GIVEN dialog open WHEN rendered THEN should display form with all fields', () => {
      render(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      expect(screen.getByTestId('claim-subscription-dialog')).toBeInTheDocument()
      expect(screen.getByText('Claim Shopify Subscription')).toBeInTheDocument()
      expect(
        screen.getByText('Enter your Shopify information to claim your subscription')
      ).toBeInTheDocument()
    })

    it('GIVEN dialog WHEN rendered THEN should show Cancel and Submit buttons', () => {
      render(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      expect(screen.getByTestId('claim-cancel-button')).toBeInTheDocument()
      expect(screen.getByTestId('claim-submit-button')).toBeInTheDocument()
      expect(screen.getByText('Claim Subscription')).toBeInTheDocument()
    })
  })

  describe('form validation', () => {
    it('GIVEN both IDs empty WHEN submit attempted THEN should show validation error', async () => {
      const user = userEvent.setup({ delay: null })
      render(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      const submitButton = screen.getByTestId('claim-submit-button')
      await user.click(submitButton)

      await waitFor(() => {
        expect(
          screen.getAllByText(/Either Shopify Customer ID or Contract ID is required/i).length
        ).toBeGreaterThan(0)
      })
      expect(mockOnSubmit).not.toHaveBeenCalled()
    })

    it('GIVEN only customer ID provided WHEN submit clicked THEN should call onSubmit with customer ID', async () => {
      const user = userEvent.setup({ delay: null })
      render(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      const customerInput = screen.getByTestId('shopify-customer-id-input')
      await user.type(customerInput, 'customer_123')

      await user.click(screen.getByTestId('claim-submit-button'))

      await waitFor(() => {
        expect(mockOnSubmit).toHaveBeenCalledWith({
          shopifyCustomerId: 'customer_123',
          contractId: '',
          grantCurrentPeriod: true,
        })
      })
    })

    it('GIVEN only contract ID provided WHEN submit clicked THEN should call onSubmit with contract ID', async () => {
      const user = userEvent.setup({ delay: null })
      render(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      const contractInput = screen.getByTestId('contract-id-input')
      await user.type(contractInput, 'gid://shopify/SubscriptionContract/123')

      await user.click(screen.getByTestId('claim-submit-button'))

      await waitFor(() => {
        expect(mockOnSubmit).toHaveBeenCalledWith({
          shopifyCustomerId: '',
          contractId: 'gid://shopify/SubscriptionContract/123',
          grantCurrentPeriod: true,
        })
      })
    })

    it('GIVEN both IDs provided WHEN submit clicked THEN should call onSubmit with both IDs', async () => {
      const user = userEvent.setup({ delay: null })
      render(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      await user.type(screen.getByTestId('shopify-customer-id-input'), 'customer_123')
      await user.type(
        screen.getByTestId('contract-id-input'),
        'gid://shopify/SubscriptionContract/456'
      )

      await user.click(screen.getByTestId('claim-submit-button'))

      await waitFor(() => {
        expect(mockOnSubmit).toHaveBeenCalledWith({
          shopifyCustomerId: 'customer_123',
          contractId: 'gid://shopify/SubscriptionContract/456',
          grantCurrentPeriod: true,
        })
      })
    })

    it('GIVEN whitespace-only customer ID WHEN submit clicked THEN should submit (whitespace is considered valid)', async () => {
      const user = userEvent.setup({ delay: null })
      render(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      const customerInput = screen.getByTestId('shopify-customer-id-input')
      await user.type(customerInput, '   ')

      await user.click(screen.getByTestId('claim-submit-button'))

      await waitFor(() => {
        expect(mockOnSubmit).toHaveBeenCalledWith({
          shopifyCustomerId: '   ',
          contractId: '',
          grantCurrentPeriod: true,
        })
      })
    })

    it('GIVEN whitespace-only contract ID WHEN submit clicked THEN should submit (whitespace is considered valid)', async () => {
      const user = userEvent.setup({ delay: null })
      render(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      const contractInput = screen.getByTestId('contract-id-input')
      await user.type(contractInput, '   ')

      await user.click(screen.getByTestId('claim-submit-button'))

      await waitFor(() => {
        expect(mockOnSubmit).toHaveBeenCalledWith({
          shopifyCustomerId: '',
          contractId: '   ',
          grantCurrentPeriod: true,
        })
      })
    })
  })

  describe('grant current period checkbox', () => {
    it('GIVEN checkbox checked WHEN submit clicked THEN should include grantCurrentPeriod: true', async () => {
      const user = userEvent.setup({ delay: null })
      render(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      await user.type(screen.getByTestId('shopify-customer-id-input'), 'customer_123')
      await user.click(screen.getByTestId('claim-submit-button'))

      await waitFor(() => {
        expect(mockOnSubmit).toHaveBeenCalledWith(
          expect.objectContaining({
            grantCurrentPeriod: true,
          })
        )
      })
    })

    it('GIVEN checkbox unchecked WHEN submit clicked THEN should include grantCurrentPeriod: false', async () => {
      const user = userEvent.setup({ delay: null })
      render(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      const checkbox = screen.getByTestId('grant-current-period-checkbox')
      await user.click(checkbox)

      await user.type(screen.getByTestId('shopify-customer-id-input'), 'customer_123')
      await user.click(screen.getByTestId('claim-submit-button'))

      await waitFor(() => {
        expect(mockOnSubmit).toHaveBeenCalledWith(
          expect.objectContaining({
            grantCurrentPeriod: false,
          })
        )
      })
    })
  })

  describe('submit button state', () => {
    it('GIVEN isSubmitting is true WHEN rendered THEN should disable submit button', () => {
      render(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
          isSubmitting={true}
        />
      )

      const submitButton = screen.getByTestId('claim-submit-button')
      expect(submitButton).toBeDisabled()
      expect(screen.getByText('Claiming...')).toBeInTheDocument()
    })

    it('GIVEN isSubmitting is false WHEN rendered THEN should enable submit button', () => {
      render(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
          isSubmitting={false}
        />
      )

      const submitButton = screen.getByTestId('claim-submit-button')
      expect(submitButton).not.toBeDisabled()
      expect(screen.getByText('Claim Subscription')).toBeInTheDocument()
    })
  })

  describe('cancel button functionality', () => {
    it('GIVEN dialog open WHEN Cancel clicked THEN should call onOpenChange with false', async () => {
      const user = userEvent.setup({ delay: null })
      render(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      await user.click(screen.getByTestId('claim-cancel-button'))

      expect(mockOnOpenChange).toHaveBeenCalledWith(false)
    })

    it('GIVEN dialog open WHEN form has data AND Cancel clicked THEN should not submit', async () => {
      const user = userEvent.setup({ delay: null })
      render(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      await user.type(screen.getByTestId('shopify-customer-id-input'), 'customer_123')
      await user.click(screen.getByTestId('claim-cancel-button'))

      expect(mockOnSubmit).not.toHaveBeenCalled()
      expect(mockOnOpenChange).toHaveBeenCalledWith(false)
    })
  })

  describe('form reset on reopen', () => {
    it('GIVEN dialog closed and reopened WHEN opened again THEN should reset form values', async () => {
      const user = userEvent.setup({ delay: null })
      const { rerender } = render(
        <ClaimSubscriptionDialog
          open={false}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      // Open dialog
      rerender(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      // Fill form
      await user.type(screen.getByTestId('shopify-customer-id-input'), 'customer_123')
      await user.click(screen.getByTestId('grant-current-period-checkbox'))

      // Close and reopen
      rerender(
        <ClaimSubscriptionDialog
          open={false}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      rerender(
        <ClaimSubscriptionDialog
          open={true}
          onOpenChange={mockOnOpenChange}
          onSubmit={mockOnSubmit}
        />
      )

      // Form should be reset
      expect(screen.getByTestId('shopify-customer-id-input')).toHaveValue('')
      expect(screen.getByTestId('grant-current-period-checkbox')).toBeChecked()
    })
  })
})
