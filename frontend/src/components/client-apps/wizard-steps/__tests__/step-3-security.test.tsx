import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Step3Security } from '../step-3-security'
import { WizardFormProvider } from '../../wizard-form-context'
import { useWizardFormContext } from '../../wizard-form-context'
import { wizardSchema } from '../../wizard-schema'

// Mock form wrapper component
function TestWrapper({
  children,
  defaultValues,
}: {
  children: React.ReactNode
  defaultValues?: any
}) {
  const mockForm = {
    state: {
      values: defaultValues || {
        sessionTtlSeconds: 3600,
        sessionRenewalTtlSeconds: undefined,
      },
    },
    Field: ({ name, children }: any) => {
      const field = {
        state: {
          value: defaultValues?.[name] || 3600,
          meta: { errors: [] },
        },
        handleChange: vi.fn(),
      }
      return children(field)
    },
    handleSubmit: vi.fn(),
  }

  return <WizardFormProvider form={mockForm as any}>{children}</WizardFormProvider>
}

describe('Step3Security', () => {
  it('renders security settings form', () => {
    render(
      <TestWrapper>
        <Step3Security mode="create" />
      </TestWrapper>
    )

    expect(screen.getByTestId('security-step')).toBeInTheDocument()
    expect(screen.getByText('Security Settings')).toBeInTheDocument()
    expect(screen.getByText(/Session Time-to-Live/)).toBeInTheDocument()
  })

  it('renders session TTL presets', () => {
    render(
      <TestWrapper>
        <Step3Security mode="create" />
      </TestWrapper>
    )

    expect(screen.getByTestId('session-ttl-preset-1800')).toBeInTheDocument() // 30 min
    expect(screen.getByTestId('session-ttl-preset-3600')).toBeInTheDocument() // 1 hour
    expect(screen.getByTestId('session-ttl-preset-7200')).toBeInTheDocument() // 2 hours
    expect(screen.getByTestId('session-ttl-preset-86400')).toBeInTheDocument() // 24 hours
  })

  it('renders advanced settings collapsed by default', () => {
    render(
      <TestWrapper>
        <Step3Security mode="create" />
      </TestWrapper>
    )

    const advancedTrigger = screen.getByTestId('advanced-security-settings-trigger')
    expect(advancedTrigger).toBeInTheDocument()
    expect(advancedTrigger).toHaveTextContent('Advanced Security Settings')

    // Content should not be visible initially
    expect(screen.queryByTestId('advanced-security-settings-content')).not.toBeInTheDocument()
  })

  it('expands advanced settings when trigger is clicked', async () => {
    const user = userEvent.setup()
    render(
      <TestWrapper>
        <Step3Security mode="create" />
      </TestWrapper>
    )

    const advancedTrigger = screen.getByTestId('advanced-security-settings-trigger')
    await user.click(advancedTrigger)

    await waitFor(() => {
      expect(screen.getByTestId('advanced-security-settings-content')).toBeInTheDocument()
    })
  })

  it('displays all 8 advanced security options when expanded', async () => {
    const user = userEvent.setup()
    render(
      <TestWrapper>
        <Step3Security mode="create" />
      </TestWrapper>
    )

    // Expand advanced settings
    const advancedTrigger = screen.getByTestId('advanced-security-settings-trigger')
    await user.click(advancedTrigger)

    await waitFor(() => {
      expect(
        screen.getByTestId('advanced-option-requireProofKeyForCodeExchange')
      ).toBeInTheDocument()
      expect(screen.getByTestId('advanced-option-implicitFlowEnabled')).toBeInTheDocument()
      expect(screen.getByTestId('advanced-option-serviceAccountsEnabled')).toBeInTheDocument()
      expect(screen.getByTestId('advanced-option-directAccessGrantsEnabled')).toBeInTheDocument()
      expect(screen.getByTestId('advanced-option-standardFlowEnabled')).toBeInTheDocument()
      expect(screen.getByTestId('advanced-option-frontchannelLogoutEnabled')).toBeInTheDocument()
      expect(screen.getByTestId('advanced-option-backchannelLogoutEnabled')).toBeInTheDocument()
      expect(screen.getByTestId('advanced-option-consentRequired')).toBeInTheDocument()
    })
  })

  it('renders with initial data in edit mode', () => {
    const initialData = {
      sessionTtlSeconds: 7200,
      sessionRenewalTtlSeconds: 14400,
    }

    render(
      <TestWrapper defaultValues={initialData}>
        <Step3Security mode="edit" />
      </TestWrapper>
    )

    // Check that the custom input shows the initial value
    const customInput = screen.getByTestId('session-ttl-custom-field') as HTMLInputElement
    expect(customInput.value).toBe('7200')
  })
})
