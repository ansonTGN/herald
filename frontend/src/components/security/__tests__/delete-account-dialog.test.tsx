import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { DeleteAccountDialog } from '../DeleteAccountDialog'

const mockClear = vi.fn()

vi.mock('@/lib/api-generated/sdk.gen', async (importOriginal) => {
  const original = await importOriginal<typeof import('@/lib/api-generated/sdk.gen')>()
  return {
    ...original,
    deleteAccount: vi.fn(),
  }
})

// Mock the reauth flow (delete_account): begin → verify → token. The index
// re-exports the sdk functions, so mock them here.
vi.mock('@/lib/api-generated', async (importOriginal) => {
  const original = await importOriginal<typeof import('@/lib/api-generated')>()
  return {
    ...original,
    handleBeginReauth: vi.fn(),
    handleVerifyReauth: vi.fn(),
  }
})

vi.mock('@/stores/auth-store', () => {
  const reset = vi.fn()
  return {
    useAuthStore: {
      getState: () => ({ reset }),
    },
    clearAuthStorage: vi.fn(),
  }
})

import { deleteAccount } from '@/lib/api-generated/sdk.gen'
import { handleBeginReauth, handleVerifyReauth } from '@/lib/api-generated'
import { clearAuthStorage, useAuthStore } from '@/stores/auth-store'

const mockDeleteAccount = vi.mocked(deleteAccount)
const mockClearAuthStorage = vi.mocked(clearAuthStorage)
const mockReset = vi.mocked(useAuthStore.getState().reset)
const mockBeginReauth = vi.mocked(handleBeginReauth)
const mockVerifyReauth = vi.mocked(handleVerifyReauth)

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  })
}

function renderDialog(props: { open?: boolean; onOpenChange?: (open: boolean) => void } = {}) {
  const queryClient = createTestQueryClient()
  queryClient.clear = mockClear
  return render(
    <QueryClientProvider client={queryClient}>
      <DeleteAccountDialog
        realmId="test-realm"
        open={props.open ?? true}
        onOpenChange={props.onOpenChange ?? vi.fn()}
      />
    </QueryClientProvider>
  )
}

describe('DeleteAccountDialog', () => {
  const originalHref = window.location.href

  beforeEach(() => {
    vi.clearAllMocks()
    Object.defineProperty(window, 'location', {
      writable: true,
      value: { href: originalHref },
    })

    // Default reauth flow mocks (delete_account): begin → verify → token.
    mockBeginReauth.mockResolvedValue({
      data: { availableFactors: ['password'] },
      error: undefined,
    })
    mockVerifyReauth.mockResolvedValue({
      data: { reauthToken: 'reauth-token-123', expiresIn: 120 },
      error: undefined,
    })
  })

  afterEach(() => {
    Object.defineProperty(window, 'location', {
      writable: true,
      value: { href: originalHref },
    })
  })

  it('GIVEN open dialog THEN shows title, description and consequences', () => {
    renderDialog()

    expect(screen.getByTestId('delete-account-dialog-title')).toHaveTextContent('Delete Account')
    expect(screen.getByTestId('delete-account-dialog-description')).toHaveTextContent(
      'This action cannot be undone. Please enter your password to confirm.'
    )
    expect(screen.getByTestId('delete-account-password-input')).toBeInTheDocument()
    expect(
      screen.getByText('Your account will be anonymized and you will not be able to log in again.')
    ).toBeInTheDocument()
    expect(
      screen.getByText('Any active subscriptions will be cancelled immediately.')
    ).toBeInTheDocument()
    expect(screen.getByText('All active sessions will be invalidated.')).toBeInTheDocument()
    expect(screen.getByText('One-time purchases will not be refunded.')).toBeInTheDocument()
    expect(
      screen.getByText('There is no self-service recovery or grace period.')
    ).toBeInTheDocument()
  })

  it('GIVEN empty password WHEN submit THEN shows required validation error', async () => {
    const user = userEvent.setup()
    renderDialog()

    await user.click(screen.getByTestId('delete-account-submit-button'))

    await waitFor(() => {
      expect(screen.getByText('Password is required')).toBeInTheDocument()
    })
  })

  it('GIVEN valid password WHEN submit succeeds THEN clears state and navigates to login', async () => {
    const user = userEvent.setup()
    mockDeleteAccount.mockResolvedValue({
      data: undefined,
      error: undefined,
    })

    renderDialog()

    await user.type(screen.getByTestId('delete-account-password-input'), 'correct-password')
    await user.click(screen.getByTestId('delete-account-submit-button'))

    await waitFor(() => {
      expect(mockDeleteAccount).toHaveBeenCalledWith({
        body: { reauthToken: 'reauth-token-123' },
      })
    })

    await waitFor(() => {
      expect(mockClear).toHaveBeenCalled()
      expect(mockReset).toHaveBeenCalled()
      expect(mockClearAuthStorage).toHaveBeenCalled()
      expect(window.location.href).toBe('/test-realm/auth/login')
    })
  })

  it('GIVEN 401 error WHEN submit fails THEN shows password error without account existence details', async () => {
    const user = userEvent.setup()
    mockDeleteAccount.mockResolvedValue({
      data: undefined,
      error: { message: 'Unauthorized', status: 401 },
    })

    renderDialog()

    await user.type(screen.getByTestId('delete-account-password-input'), 'wrong-password')
    await user.click(screen.getByTestId('delete-account-submit-button'))

    await waitFor(() => {
      expect(screen.getByTestId('delete-account-error-alert')).toBeInTheDocument()
      expect(screen.getByTestId('delete-account-error-message')).toHaveTextContent(
        'The password you entered is incorrect. Please try again.'
      )
    })
  })

  it('GIVEN 409 error WHEN submit fails THEN shows already-deleted error without existence details', async () => {
    const user = userEvent.setup()
    mockDeleteAccount.mockResolvedValue({
      data: undefined,
      error: { message: 'Conflict', status: 409 },
    })

    renderDialog()

    await user.type(screen.getByTestId('delete-account-password-input'), 'any-password')
    await user.click(screen.getByTestId('delete-account-submit-button'))

    await waitFor(() => {
      expect(screen.getByTestId('delete-account-error-alert')).toBeInTheDocument()
      expect(screen.getByTestId('delete-account-error-message')).toHaveTextContent(
        'This account cannot be deleted. It may have already been deleted.'
      )
    })
  })

  it('GIVEN generic error WHEN submit fails THEN shows generic error', async () => {
    const user = userEvent.setup()
    mockDeleteAccount.mockResolvedValue({
      data: undefined,
      error: { message: 'Server error', status: 500 },
    })

    renderDialog()

    await user.type(screen.getByTestId('delete-account-password-input'), 'password')
    await user.click(screen.getByTestId('delete-account-submit-button'))

    await waitFor(() => {
      expect(screen.getByTestId('delete-account-error-alert')).toBeInTheDocument()
      expect(screen.getByTestId('delete-account-error-message')).toHaveTextContent(
        'Account deletion failed. Please try again later.'
      )
    })
  })

  it('WHEN cancel button is clicked THEN calls onOpenChange with false', async () => {
    const user = userEvent.setup()
    const onOpenChange = vi.fn()
    renderDialog({ onOpenChange })

    await user.click(screen.getByTestId('delete-account-cancel-button'))

    expect(onOpenChange).toHaveBeenCalledWith(false)
  })
})
