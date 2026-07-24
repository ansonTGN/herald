import React from 'react'
import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

const mockNavigate = vi.fn()

vi.mock('@tanstack/react-router', async () => {
  const actual =
    await vi.importActual<typeof import('@tanstack/react-router')>('@tanstack/react-router')
  return {
    ...actual,
    createFileRoute: () => (config: Record<string, unknown>) => ({
      useParams: () => ({ realmId: 'test-realm' }),
      ...config,
    }),
    useNavigate: () => mockNavigate,
  }
})

vi.mock('@/components/profile/change-password-form', () => ({
  ChangePasswordForm: () => <div data-testid="change-password-form">Change Password Form</div>,
}))

vi.mock('@/components/profile/totp/totp-status-card', () => ({
  TotpStatusCard: () => <div data-testid="totp-status-card">TOTP Status Card</div>,
}))

vi.mock('@/components/profile/totp/totp-disable-form', () => ({
  TotpDisableForm: () => <div data-testid="totp-disable-form">TOTP Disable Form</div>,
}))

vi.mock('@/components/profile/totp/totp-regenerate-form', () => ({
  TotpRegenerateForm: () => <div data-testid="totp-regenerate-form">TOTP Regenerate Form</div>,
}))

vi.mock('@/components/profile/passkey/passkey-list', () => ({
  PasskeyList: () => <div data-testid="passkey-list">Passkey List</div>,
}))

vi.mock('@/components/profile/passkey/passkey-register-form', () => ({
  PasskeyRegisterForm: () => <div data-testid="passkey-register-form">Passkey Register Form</div>,
}))

vi.mock('@/components/ui/tabs', () => ({
  Tabs: ({ children }: { children: React.ReactNode }) => <div data-testid="tabs">{children}</div>,
  TabsList: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="tabs-list">{children}</div>
  ),
  TabsTrigger: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="tabs-trigger">{children}</div>
  ),
  TabsContent: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="tabs-content">{children}</div>
  ),
}))

vi.mock('@/lib/api-generated/sdk.gen', async (importOriginal) => {
  const original = await importOriginal<typeof import('@/lib/api-generated/sdk.gen')>()
  return {
    ...original,
    deleteAccount: vi.fn(),
  }
})

// Stub the user feature-availability query so the page never issues a real
// network call. Each test seeds its desired `passkeyEnabled` via the module
// override below.
let mockPasskeyEnabled = true
vi.mock('@/data/query-options', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/data/query-options')>()
  return {
    ...actual,
    userFeatureAvailabilityQueryOptions: {
      queryKey: ['user-feature-availability', 'test'],
      queryFn: async () => ({
        user: {
          passkeyEnabled: mockPasskeyEnabled,
          pointsVisible: false,
          subscriptionVisible: false,
          invoicesVisible: false,
        },
        invoiceEligibility: {},
      }),
    },
  }
})

import { ProfileSecurity } from '../index'

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  })
}

// `mockPasskeyEnabled` defaults to true to preserve the pre-existing
// assertions that expect the passkey tab to be present.
function renderSecurityPage(passkeyEnabled = true) {
  mockPasskeyEnabled = passkeyEnabled
  const queryClient = createTestQueryClient()
  return render(
    <QueryClientProvider client={queryClient}>
      <ProfileSecurity />
    </QueryClientProvider>
  )
}

describe('ProfileSecurity', () => {
  it('GIVEN security page THEN renders danger operations section with delete account button', () => {
    renderSecurityPage()

    expect(screen.getByTestId('danger-operations-section')).toBeInTheDocument()
    expect(screen.getByTestId('delete-account-open-button')).toHaveTextContent('Delete Account')
  })

  it('WHEN delete account button is clicked THEN opens delete account dialog', async () => {
    const user = userEvent.setup()
    renderSecurityPage()

    await user.click(screen.getByTestId('delete-account-open-button'))

    expect(screen.getByTestId('delete-account-dialog')).toBeInTheDocument()
    expect(screen.getByTestId('delete-account-dialog-title')).toHaveTextContent('Delete Account')
  })

  it('GIVEN passkey enabled for realm THEN renders the passkey management UI', async () => {
    renderSecurityPage(true)

    expect(await screen.findByTestId('passkey-list')).toBeInTheDocument()
  })

  it('GIVEN passkey disabled for realm THEN hides the passkey management UI', async () => {
    renderSecurityPage(false)
    // Wait for the query to settle so the gating has applied.
    await screen.findByTestId('tabs')

    expect(screen.queryByTestId('passkey-list')).not.toBeInTheDocument()
  })
})
