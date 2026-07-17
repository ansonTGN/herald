import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { EmailOtpConfigForm } from '../email-otp-config-form'
import type { EmailOtpConfigForm as EmailOtpConfigFormValues } from '@/lib/schemas/realm-config'

/**
 * Email-OTP realm configuration form (FE-D02).
 *
 * Mirrors `passkey-config-form.test.tsx`: the form is a presentational editor
 * driven by an `onSave` callback. We assert the switches reflect their initial
 * config, that toggling composes the right payload, and the submission guard /
 * disabled / error states behave. The form has two independent booleans:
 * `enabled` (OTP login enablement) and `autoRegister` (auto-registration toggle).
 */
describe('EmailOtpConfigForm', () => {
  const mockOnSave = vi.fn()
  const defaultProps = {
    realmId: 'admin',
    onSave: mockOnSave,
    isLoading: false,
  }

  beforeEach(() => {
    mockOnSave.mockClear()
  })

  it('GIVEN form is rendered WHEN no initial config THEN should display all controls with default values', async () => {
    const screen = render(<EmailOtpConfigForm {...defaultProps} />)

    expect(screen.getByTestId('email-otp-enabled-switch')).toBeInTheDocument()
    expect(screen.getByTestId('email-otp-auto-register-switch')).toBeInTheDocument()
    expect(screen.getByTestId('email-otp-save-button')).toBeInTheDocument()

    expect(screen.getByTestId('email-otp-enabled-switch')).not.toBeChecked()
    expect(screen.getByTestId('email-otp-auto-register-switch')).not.toBeChecked()
  })

  it('GIVEN initial config provided WHEN rendering THEN should reflect the supplied values', async () => {
    const initialConfig: EmailOtpConfigFormValues = {
      enabled: true,
      autoRegister: false,
    }

    const screen = render(<EmailOtpConfigForm {...defaultProps} initialConfig={initialConfig} />)

    expect(screen.getByTestId('email-otp-enabled-switch')).toBeChecked()
    expect(screen.getByTestId('email-otp-auto-register-switch')).not.toBeChecked()
  })

  it('GIVEN user toggles enabled switch WHEN submitting THEN should call onSave with enabled true and defaults preserved', async () => {
    mockOnSave.mockResolvedValue(undefined)
    const screen = render(<EmailOtpConfigForm {...defaultProps} />)

    await userEvent.click(screen.getByTestId('email-otp-enabled-switch'))
    await userEvent.click(screen.getByTestId('email-otp-save-button'))

    await waitFor(() => {
      expect(mockOnSave).toHaveBeenCalledWith({
        enabled: true,
        autoRegister: false,
      })
    })
  })

  it('GIVEN user toggles autoRegister switch WHEN submitting THEN should include autoRegister true', async () => {
    mockOnSave.mockResolvedValue(undefined)
    const screen = render(<EmailOtpConfigForm {...defaultProps} />)

    await userEvent.click(screen.getByTestId('email-otp-auto-register-switch'))
    await userEvent.click(screen.getByTestId('email-otp-save-button'))

    await waitFor(() => {
      expect(mockOnSave).toHaveBeenCalledWith(
        expect.objectContaining({ enabled: false, autoRegister: true })
      )
    })
  })

  it('GIVEN isLoading prop is true WHEN rendering THEN should disable save button', async () => {
    const screen = render(<EmailOtpConfigForm {...defaultProps} isLoading={true} />)
    expect(screen.getByTestId('email-otp-save-button')).toBeDisabled()
  })

  it('GIVEN form is disabled WHEN rendering THEN should disable all controls and the save button', async () => {
    const screen = render(<EmailOtpConfigForm {...defaultProps} disabled={true} />)

    expect(screen.getByTestId('email-otp-enabled-switch')).toBeDisabled()
    expect(screen.getByTestId('email-otp-auto-register-switch')).toBeDisabled()
    expect(screen.getByTestId('email-otp-save-button')).toBeDisabled()
  })

  it('GIVEN onSave rejects WHEN submitting THEN should handle the error gracefully', async () => {
    mockOnSave.mockRejectedValue(new Error('Failed to save configuration'))
    const screen = render(<EmailOtpConfigForm {...defaultProps} />)

    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

    await userEvent.click(screen.getByTestId('email-otp-enabled-switch'))
    await userEvent.click(screen.getByTestId('email-otp-save-button'))

    await waitFor(() => {
      expect(mockOnSave).toHaveBeenCalled()
    })
    await new Promise((resolve) => setTimeout(resolve, 0))

    consoleSpy.mockRestore()
  })

  it('GIVEN form is submitting WHEN save is in progress THEN should disable the save button', async () => {
    mockOnSave.mockImplementation(() => new Promise((resolve) => setTimeout(resolve, 100)))

    const screen = render(<EmailOtpConfigForm {...defaultProps} />)
    const saveButton = screen.getByTestId('email-otp-save-button')
    await userEvent.click(saveButton)

    await waitFor(() => {
      expect(saveButton).toBeDisabled()
    })
  })
})
