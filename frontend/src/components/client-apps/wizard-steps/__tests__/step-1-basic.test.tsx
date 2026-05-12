import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Step1Basic } from '../step-1-basic'
import { WizardFormProvider } from '../../wizard-form-context'

// Mock the TanStack Form
const mockForm = {
  Field: vi.fn(({ children }) => (
    <div data-testid="mock-field">
      {children({
        state: {
          value: '',
          meta: { isTouched: false, errors: [] },
        },
        name: 'test-field',
        handleChange: vi.fn(),
        handleBlur: vi.fn(),
      })}
    </div>
  )),
  state: {
    values: {
      name: '',
      description: '',
      appType: 'WEB',
      clientType: 'CONFIDENTIAL',
    },
    isDirty: false,
    isSubmitted: false,
    isValid: true,
  },
}

describe('Step1Basic', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  const renderWithProvider = (mode: 'create' | 'edit', initialValues = {}) => {
    return render(
      <WizardFormProvider form={mockForm as any}>
        <Step1Basic mode={mode} />
      </WizardFormProvider>
    )
  }

  it('should render step heading and description for create mode', () => {
    renderWithProvider('create')

    expect(screen.getByText('Basic Information')).toBeInTheDocument()
    expect(
      screen.getByText(/Enter the basic details for your new client application/)
    ).toBeInTheDocument()
  })

  it('should render step heading and description for edit mode', () => {
    renderWithProvider('edit')

    expect(screen.getByText('Basic Information')).toBeInTheDocument()
    expect(
      screen.getByText(/Edit the basic information for this client application/)
    ).toBeInTheDocument()
  })

  it('should render all required form fields', () => {
    renderWithProvider('create')

    // Check for fields using data-testid attributes
    expect(screen.getByTestId('client-app-name-input')).toBeInTheDocument()
    expect(screen.getByTestId('client-app-description-input')).toBeInTheDocument()
    expect(screen.getByTestId('client-app-app-type-select')).toBeInTheDocument()
    expect(screen.getByTestId('client-app-client-type-radiogroup')).toBeInTheDocument()
  })

  it('should render help text for app name field', () => {
    renderWithProvider('create')

    expect(screen.getByText(/Choose a descriptive name for your app/)).toBeInTheDocument()
  })

  it('should render help text for description field', () => {
    renderWithProvider('create')

    expect(screen.getByText(/Optional description of your app's purpose/)).toBeInTheDocument()
  })
})
