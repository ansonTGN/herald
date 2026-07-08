import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { PublicConfigResponse } from '@/lib/api-generated'

/**
 * FE-D03 — white-label integration for the register route.
 *
 * RegisterPage renders four distinct `AuthPageWrapper` branches based on its
 * derived state (loading, error, registration-disabled, form). The fix threads
 * the derived `whiteLabel` into each; this suite asserts the brand surfaces
 * (logo image via `auth-brand-logo`, and the `registerTitle` override on the
 * form) on every branch, plus the conditional `registerSubtitle`.
 *
 * To make a missing prop a *real* failure rather than a silent pass, the mock
 * config ships a `logoUrl`. If a branch forgets to forward `whiteLabel`, the
 * wrapper falls back to `auth-brand-text` (Herald) and the logo node is absent,
 * failing that branch's assertion.
 */

const WHITE_LABEL = {
  logoUrl: 'https://cdn.example.com/brand.svg',
  registerTitle: 'Create your Example account',
  registerSubtitle: 'Example subtitle',
}

/** Mutable per-test fixture so each `it` can flip loading/error/disabled/form. */
let publicConfigFixture: {
  data: Partial<PublicConfigResponse> | null
  loading: boolean
  error: boolean
}

vi.mock('@tanstack/react-router', async () => {
  const actual =
    await vi.importActual<typeof import('@tanstack/react-router')>('@tanstack/react-router')
  return {
    ...actual,
    createFileRoute: () => (config: Record<string, unknown>) => ({
      useParams: () => ({ realmId: 'test-realm' }),
      ...config,
    }),
    useNavigate: () => ({ navigate: vi.fn() }),
    Link: ({
      to,
      params,
      children,
      ...props
    }: {
      to: string
      params?: Record<string, string>
      children?: React.ReactNode
    }) => {
      let href = to as string
      if (params) {
        Object.entries(params).forEach(([key, value]) => {
          href = href.replace(new RegExp(`\\$\\${key}|\\$\\{${key}\\}`, 'g'), value)
        })
      }
      return (
        <a href={href} {...props}>
          {children}
        </a>
      )
    },
  }
})

vi.mock('@/components/auth/register-form', () => ({
  // Stubbed so the form branch mounts without triggering its own query/mutation.
  RegisterForm: () => <div data-testid="register-form-stub" />,
}))

vi.mock('@/data/query-options', () => ({
  publicConfigQueryOptions: () => ({
    queryKey: ['public-config', 'test-realm'],
    queryFn: () => {
      if (publicConfigFixture.loading) {
        // Never resolves → keeps the query in `isLoading` for the loading test.
        return new Promise(() => {})
      }
      if (publicConfigFixture.error) {
        return Promise.reject(new Error('boom'))
      }
      return Promise.resolve(publicConfigFixture.data)
    },
  }),
  queryKeys: {
    publicConfig: () => ['public-config', 'test-realm'],
  },
}))

import { RegisterPage } from '../register'

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
}

function renderRegisterPage() {
  return render(
    <QueryClientProvider client={createTestQueryClient()}>
      <RegisterPage />
    </QueryClientProvider>
  )
}

/** A config with registration enabled + the white-label brand. */
function enabledConfigWithBrand(): Partial<PublicConfigResponse> {
  return {
    realmName: 'Realm Name',
    registration: { enabled: true },
    whiteLabel: WHITE_LABEL,
  } as Partial<PublicConfigResponse>
}

/** A config with registration DISABLED + the white-label brand. */
function disabledConfigWithBrand(): Partial<PublicConfigResponse> {
  return {
    realmName: 'Realm Name',
    registration: { enabled: false },
    whiteLabel: WHITE_LABEL,
  } as Partial<PublicConfigResponse>
}

describe('RegisterPage white-label integration (FE-D03)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    publicConfigFixture = { data: enabledConfigWithBrand(), loading: false, error: false }
  })

  it('renders the brand logo on the form state and applies registerTitle/registerSubtitle', async () => {
    renderRegisterPage()

    await screen.findByTestId('register-card')
    expect(screen.getByTestId('auth-brand-logo')).toHaveAttribute(
      'src',
      'https://cdn.example.com/brand.svg'
    )
    expect(screen.getByTestId('register-title')).toHaveTextContent('Create your Example account')
    // registerSubtitle present → shown.
    expect(screen.getByTestId('register-subtitle')).toHaveTextContent('Example subtitle')
  })

  it('omits the subtitle when whiteLabel.registerSubtitle is absent', async () => {
    publicConfigFixture.data = {
      realmName: 'Realm Name',
      registration: { enabled: true },
      whiteLabel: { logoUrl: WHITE_LABEL.logoUrl, registerTitle: WHITE_LABEL.registerTitle },
    } as Partial<PublicConfigResponse>

    renderRegisterPage()

    await screen.findByTestId('register-card')
    expect(screen.queryByTestId('register-subtitle')).not.toBeInTheDocument()
  })

  it('renders the brand logo on the registration-disabled state', async () => {
    publicConfigFixture.data = disabledConfigWithBrand()

    renderRegisterPage()

    await screen.findByTestId('registration-disabled-title')
    // If the disabled branch forgot to forward whiteLabel, no logo renders here.
    expect(screen.getByTestId('auth-brand-logo')).toHaveAttribute(
      'src',
      'https://cdn.example.com/brand.svg'
    )
  })

  it('renders the brand wrapper (Herald fallback) on the error state', async () => {
    publicConfigFixture.error = true

    renderRegisterPage()

    // When the query rejects there is no whiteLabel to forward yet, so the
    // wrapper correctly falls back to the default Herald brand. The regression
    // value of this test: the error branch still mounts AuthPageWrapper and
    // surfaces the brand area (it does not crash or render a blank page).
    await waitFor(() => {
      expect(screen.getByTestId('auth-brand-text')).toHaveTextContent('Herald')
    })
    expect(screen.queryByTestId('auth-brand-logo')).not.toBeInTheDocument()
  })

  it('renders the brand wrapper (Herald fallback) on the loading state', async () => {
    publicConfigFixture.loading = true

    renderRegisterPage()

    // While the query is in flight there is no whiteLabel yet, so the wrapper
    // shows the default Herald brand. The loading branch must still mount
    // AuthPageWrapper so the brand area is present (not blank).
    expect(screen.getByTestId('auth-brand-text')).toHaveTextContent('Herald')
    expect(screen.queryByTestId('auth-brand-logo')).not.toBeInTheDocument()
  })
})
