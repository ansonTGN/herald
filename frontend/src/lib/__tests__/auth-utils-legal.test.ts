import { describe, it, expect, vi, beforeEach } from 'vitest'
import { loginFlow, completeLoginAfterTotp } from '@/lib/auth-utils'
import type { LoginResponse, VerifyTotpResponse } from '@/lib/api-generated'

const mockLogin = vi.fn()
const mockLogout = vi.fn()
const mockSetAuthStatus = vi.fn()
const mockSetUserPermissions = vi.fn()
const mockSetUserProfile = vi.fn()
const mockReset = vi.fn()
const mockSetIsLoading = vi.fn()
const mockClearAuthStorage = vi.fn()

vi.mock('@/stores/auth-store', () => ({
  useAuthStore: {
    getState: () => ({
      login: mockLogin,
      logout: mockLogout,
      setAuthStatus: mockSetAuthStatus,
      setUserPermissions: mockSetUserPermissions,
      setUserProfile: mockSetUserProfile,
      reset: mockReset,
      setIsLoading: mockSetIsLoading,
    }),
  },
  clearAuthStorage: () => mockClearAuthStorage(),
}))

const mockPerformLogin = vi.fn()
const mockFetchAuthData = vi.fn()
const mockPerformLogout = vi.fn()

vi.mock('@/lib/auth-service', () => ({
  performLogin: (...args: unknown[]) => mockPerformLogin(...args),
  fetchAuthData: (...args: unknown[]) => mockFetchAuthData(...args),
  performLogout: (...args: unknown[]) => mockPerformLogout(...args),
}))

vi.mock('@/lib/constants/auth-constants', () => ({
  hasAdminPermission: () => false,
  DEFAULT_USER_REDIRECT: '/user/profile',
  DEFAULT_ADMIN_REDIRECT: '/manage',
  getSafeRedirectPath: (path: string | undefined, fallback: string) => path ?? fallback,
}))

function makeLoginResponse(overrides?: Partial<LoginResponse>): LoginResponse {
  return {
    userId: 'user-001',
    realmId: 'realm-1',
    message: 'OK',
    expiresInSeconds: 3600,
    ...overrides,
  }
}

function makeVerifyTotpResponse(overrides?: Partial<VerifyTotpResponse>): VerifyTotpResponse {
  return {
    userId: 'user-001',
    token: 'token-001',
    message: 'OK',
    expiresInSeconds: 3600,
    ...overrides,
  }
}

describe('loginFlow consent required handling', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns early when consentRequired is true and does not authenticate', async () => {
    const response = makeLoginResponse({
      consentRequired: true,
      agreements: [
        {
          agreement_type: 'terms_of_service',
          version_id: 'tos-v2',
          version_no: 2,
          effective_at: '2026-06-30T00:00:00Z',
          title: 'Terms',
          summary: null,
        },
      ],
    })
    mockPerformLogin.mockResolvedValue(response)

    const result = await loginFlow('realm-1', {
      clientId: 'client-1',
      password: 'secret',
    })

    expect(result.response.consentRequired).toBe(true)
    expect(mockLogin).not.toHaveBeenCalled()
    expect(mockFetchAuthData).not.toHaveBeenCalled()
  })

  it('returns early when consent_required snake_case field is true', async () => {
    const response = makeLoginResponse({
      consent_required: true,
    } as LoginResponse)
    mockPerformLogin.mockResolvedValue(response)

    await loginFlow('realm-1', { clientId: 'client-1', password: 'secret' })

    expect(mockLogin).not.toHaveBeenCalled()
    expect(mockFetchAuthData).not.toHaveBeenCalled()
  })

  it('authenticates normally when consentRequired is false', async () => {
    const response = makeLoginResponse({ consentRequired: false })
    mockPerformLogin.mockResolvedValue(response)
    mockFetchAuthData.mockResolvedValue({
      authStatus: { authenticated: true, realmId: 'realm-1' },
      userPermissions: { permissions: [], roles: [] },
      userProfile: null,
    })

    await loginFlow('realm-1', { clientId: 'client-1', password: 'secret' })

    expect(mockLogin).toHaveBeenCalledWith('realm-1')
    expect(mockFetchAuthData).toHaveBeenCalledWith()
  })
})

describe('completeLoginAfterTotp consent required handling', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns empty object when consentRequired is true and does not update auth state', async () => {
    const response = makeVerifyTotpResponse({
      consentRequired: true,
      agreements: [
        {
          agreement_type: 'privacy_policy',
          version_id: 'privacy-v3',
          version_no: 3,
          effective_at: '2026-06-30T00:00:00Z',
          title: 'Privacy',
          summary: null,
        },
      ],
    })

    const result = await completeLoginAfterTotp('realm-1', response)

    expect(result).toEqual({})
    expect(mockFetchAuthData).not.toHaveBeenCalled()
    expect(mockSetAuthStatus).not.toHaveBeenCalled()
  })

  it('returns early when consent_required snake_case field is true', async () => {
    const response = makeVerifyTotpResponse({
      consent_required: true,
    } as VerifyTotpResponse)

    await completeLoginAfterTotp('realm-1', response)

    expect(mockFetchAuthData).not.toHaveBeenCalled()
    expect(mockSetAuthStatus).not.toHaveBeenCalled()
  })

  it('completes login normally when consentRequired is absent', async () => {
    const response = makeVerifyTotpResponse()
    mockFetchAuthData.mockResolvedValue({
      authStatus: { authenticated: true, realmId: 'realm-1' },
      userPermissions: { permissions: [], roles: [] },
      userProfile: null,
    })

    await completeLoginAfterTotp('realm-1', response)

    expect(mockFetchAuthData).toHaveBeenCalledWith()
    expect(mockSetAuthStatus).toHaveBeenCalled()
  })
})
