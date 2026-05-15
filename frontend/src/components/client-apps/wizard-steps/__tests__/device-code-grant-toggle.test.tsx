import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Step3Security } from '../step-3-security'
import { WizardFormProvider } from '../../wizard-form-context'

/**
 * Minimal mock form that supports the Field render-prop pattern
 * used by step-3-security.tsx via useWizardFormContext().
 *
 * It tracks per-field values and exposes handleChange spies so tests
 * can assert that toggling the switch calls field.handleChange with
 * the new boolean value.
 */
function createMockForm(defaultValues: Record<string, unknown> = {}) {
  const values: Record<string, unknown> = {
    sessionTtlSeconds: 3600,
    sessionRenewalTtlSeconds: undefined,
    ...defaultValues,
  }

  const handleChangeSpies: Record<string, ReturnType<typeof vi.fn>> = {}

  function getOrCreateSpy(fieldName: string) {
    if (!handleChangeSpies[fieldName]) {
      handleChangeSpies[fieldName] = vi.fn((val: unknown) => {
        values[fieldName] = val
      })
    }
    return handleChangeSpies[fieldName]
  }

  const mockForm = {
    state: {
      values,
      isSubmitted: false,
    },
    Field: ({ name, children }: { name: string; children: (field: any) => React.ReactNode }) => {
      const field = {
        state: {
          value: values[name],
          meta: { errors: [], isTouched: false },
        },
        handleChange: getOrCreateSpy(name),
      }
      return children(field)
    },
    handleSubmit: vi.fn(),
  }

  return { mockForm, handleChangeSpies, values }
}

describe('Device Code Grant toggle in Step3Security', () => {
  it('defaults to off (false) for new client apps', () => {
    const { mockForm } = createMockForm()
    render(
      <WizardFormProvider form={mockForm as any}>
        <Step3Security mode="create" />
      </WizardFormProvider>
    )

    const toggle = screen.getByTestId('device-code-grant-switch')
    expect(toggle).toHaveAttribute('aria-checked', 'false')
  })

  it('reflects saved value true for existing client apps', () => {
    const { mockForm } = createMockForm({ deviceCodeGrantEnabled: true })
    render(
      <WizardFormProvider form={mockForm as any}>
        <Step3Security mode="edit" />
      </WizardFormProvider>
    )

    const toggle = screen.getByTestId('device-code-grant-switch')
    expect(toggle).toHaveAttribute('aria-checked', 'true')
  })

  it('toggling calls field.handleChange with the new value', async () => {
    const user = userEvent.setup()
    const { mockForm, handleChangeSpies } = createMockForm()

    render(
      <WizardFormProvider form={mockForm as any}>
        <Step3Security mode="create" />
      </WizardFormProvider>
    )

    const toggle = screen.getByTestId('device-code-grant-switch')
    await user.click(toggle)

    expect(handleChangeSpies['deviceCodeGrantEnabled']).toHaveBeenCalledWith(true)
  })

  it('toggling off calls field.handleChange with false', async () => {
    const user = userEvent.setup()
    const { mockForm, handleChangeSpies } = createMockForm({
      deviceCodeGrantEnabled: true,
    })

    render(
      <WizardFormProvider form={mockForm as any}>
        <Step3Security mode="edit" />
      </WizardFormProvider>
    )

    const toggle = screen.getByTestId('device-code-grant-switch')
    await user.click(toggle)

    expect(handleChangeSpies['deviceCodeGrantEnabled']).toHaveBeenCalledWith(false)
  })
})
