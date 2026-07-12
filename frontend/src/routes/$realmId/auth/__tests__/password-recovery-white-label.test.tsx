import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

vi.mock('@tanstack/react-router', async () => {
  const actual =
    await vi.importActual<typeof import('@tanstack/react-router')>('@tanstack/react-router')
  return {
    ...actual,
    createFileRoute: () => (config: Record<string, unknown>) => config,
    useNavigate: () => vi.fn(),
    Link: ({ children, to, ...props }: React.ComponentProps<'a'> & { to: string }) => (
      <a href={to} {...props}>
        {children}
      </a>
    ),
  }
})

vi.mock('@/data/query-options', () => ({
  publicConfigQueryOptions: (realmId: string) => ({
    queryKey: ['public-config', realmId],
    queryFn: async () => ({
      realmName: 'Recovery Realm',
      whiteLabel: {
        brandName: 'Recovery Brand',
        logoUrl: 'https://cdn.example.com/recovery.svg',
      },
    }),
  }),
  turnstileStatusQueryOptions: (realmId: string) => ({
    queryKey: ['turnstile-status', realmId],
    queryFn: async () => ({ enabled: false, site_key: null }),
  }),
}))

import { ForgotPasswordPage } from '../forgot-password'
import { ResetPasswordPage } from '../reset-password'

function renderPage(page: React.ReactNode) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(<QueryClientProvider client={client}>{page}</QueryClientProvider>)
}

describe('password recovery route white-label integration', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('brands the forgot-password route from public config', async () => {
    window.history.replaceState({}, '', '/test-realm/auth/forgot-password')
    renderPage(<ForgotPasswordPage />)
    expect(await screen.findByTestId('auth-brand-logo')).toHaveAttribute(
      'src',
      'https://cdn.example.com/recovery.svg'
    )
    expect(screen.getByTestId('forgot-password-form')).toBeInTheDocument()
    expect(document.title).toBe('Recovery Brand')
  })

  it('brands the reset-password route from public config', async () => {
    window.history.replaceState({}, '', '/test-realm/auth/reset-password?code=reset-code')
    renderPage(<ResetPasswordPage />)
    expect(await screen.findByTestId('auth-brand-logo')).toHaveAttribute(
      'src',
      'https://cdn.example.com/recovery.svg'
    )
    expect(screen.getByTestId('reset-password-form')).toBeInTheDocument()
    expect(document.title).toBe('Recovery Brand')
  })
})
