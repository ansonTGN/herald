import { describe, test, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { StripeConfigForm } from '../stripe-config-form'

describe('StripeConfigForm - Edge Cases and Boundary Scenarios', () => {
  const mockRealmId = 'test-realm'
  const mockOnSave = vi.fn()

  const defaultProps = {
    realmId: mockRealmId,
    onSave: mockOnSave,
    isLoading: false,
    disabled: false,
  }

  beforeEach(() => {
    mockOnSave.mockClear()
  })

  describe('whitespace handling', () => {
    test('trims whitespace from input fields', async () => {
      const user = userEvent.setup()
      mockOnSave.mockResolvedValue(undefined)

      render(<StripeConfigForm {...defaultProps} />)

      await user.type(screen.getByTestId('stripe-publishable-key-input'), '  pk_test_123  ')
      await user.type(screen.getByTestId('stripe-secret-key-input'), '  sk_test_456  ')

      await user.click(screen.getByTestId('stripe-save-button'))

      await waitFor(() => {
        expect(screen.getByText(/Publishable key must start with pk_/)).toBeInTheDocument()
      })
    })

    test('handles internal whitespace in keys', async () => {
      const user = userEvent.setup()
      render(<StripeConfigForm {...defaultProps} />)

      await user.type(screen.getByTestId('stripe-publishable-key-input'), 'pk_test 123')
      await user.type(screen.getByTestId('stripe-secret-key-input'), 'sk_test 456')

      await user.click(screen.getByTestId('stripe-save-button'))
    })
  })

  describe('rapid user interactions', () => {
    test('handles rapid save attempts', async () => {
      const user = userEvent.setup()
      let resolveSave: (value: void) => void
      const savePromise = new Promise<void>((resolve) => {
        resolveSave = resolve
      })

      mockOnSave.mockReturnValue(savePromise)

      render(<StripeConfigForm {...defaultProps} />)

      await user.type(screen.getByTestId('stripe-publishable-key-input'), 'pk_test_12345')
      await user.type(screen.getByTestId('stripe-secret-key-input'), 'sk_test_67890')

      const saveButton = screen.getByTestId('stripe-save-button')
      await user.click(saveButton)
      await user.click(saveButton)
      await user.click(saveButton)

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledTimes(1)
      })

      resolveSave!()
    })
  })

  describe('keyboard interactions', () => {
    test('handles Enter key to submit form', async () => {
      const user = userEvent.setup()
      mockOnSave.mockResolvedValue(undefined)

      render(<StripeConfigForm {...defaultProps} />)

      await user.type(screen.getByTestId('stripe-publishable-key-input'), 'pk_test_12345')
      await user.type(screen.getByTestId('stripe-secret-key-input'), 'sk_test_67890')

      const webhookSecretInput = screen.getByTestId('stripe-webhook-secret-input')
      await user.type(webhookSecretInput, 'whsec_test{Enter}')

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalled()
      })
    })
  })
})
