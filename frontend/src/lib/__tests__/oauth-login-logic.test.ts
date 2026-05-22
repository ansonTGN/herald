/**
 * Tests for OAuth-related login logic:
 * - loginSearchSchema parsing (backward compatibility + OAuth fields)
 * - loginFlow redirectTo branching
 * - completeLoginAfterTotp redirectTo branching
 * - validateOAuthParams completeness check
 * - loginFlow passes OAuth fields to performLogin
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { loginSearchSchema } from '@/lib/schemas/search-params'
import type { LoginResponse, VerifyTotpResponse, LoginRequestPayload } from '@/lib/api-generated'

// --- Mocks ---

vi.mock('@/lib/auth-service', () => ({
  performLogin: vi.fn(),
  fetchAuthData: vi.fn(),
  performLogout: vi.fn(),
}))

vi.mock('@/stores/auth-store', () => ({
  useAuthStore: {
    getState: vi.fn(),
  },
  clearAuthStorage: vi.fn(),
}))

// Import mocked modules after vi.mock declarations
import { performLogin, fetchAuthData } from '@/lib/auth-service'
import { useAuthStore } from '@/stores/auth-store'
import { loginFlow, completeLoginAfterTotp, validateOAuthParams } from '@/lib/auth-utils'

// --- Factories ---

function makeLoginResponse(overrides?: Partial<LoginResponse>): LoginResponse {
  return {
    expiresInSeconds: 3600,
    message: 'Login successful',
    realmId: 'realm1',
    userId: 'user-1',
    requiresTotp: false,
    ...overrides,
  }
}

function makeVerifyTotpResponse(overrides?: Partial<VerifyTotpResponse>): VerifyTotpResponse {
  return {
    expiresInSeconds: 3600,
    message: 'TOTP verified',
    token: 'tok-1',
    userId: 'user-1',
    ...overrides,
  }
}

function makeStoreMock() {
  return {
    login: vi.fn(),
    logout: vi.fn(),
    setAuthStatus: vi.fn(),
    setUserPermissions: vi.fn(),
    setUserProfile: vi.fn(),
    setIsLoading: vi.fn(),
    reset: vi.fn(),
    clearStorage: vi.fn(),
    permissions: [],
    roles: [],
    isAuthenticated: false,
  }
}

function makeAuthDataResponse(overrides?: { permissions?: string[] }) {
  return {
    authStatus: { authenticated: true, realmId: 'realm1' },
    userPermissions: {
      permissions: overrides?.permissions ?? ['user:profile:read'],
      roles: ['user'],
    },
    userProfile: { id: 'user-1', email: 'user@test.com' },
  }
}

function baseCredentials(): LoginRequestPayload {
  return {
    clientId: 'client-1',
    username: 'user@test.com',
    password: 'password123',
  }
}

// --- Setup ---

beforeEach(() => {
  vi.mocked(useAuthStore.getState).mockReturnValue(
    makeStoreMock() as ReturnType<typeof useAuthStore.getState>
  )
})

// --- Tests ---

describe('loginSearchSchema', () => {
  it('parses empty {} — backward compatible', () => {
    const result = loginSearchSchema.parse({})
    expect(result).toEqual({
      redirect: undefined,
      clientId: undefined,
      oauthClientId: undefined,
      redirectUri: undefined,
      state: undefined,
    })
  })

  it('parses { redirect: "/manage" } — existing field works', () => {
    const result = loginSearchSchema.parse({ redirect: '/manage' })
    expect(result.redirect).toBe('/manage')
    expect(result.oauthClientId).toBeUndefined()
  })

  it('parses all 3 OAuth fields together', () => {
    const result = loginSearchSchema.parse({
      oauthClientId: 'my-client',
      redirectUri: 'https://app.example.com/callback',
      state: 'abc123',
    })
    expect(result.oauthClientId).toBe('my-client')
    expect(result.redirectUri).toBe('https://app.example.com/callback')
    expect(result.state).toBe('abc123')
  })

  it('parses redirect + all 3 OAuth fields together', () => {
    const result = loginSearchSchema.parse({
      redirect: '/manage',
      oauthClientId: 'my-client',
      redirectUri: 'https://app.example.com/callback',
      state: 'xyz789',
    })
    expect(result.redirect).toBe('/manage')
    expect(result.oauthClientId).toBe('my-client')
    expect(result.redirectUri).toBe('https://app.example.com/callback')
    expect(result.state).toBe('xyz789')
  })

  it('treats all fields as optional — missing values are undefined', () => {
    const result = loginSearchSchema.parse({ oauthClientId: 'only-one' })
    expect(result.oauthClientId).toBe('only-one')
    expect(result.redirectUri).toBeUndefined()
    expect(result.state).toBeUndefined()
    expect(result.redirect).toBeUndefined()
    expect(result.clientId).toBeUndefined()
  })
})

describe('loginFlow redirectTo logic', () => {
  it('when redirectTo present and no TOTP, returns redirectTo without calling store.login or fetchAuthData', async () => {
    const storeMock = makeStoreMock()
    vi.mocked(useAuthStore.getState).mockReturnValue(
      storeMock as ReturnType<typeof useAuthStore.getState>
    )

    vi.mocked(performLogin).mockResolvedValue(
      makeLoginResponse({ redirectTo: 'https://app.example.com/callback?code=abc' })
    )

    const result = await loginFlow('realm1', baseCredentials())

    expect(result.redirectTo).toBeUndefined()
    expect(result.response.redirectTo).toBe('https://app.example.com/callback?code=abc')
    expect(result.redirectPath).toBe('/user/profile')
    expect(storeMock.login).not.toHaveBeenCalled()
    expect(fetchAuthData).not.toHaveBeenCalled()
  })

  it('when redirectTo present and TOTP required, preserves redirectTo in response', async () => {
    vi.mocked(performLogin).mockResolvedValue(
      makeLoginResponse({
        requiresTotp: true,
        tempToken: 'temp-tok',
        redirectTo: 'https://app.example.com/callback?code=abc',
      })
    )

    const result = await loginFlow('realm1', baseCredentials())

    expect(result.response.redirectTo).toBe('https://app.example.com/callback?code=abc')
    expect(result.response.requiresTotp).toBe(true)
  })

  it('when redirectTo absent, runs existing logic with store.login and fetchAuthData', async () => {
    const storeMock = makeStoreMock()
    vi.mocked(useAuthStore.getState).mockReturnValue(
      storeMock as ReturnType<typeof useAuthStore.getState>
    )

    vi.mocked(performLogin).mockResolvedValue(makeLoginResponse({ redirectTo: undefined }))
    vi.mocked(fetchAuthData).mockResolvedValue(makeAuthDataResponse())

    const result = await loginFlow('realm1', baseCredentials())

    expect(result.redirectTo).toBeUndefined()
    expect(result.response.redirectTo).toBeUndefined()
    expect(storeMock.login).toHaveBeenCalledWith('realm1')
    expect(fetchAuthData).toHaveBeenCalledWith('realm1')
    expect(result.redirectPath).toBe('/user/profile')
  })

  it('when redirectTo is null, same as absent — runs existing logic', async () => {
    const storeMock = makeStoreMock()
    vi.mocked(useAuthStore.getState).mockReturnValue(
      storeMock as ReturnType<typeof useAuthStore.getState>
    )

    vi.mocked(performLogin).mockResolvedValue(makeLoginResponse({ redirectTo: null }))
    vi.mocked(fetchAuthData).mockResolvedValue(makeAuthDataResponse())

    const result = await loginFlow('realm1', baseCredentials())

    expect(result.redirectTo).toBeUndefined()
    expect(result.response.redirectTo).toBeNull()
    expect(storeMock.login).toHaveBeenCalledWith('realm1')
    expect(fetchAuthData).toHaveBeenCalledWith('realm1')
  })

  it('calls store.logout when performLogin throws', async () => {
    const storeMock = makeStoreMock()
    vi.mocked(useAuthStore.getState).mockReturnValue(
      storeMock as ReturnType<typeof useAuthStore.getState>
    )

    vi.mocked(performLogin).mockRejectedValue(new Error('Network error'))

    await expect(loginFlow('realm1', baseCredentials())).rejects.toThrow('Network error')
    expect(storeMock.logout).toHaveBeenCalled()
  })
})

describe('completeLoginAfterTotp redirectTo logic', () => {
  it('when redirectTo present, returns it without calling fetchAuthData', async () => {
    const storeMock = makeStoreMock()
    vi.mocked(useAuthStore.getState).mockReturnValue(
      storeMock as ReturnType<typeof useAuthStore.getState>
    )

    const verifyResponse = makeVerifyTotpResponse({
      redirectTo: 'https://app.example.com/callback?code=abc',
    })

    const result = await completeLoginAfterTotp('realm1', verifyResponse)

    expect(result).toEqual({
      redirectPath: undefined,
      redirectTo: 'https://app.example.com/callback?code=abc',
    })
    expect(fetchAuthData).not.toHaveBeenCalled()
  })

  it('when redirectTo absent, fetches auth data and returns redirectPath', async () => {
    const storeMock = makeStoreMock()
    vi.mocked(useAuthStore.getState).mockReturnValue(
      storeMock as ReturnType<typeof useAuthStore.getState>
    )

    vi.mocked(fetchAuthData).mockResolvedValue(makeAuthDataResponse())

    const verifyResponse = makeVerifyTotpResponse({ redirectTo: undefined })

    const result = await completeLoginAfterTotp('realm1', verifyResponse)

    expect(result.redirectPath).toBe('/user/profile')
    expect(result.redirectTo).toBeUndefined()
    expect(fetchAuthData).toHaveBeenCalledWith('realm1')
    expect(storeMock.setAuthStatus).toHaveBeenCalled()
    expect(storeMock.setUserPermissions).toHaveBeenCalled()
    expect(storeMock.setUserProfile).toHaveBeenCalled()
  })

  it('when redirectTo is null, same as absent — runs existing logic', async () => {
    const storeMock = makeStoreMock()
    vi.mocked(useAuthStore.getState).mockReturnValue(
      storeMock as ReturnType<typeof useAuthStore.getState>
    )

    vi.mocked(fetchAuthData).mockResolvedValue(makeAuthDataResponse())

    const verifyResponse = makeVerifyTotpResponse({ redirectTo: null })

    const result = await completeLoginAfterTotp('realm1', verifyResponse)

    expect(result.redirectTo).toBeUndefined()
    expect(fetchAuthData).toHaveBeenCalledWith('realm1')
  })

  it('calls store.logout when fetchAuthData throws', async () => {
    const storeMock = makeStoreMock()
    vi.mocked(useAuthStore.getState).mockReturnValue(
      storeMock as ReturnType<typeof useAuthStore.getState>
    )

    vi.mocked(fetchAuthData).mockRejectedValue(new Error('Session expired'))

    const verifyResponse = makeVerifyTotpResponse({ redirectTo: undefined })

    await expect(completeLoginAfterTotp('realm1', verifyResponse)).rejects.toThrow(
      'Session expired'
    )
    expect(storeMock.logout).toHaveBeenCalled()
  })
})

describe('validateOAuthParams completeness', () => {
  it.each([
    {
      name: 'all 3 present — complete',
      params: { oauthClientId: 'c1', redirectUri: 'https://app.example.com/cb', state: 's1' },
      expectedOAuth: {
        oauthClientId: 'c1',
        redirectUri: 'https://app.example.com/cb',
        state: 's1',
      },
      expectedPartial: false,
    },
    {
      name: 'only oauthClientId — partial',
      params: { oauthClientId: 'c1' },
      expectedOAuth: null,
      expectedPartial: true,
    },
    {
      name: 'only redirectUri — partial',
      params: { redirectUri: 'https://app.example.com/cb' },
      expectedOAuth: null,
      expectedPartial: true,
    },
    {
      name: 'only state — partial',
      params: { state: 's1' },
      expectedOAuth: null,
      expectedPartial: true,
    },
    {
      name: 'oauthClientId + redirectUri — partial',
      params: { oauthClientId: 'c1', redirectUri: 'https://app.example.com/cb' },
      expectedOAuth: null,
      expectedPartial: true,
    },
    {
      name: 'oauthClientId + state — partial',
      params: { oauthClientId: 'c1', state: 's1' },
      expectedOAuth: null,
      expectedPartial: true,
    },
    {
      name: 'redirectUri + state — partial',
      params: { redirectUri: 'https://app.example.com/cb', state: 's1' },
      expectedOAuth: null,
      expectedPartial: true,
    },
    {
      name: 'none present — not partial',
      params: {},
      expectedOAuth: null,
      expectedPartial: false,
    },
  ] as const)('$name', ({ params, expectedOAuth, expectedPartial }) => {
    const { oauthParams, hasPartialOAuth } = validateOAuthParams(params)
    expect(oauthParams).toEqual(expectedOAuth)
    expect(hasPartialOAuth).toBe(expectedPartial)
  })
})

describe('loginFlow passes OAuth fields to performLogin', () => {
  it('forwards oauthClientId, redirectUri, state in the payload', async () => {
    vi.mocked(performLogin).mockResolvedValue(
      makeLoginResponse({ redirectTo: 'https://app.example.com/callback?code=abc' })
    )

    const creds: LoginRequestPayload = {
      ...baseCredentials(),
      oauthClientId: 'oauth-client-42',
      redirectUri: 'https://app.example.com/callback',
      state: 'csrf-state-token',
    }

    await loginFlow('realm1', creds)

    expect(performLogin).toHaveBeenCalledWith(
      'realm1',
      expect.objectContaining({
        clientId: 'client-1',
        oauthClientId: 'oauth-client-42',
        redirectUri: 'https://app.example.com/callback',
        state: 'csrf-state-token',
      })
    )
  })
})
