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
  }
})

vi.mock('@/data/query-options', () => ({
  publicConfigQueryOptions: (realmId: string) => ({
    queryKey: ['public-config', realmId],
    queryFn: async () => ({
      realmName: 'Email Realm',
      whiteLabel: {
        brandName: 'Email Brand',
        logoUrl: 'https://cdn.example.com/email.svg',
      },
    }),
  }),
  turnstileStatusQueryOptions: (realmId: string) => ({
    queryKey: ['turnstile-status', realmId],
    queryFn: async () => ({ enabled: false, site_key: null }),
  }),
}))

import { VerifyEmailPage } from '../verify-email'

describe('verify-email route white-label integration', () => {
  beforeEach(() => {
    window.history.replaceState({}, '', '/test-realm/auth/verify-email')
  })

  it('loads and renders the realm brand around every verification state', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <VerifyEmailPage />
      </QueryClientProvider>
    )
    expect(await screen.findByTestId('auth-brand-logo')).toHaveAttribute(
      'src',
      'https://cdn.example.com/email.svg'
    )
    expect(screen.getByTestId('verify-email-title')).toBeInTheDocument()
    expect(document.title).toBe('Email Brand')
  })
})
