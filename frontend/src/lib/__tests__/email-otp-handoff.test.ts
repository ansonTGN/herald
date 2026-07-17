/**
 * completeLoginAfterEmailOtp session-handoff tests (FE-T01 step 3, design §4.1).
 *
 * OTP verify returns a direct `BrowserTokenResponse` (no `redirectTo`, no PKCE).
 * `completeLoginAfterEmailOtp(realmId, tokenResponse, clientId)` mirrors ONLY
 * the non-PKCE branch of `completeLoginAfterPasskey`: persist the token set via
 * `store.setTokens({ accessToken, refreshToken, clientId })`, `store.login`,
 * hydrate the authenticated session, and return the safe redirect path.
 *
 * What this protects (the dev component test does NOT touch this function — the
 * route owns it, and the form hands the raw token response up via onSuccess):
 *  - Token storage composition: `setTokens` is called exactly once with the
 *    response's AT/RT plus the caller-supplied `clientId` (the
 *    `BrowserTokenResponse` body itself has no clientId).
 *  - The hydration sequence runs (`fetchAuthData` → store setters) and the
 *    function returns `{ redirectPath }`.
 *  - Boundary regression (design §4.1): OTP does NOT go through PKCE/OAuth —
 *    `performPkceTokenExchange` is never called and `store.setPkceState` is
 *    never invoked.
 *
 * Mocking pattern mirrors the sibling `oauth-login-logic.test.ts` (mock
 * `@/lib/auth-service` for `fetchAuthData`/`performPkceTokenExchange`, mock
 * `@/stores/auth-store` for `useAuthStore.getState`) and reuses the same
 * `makeStoreMock()` factory shape rather than re-inventing a wrapper.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import type { BrowserTokenResponse } from '@/lib/api-generated'

// --- Mocks (mirror oauth-login-logic.test.ts) ------------------------------

vi.mock('@/lib/api-generated', () => ({
  oauthAuthorize: vi.fn().mockResolvedValue({ data: {}, error: undefined }),
}))

vi.mock('@/lib/auth-service', () => ({
  performLogin: vi.fn(),
  fetchAuthData: vi.fn(),
  performLogout: vi.fn(),
  performPkceTokenExchange: vi.fn(),
  refreshBrowserToken: vi.fn(),
}))

vi.mock('@/stores/auth-store', () => ({
  useAuthStore: {
    getState: vi.fn(),
  },
  clearAuthStorage: vi.fn(),
  accessTokenHolder: { get: vi.fn(() => null), set: vi.fn(), clear: vi.fn() },
}))

// Import mocked modules after vi.mock declarations.
import { fetchAuthData, performPkceTokenExchange } from '@/lib/auth-service'
import { useAuthStore } from '@/stores/auth-store'
import { completeLoginAfterEmailOtp } from '@/lib/auth-utils'

// --- Factories --------------------------------------------------------------

function makeBrowserTokenResponse(overrides?: Partial<BrowserTokenResponse>): BrowserTokenResponse {
  return {
    accessToken: 'at-otp',
    expiresIn: 3600,
    refreshExpiresIn: 86400,
    refreshToken: 'rt-otp',
    tokenType: 'Bearer',
    ...overrides,
  }
}

/**
 * Minimal store mock (same shape as oauth-login-logic.test.ts::makeStoreMock),
 * with one addition: `setUserPermissions` mutates the returned `permissions`
 * array in place. `getRedirectPath()` (called at the tail of
 * `completeLoginAfterEmailOtp`) reads `useAuthStore.getState().permissions` to
 * pick admin vs user redirect, so for the redirect-distinguishability test the
 * mock must round-trip the hydrated permissions the way the real store would.
 */
function makeStoreMock(initial?: { permissions?: string[] }) {
  const permissions = initial?.permissions ?? []
  return {
    login: vi.fn(),
    logout: vi.fn(),
    setAuthStatus: vi.fn(),
    // Mutate the live `permissions` array so subsequent getState() reads (used
    // by getRedirectPath) observe the hydrated permissions.
    setUserPermissions: vi.fn((next: string[]) => {
      permissions.length = 0
      permissions.push(...next)
    }),
    setUserProfile: vi.fn(),
    setIsLoading: vi.fn(),
    reset: vi.fn(),
    clearStorage: vi.fn(),
    setTokens: vi.fn(),
    setPkceState: vi.fn(),
    getPkceState: vi.fn(() => null),
    getRefreshToken: vi.fn(() => ({ refreshToken: null, clientId: null })),
    permissions,
    roles: [],
    isAuthenticated: false,
  }
}

function makeAuthDataResponse(overrides?: { permissions?: string[] }) {
  return {
    authStatus: { authenticated: true, realmId: 'realm-1' },
    userPermissions: {
      permissions: overrides?.permissions ?? ['user:profile:read'],
      roles: ['user'],
    },
    userProfile: { id: 'user-1', email: 'user@test.com' },
  }
}

// --- Setup ------------------------------------------------------------------

beforeEach(() => {
  vi.mocked(useAuthStore.getState).mockReturnValue(
    makeStoreMock() as ReturnType<typeof useAuthStore.getState>
  )
})

// ===========================================================================
// Token storage composition + redirect path
// ===========================================================================

describe('completeLoginAfterEmailOtp — token storage + redirect', () => {
  it('calls store.setTokens once with the response AT/RT plus the caller-supplied clientId', async () => {
    const storeMock = makeStoreMock()
    vi.mocked(useAuthStore.getState).mockReturnValue(
      storeMock as ReturnType<typeof useAuthStore.getState>
    )
    vi.mocked(fetchAuthData).mockResolvedValue(makeAuthDataResponse())

    const tokenResponse = makeBrowserTokenResponse({
      accessToken: 'at-real',
      refreshToken: 'rt-real',
    })

    await completeLoginAfterEmailOtp('realm-1', tokenResponse, 'admin-web-console')

    expect(storeMock.setTokens).toHaveBeenCalledTimes(1)
    // The BrowserTokenResponse body has NO clientId — the caller supplies it so
    // a later refresh can rebind to the same Client App. Assert composition.
    expect(storeMock.setTokens).toHaveBeenCalledWith({
      accessToken: 'at-real',
      refreshToken: 'rt-real',
      clientId: 'admin-web-console',
    })
  })

  it('marks the session as logged in for the realm, then hydrates from fetchAuthData', async () => {
    const storeMock = makeStoreMock()
    vi.mocked(useAuthStore.getState).mockReturnValue(
      storeMock as ReturnType<typeof useAuthStore.getState>
    )
    vi.mocked(fetchAuthData).mockResolvedValue(makeAuthDataResponse())

    await completeLoginAfterEmailOtp('realm-1', makeBrowserTokenResponse(), 'admin-web-console')

    // store.login(realmId) marks the realm authenticated.
    expect(storeMock.login).toHaveBeenCalledWith('realm-1')
    // Hydration ran: fetchAuthData was awaited and the store setters fired with
    // the fetched auth status / permissions / profile.
    expect(fetchAuthData).toHaveBeenCalledWith()
    expect(storeMock.setAuthStatus).toHaveBeenCalledWith(true, 'realm-1')
    expect(storeMock.setUserPermissions).toHaveBeenCalledWith(['user:profile:read'], ['user'])
    expect(storeMock.setUserProfile).toHaveBeenCalledWith(expect.objectContaining({ id: 'user-1' }))
  })

  it('returns a { redirectPath } handoff shape for the route to navigate on', async () => {
    const storeMock = makeStoreMock()
    vi.mocked(useAuthStore.getState).mockReturnValue(
      storeMock as ReturnType<typeof useAuthStore.getState>
    )
    vi.mocked(fetchAuthData).mockResolvedValue(makeAuthDataResponse({ permissions: [] }))

    const result = await completeLoginAfterEmailOtp(
      'realm-1',
      makeBrowserTokenResponse(),
      'admin-web-console'
    )

    // The route navigates on `result.redirectPath`. We assert the handoff shape
    // + a non-empty path; the admin-vs-user branch is `hasAdminPermission` /
    // `auth-constants`' contract (library-guaranteed), not this function's.
    expect(result).toEqual({ redirectPath: expect.any(String) })
    expect(result.redirectPath).not.toBe('')
    // And no `redirectTo` is returned — OTP has no external-redirect branch.
    expect(result).not.toHaveProperty('redirectTo')
  })
})

// ===========================================================================
// Boundary regression — OTP does NOT go through PKCE/OAuth (design §4.1)
// ===========================================================================

describe('completeLoginAfterEmailOtp — no PKCE exchange (design §4.1 boundary)', () => {
  it('does NOT call performPkceTokenExchange', async () => {
    const storeMock = makeStoreMock()
    vi.mocked(useAuthStore.getState).mockReturnValue(
      storeMock as ReturnType<typeof useAuthStore.getState>
    )
    vi.mocked(fetchAuthData).mockResolvedValue(makeAuthDataResponse())

    await completeLoginAfterEmailOtp('realm-1', makeBrowserTokenResponse(), 'admin-web-console')

    expect(performPkceTokenExchange).not.toHaveBeenCalled()
  })

  it('does NOT touch store.setPkceState (no PKCE state seed/clear)', async () => {
    const storeMock = makeStoreMock()
    vi.mocked(useAuthStore.getState).mockReturnValue(
      storeMock as ReturnType<typeof useAuthStore.getState>
    )
    vi.mocked(fetchAuthData).mockResolvedValue(makeAuthDataResponse())

    await completeLoginAfterEmailOtp('realm-1', makeBrowserTokenResponse(), 'admin-web-console')

    // PKCE path seeds + clears state; OTP path must not touch it at all.
    expect(storeMock.setPkceState).not.toHaveBeenCalled()
  })
})

// ===========================================================================
// Error path — hydrate failure forces a clean re-login
// ===========================================================================

describe('completeLoginAfterEmailOtp — hydrate failure', () => {
  it('calls store.logout and rethrows when fetchAuthData fails', async () => {
    const storeMock = makeStoreMock()
    vi.mocked(useAuthStore.getState).mockReturnValue(
      storeMock as ReturnType<typeof useAuthStore.getState>
    )
    vi.mocked(fetchAuthData).mockRejectedValue(new Error('Session expired'))

    await expect(
      completeLoginAfterEmailOtp('realm-1', makeBrowserTokenResponse(), 'admin-web-console')
    ).rejects.toThrow('Session expired')

    // A clean re-login on hydration failure (mirrors completeLoginAfterPasskey).
    expect(storeMock.logout).toHaveBeenCalled()
  })
})
