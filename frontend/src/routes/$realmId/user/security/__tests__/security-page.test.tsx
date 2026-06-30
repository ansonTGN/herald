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

import { ProfileSecurity } from '../index'

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  })
}

function renderSecurityPage() {
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
})
