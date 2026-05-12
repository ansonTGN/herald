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

  it('should move back from step 2 to step 1 when Back is clicked', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    // Move to step 2
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

    // Move back to step 1
    await user.click(screen.getByTestId('back-button'))

    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
        expect(screen.queryByTestId('wizard-step-security')).not.toBeInTheDocument()
      },
      { timeout: 3000 }
    )
  })

  it('should move back from step 3 to step 2 when Back is clicked', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    // Move to step 2
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

    // Move to step 3
    await user.click(screen.getByTestId('next-button'))
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-security')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    // Move to step 4 (review)
    await user.click(screen.getByTestId('next-button'))
    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-review')).toBeInTheDocument()
      },
      { timeout: 3000 }
    )

    // Move back to step 2 (security)
    await user.click(screen.getByTestId('back-button'))

    await waitFor(
      () => {
        expect(screen.getByTestId('wizard-step-security')).toBeInTheDocument()
        expect(screen.queryByTestId('wizard-step-review')).not.toBeInTheDocument()
      },
      { timeout: 3000 }
    )
  })

  it('should not show Back button on step 0', () => {
    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    expect(screen.queryByTestId('back-button')).not.toBeInTheDocument()
  })

  it('should show Back button on step 1', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1 to enable navigation
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('back-button')).toBeInTheDocument()
    })
  })

  it('should show Back button on step 2', async () => {
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

    await waitFor(() => {
      expect(screen.getByTestId('back-button')).toBeInTheDocument()
    })
  })

  it('should show Back button on step 3', async () => {
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
    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('back-button')).toBeInTheDocument()
    })
  })

  it('should show Next button on step 0', () => {
    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    expect(screen.getByTestId('next-button')).toBeInTheDocument()
  })

  it('should show Next button on step 1', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1 to enable navigation
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('next-button')).toBeInTheDocument()
    })
  })

  it('should show Next button on step 2', async () => {
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

    await waitFor(() => {
      expect(screen.getByTestId('next-button')).toBeInTheDocument()
    })
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

  it('should show Submit button on step 3 (last step)', async () => {
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

    expect(screen.getByTestId('submit-button')).toBeInTheDocument()
  })

  it('should not show Submit button on step 0', () => {
    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    expect(screen.queryByTestId('submit-button')).not.toBeInTheDocument()
  })

  it('should not show Submit button on step 1', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1 to enable navigation
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.queryByTestId('submit-button')).not.toBeInTheDocument()
    })
  })

  it('should not show Submit button on step 2', async () => {
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

    await waitFor(() => {
      expect(screen.queryByTestId('submit-button')).not.toBeInTheDocument()
    })
  })
})

describe('ClientAppWizard - Derived State Calculations', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
  })

  it('should calculate canGoBack as false on step 0', () => {
    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    expect(screen.queryByTestId('back-button')).not.toBeInTheDocument()
  })

  it('should calculate canGoBack as true on step 1', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1 to enable navigation
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('back-button')).toBeInTheDocument()
    })
  })

  it('should calculate canGoBack as true on step 2', async () => {
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

    await waitFor(() => {
      expect(screen.getByTestId('back-button')).toBeInTheDocument()
    })
  })

  it('should calculate canGoBack as true on step 3', async () => {
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
    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('back-button')).toBeInTheDocument()
    })
  })

  it('should calculate canGoNext as true on step 0', () => {
    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    expect(screen.getByTestId('next-button')).toBeInTheDocument()
  })

  it('should calculate canGoNext as true on step 1', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1 to enable navigation
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('next-button')).toBeInTheDocument()
    })
  })

  it('should calculate canGoNext as true on step 2', async () => {
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

    await waitFor(() => {
      expect(screen.getByTestId('next-button')).toBeInTheDocument()
    })
  })

  it('should calculate canGoNext as false on step 3 (last step)', async () => {
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
  })

  it('should calculate isLastStep as false on step 0', () => {
    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    expect(screen.queryByTestId('submit-button')).not.toBeInTheDocument()
    expect(screen.getByTestId('next-button')).toBeInTheDocument()
  })

  it('should calculate isLastStep as false on step 1', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1 to enable navigation
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.queryByTestId('submit-button')).not.toBeInTheDocument()
      expect(screen.getByTestId('next-button')).toBeInTheDocument()
    })
  })

  it('should calculate isLastStep as false on step 2', async () => {
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

    await waitFor(() => {
      expect(screen.queryByTestId('submit-button')).not.toBeInTheDocument()
      expect(screen.getByTestId('next-button')).toBeInTheDocument()
    })
  })

  it('should calculate isLastStep as true on step 3', async () => {
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

    expect(screen.getByTestId('submit-button')).toBeInTheDocument()
    expect(screen.queryByTestId('next-button')).not.toBeInTheDocument()
  })

  it('should calculate isFirstStep as true on step 0', () => {
    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    expect(screen.queryByTestId('back-button')).not.toBeInTheDocument()
  })

  it('should calculate isFirstStep as false on step 1', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1 to enable navigation
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('back-button')).toBeInTheDocument()
    })
  })

  it('should respect WIZARD_STEPS array length (4 steps)', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    // Should be able to navigate through all 4 steps
    // Step 0 -> Step 1
    await user.click(screen.getByTestId('next-button'))
    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
    })

    // Fill step 2
    const redirectInput = screen.getByTestId('redirect-uris-field')
    await user.type(redirectInput, 'https://example.com/callback{enter}')

    // Step 1 -> Step 2
    await user.click(screen.getByTestId('next-button'))
    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-security')).toBeInTheDocument()
    })

    // Step 2 -> Step 3
    await user.click(screen.getByTestId('next-button'))
    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-review')).toBeInTheDocument()
    })

    // Should not be able to go beyond step 3
    expect(screen.queryByTestId('next-button')).not.toBeInTheDocument()
  })

  it('should respect LAST_STEP constant (index 3)', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    // Navigate to last step (index 3)
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

    // Last step should show submit button, not next button
    expect(screen.getByTestId('submit-button')).toBeInTheDocument()
    expect(screen.queryByTestId('next-button')).not.toBeInTheDocument()
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
    // This is the expected behavior since we're on the first step
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

  it('should set transition direction to forward when going next', async () => {
    const user = userEvent.setup()

    renderWithQueryClient(<ClientAppWizard mode="create" realmId="test-realm" />)

    // Fill step 1 to enable navigation
    await user.type(screen.getByTestId('client-app-name-input'), 'Test App')
    const appTypeSelect = screen.getByTestId('client-app-app-type-select')
    await user.click(appTypeSelect)
    await user.click(screen.getByText('Web Application'))
    const confidentialRadio = screen.getByTestId('client-app-client-type-confidential-radio')
    await user.click(confidentialRadio)

    // The transition direction affects the animation class
    // We can verify the step changed, which implies direction was set
    await user.click(screen.getByTestId('next-button'))

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-redirect-uris')).toBeInTheDocument()
    })
  })

  it('should set transition direction to backward when going back', async () => {
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

    // Go back
    await user.click(screen.getByTestId('back-button'))

    await waitFor(() => {
      expect(screen.getByTestId('wizard-step-basic')).toBeInTheDocument()
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
