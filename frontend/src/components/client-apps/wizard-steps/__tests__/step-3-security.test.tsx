import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Step3Security } from '../step-3-security'
import { WizardFormProvider } from '../../wizard-form-context'

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
  it('renders advanced settings collapsed by default', () => {
    render(
      <TestWrapper>
        <Step3Security mode="create" />
      </TestWrapper>
    )

    const advancedTrigger = screen.getByTestId('advanced-security-settings-trigger')
    expect(advancedTrigger).toBeInTheDocument()

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
})
