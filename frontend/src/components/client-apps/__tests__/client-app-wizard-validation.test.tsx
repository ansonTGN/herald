import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ClientAppWizard } from '../client-app-wizard'
import { toast } from 'sonner'

// Mock the navigate function
const mockNavigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

// Mock the API functions
vi.mock('@/lib/api-generated', () => ({
  createClientApp: vi.fn(),
  updateClientApp: vi.fn(),
}))

// Helper function to create test query client
const createTestQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  })

// Helper function to render with query client
const renderWithQueryClient = (component: React.ReactElement) => {
  const queryClient = createTestQueryClient()
  return render(<QueryClientProvider client={queryClient}>{component}</QueryClientProvider>)
}

describe('ClientAppWizard - Validation on Submit', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
  })

  it('should show validation error on submit when name is missing', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Navigate to review without filling data (all 4 steps)
    await user.click(screen.getByTestId('next-button'))
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    await user.click(screen.getByTestId('next-button'))
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-security')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    await user.click(screen.getByTestId('next-button'))
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-review')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    // Submit with empty data
    await user.click(screen.getByTestId('submit-button'))

    // Should show validation error about basic info
    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith(
        'Please complete all required fields in Basic Information'
      )
    })

    // Should stay on review step (no navigation)
    expect(mockNavigate).not.toHaveBeenCalled()
  })

  it('should show validation error on submit when redirect URIs list is empty', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    // Navigate to step 2 without adding URIs
    await user.click(screen.getByTestId('next-button'))
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    // Navigate to step 3
    await user.click(screen.getByTestId('next-button'))
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-security')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    // Navigate to review
    await user.click(screen.getByTestId('next-button'))
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-review')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    // Submit - should fail because no redirect URIs
    await user.click(screen.getByTestId('submit-button'))

    // Basic info is valid, so it should fail on redirect URIs
    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('Please add at least one redirect URI')
    })
  })
})

describe('ClientAppWizard - Free Step Navigation (No Per-Step Validation)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
  })

  it('should allow navigating through all steps without data', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Navigate to step 2
    await user.click(screen.getByTestId('next-button'))
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    // Navigate to step 3
    await user.click(screen.getByTestId('next-button'))
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-security')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    // Navigate to review
    await user.click(screen.getByTestId('next-button'))
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-review')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    // Submit should show validation error
    await user.click(screen.getByTestId('submit-button'))

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalled()
    })
  })

  it('should show complete review sections when form is fully filled', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1 completely
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    // Navigate to step 2 and add redirect URI
    await user.click(screen.getByTestId('next-button'))
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    const redirectInput = screen.getByTestId('redirect-uris-field')
    await user.type(redirectInput, 'https://example.com/callback{enter}')

    // Navigate to step 3
    await user.click(screen.getByTestId('next-button'))
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-security')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    // Navigate to review
    await user.click(screen.getByTestId('next-button'))
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-review')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    // Check that sections exist in the review step
    const basicInfoSection = screen.getByTestId('review-basic-info')
    const redirectUrisSection = screen.getByTestId('review-redirect-uris')
    const securitySection = screen.getByTestId('review-security')

    expect(basicInfoSection).toBeInTheDocument()
    expect(redirectUrisSection).toBeInTheDocument()
    expect(securitySection).toBeInTheDocument()
  })
})
