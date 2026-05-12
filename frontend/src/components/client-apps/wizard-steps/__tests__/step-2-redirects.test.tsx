import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Step2Redirects } from '../step-2-redirects'
import { WizardFormProvider } from '../../wizard-form-context'

// Mock the TanStack Form
const mockForm = {
  Field: vi.fn(({ children }) => (
    <div data-testid="mock-field">
      {children({
        state: {
          value: [],
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
      redirectUris: [],
      postLogoutUris: [],
      webOrigins: [],
    },
    isDirty: false,
    isSubmitted: false,
    isValid: true,
  },
}

describe('Step2Redirects', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  const renderWithProvider = (mode: 'create' | 'edit', initialValues = {}) => {
    return render(
      <WizardFormProvider form={mockForm as any}>
        <Step2Redirects mode={mode} />
      </WizardFormProvider>
    )
  }

  it('should render step heading and description for create mode', () => {
    renderWithProvider('create')

    expect(screen.getByText('Redirect URIs')).toBeInTheDocument()
    expect(screen.getByText(/Configure the allowed OAuth redirect URIs/)).toBeInTheDocument()
  })

  it('should render step heading and description for edit mode', () => {
    renderWithProvider('edit')

    expect(screen.getByText('Redirect URIs')).toBeInTheDocument()
    expect(screen.getByText(/Configure the allowed OAuth redirect URIs/)).toBeInTheDocument()
  })

  it('should render valid redirect URIs field', () => {
    renderWithProvider('create')

    expect(screen.getByText('Valid Redirect URIs')).toBeInTheDocument()
  })

  it('should render valid post logout URIs field', () => {
    renderWithProvider('create')

    expect(screen.getByText('Valid Post Logout URIs')).toBeInTheDocument()
  })

  it('should render advanced CORS settings toggle', () => {
    renderWithProvider('create')

    expect(screen.getByText('Advanced CORS Settings')).toBeInTheDocument()
  })

  it('should show help text for redirect URIs', () => {
    renderWithProvider('create')

    expect(screen.getByText(/Enter the OAuth 2.0 redirect URIs/)).toBeInTheDocument()
  })
})
