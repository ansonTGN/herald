import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ClientAppWizard } from '../client-app-wizard'

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

describe('ClientAppWizard - State Machine Step Transitions', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
  })

  it('should initialize at step 0 (Basic Information)', () => {
    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    expect(screen.getByTestId('wizard-step-basic')).toBeInTheDocument()
    expect(screen.queryByTestId('wizard-step-redirect-uris')).not.toBeInTheDocument()
    expect(screen.queryByTestId('wizard-step-security')).not.toBeInTheDocument()
    expect(screen.queryByTestId('wizard-step-review')).not.toBeInTheDocument()
  })

  it('should move to step 1 (Redirect URIs) when Next is clicked', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill required fields in step 1
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    // Now navigation should work
    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
      expect(screen.queryByTestId('wizard-step-basic')).not.toBeInTheDocument()
    })
  })

  it('should move to step 2 (Security) when Next is clicked twice', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    // First click: Step 0 -> Step 1
    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
    })

    // Fill step 2 - add a redirect URI
    const redirectInput = screen.getByTestId('redirect-uris-field')
    await user.type(redirectInput, 'https://example.com/callback{enter}')

    // Second click: Step 1 -> Step 2
    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-security')).toBeInTheDocument()
      expect(screen.queryByTestId('wizard-step-redirect-uris')).not.toBeInTheDocument()
    })
  })

  it('should move to step 3 (Review) when Next is clicked three times', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    // Step 0 -> Step 1
    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
    })

    // Fill step 2 - add a redirect URI
    const redirectInput = screen.getByTestId('redirect-uris-field')
    await user.type(redirectInput, 'https://example.com/callback{enter}')

    // Step 1 -> Step 2
    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-security')).toBeInTheDocument()
    })

    // Step 2 -> Step 3 (step 3 has default values that pass validation)
    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-review')).toBeInTheDocument()
      expect(screen.queryByTestId('wizard-step-security')).not.toBeInTheDocument()
    })
  })

  it('should move back from step 1 to step 0 when Back is clicked', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1 to enable navigation
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    // Move to step 1
    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
    })

    // Move back to step 0
    await user.click(screen.getByTestId('back-button'))

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-basic')).toBeInTheDocument()
      expect(screen.queryByTestId('wizard-step-redirect-uris')).not.toBeInTheDocument()
    })
  })

  it('should not show Back button on step 0', () => {
    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    expect(screen.queryByTestId('back-button')).not.toBeInTheDocument()
  })

  it('should not show Next button on step 3 (last step)', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    await user.click(screen.getByTestId('next-button'))
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    // Fill step 2
    const redirectInput = screen.getByTestId('redirect-uris-field')
    await user.type(redirectInput, 'https://example.com/callback{enter}')

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

    expect(screen.queryByTestId('next-button')).not.toBeInTheDocument()
    expect(screen.getByTestId('submit-button')).toBeInTheDocument()
  })

  it('should not show Submit button on step 0', () => {
    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    expect(screen.queryByTestId('submit-button')).not.toBeInTheDocument()
  })
})

describe('ClientAppWizard - Transition States', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
  })

  it('should disable navigation during forward transition', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1 to enable navigation
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    const nextButton = screen.getByTestId('next-button')

    // Click to trigger transition
    await user.click(nextButton)

    // Button should be disabled during 200ms transition
    expect(nextButton).toBeDisabled()

    // Wait for transition to complete
    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
    })

    // Button should be enabled again after transition
    await waitFor(
      () => {
        expect(nextButton).not.toBeDisabled()
      },
      { timeout: 300 }
    )
  })

  it('should disable navigation during backward transition', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1 to enable navigation
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    // Move to step 1
    await user.click(screen.getByTestId('next-button'))

    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    const backButton = screen.getByTestId('back-button')

    // Click to trigger backward transition
    await user.click(backButton)

    // Button should be disabled during transition
    expect(backButton).toBeDisabled()

    // Wait for transition to complete and step to change
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-basic')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    // After transition completes and we're back on step 0, there should be no back button
    expect(screen.queryByTestId('back-button')).not.toBeInTheDocument()
  })

  it('should prevent rapid clicks during transition', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1 to enable navigation
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    const nextButton = screen.getByTestId('next-button')

    // Rapid clicks
    await user.click(nextButton)
    await user.click(nextButton)
    await user.click(nextButton)

    // Should only move one step forward, not multiple
    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
      expect(screen.queryByTestId('wizard-step-security')).not.toBeInTheDocument()
    })
  })

  it('should use stepKey to force re-render on step change', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1 to enable navigation
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    // Step key should increment with each navigation
    const initialStep = screen.getByTestId('wizard-step-basic')

    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
      // The step should be re-rendered due to key change
      expect(initialStep).not.toBeInTheDocument()
    })
  })
})
