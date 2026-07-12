import { act, renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const mockOauthLogin = vi.fn()

vi.mock('@/lib/api-generated', () => ({
  oauthLogin: (...args: unknown[]) => mockOauthLogin(...args),
}))

vi.mock('sonner', () => ({
  toast: { error: vi.fn() },
}))

import { useOAuthLogin } from '../use-oauth-login'

describe('useOAuthLogin', () => {
  beforeEach(() => {
    mockOauthLogin.mockReset()
    mockOauthLogin.mockResolvedValue({ data: { authUrl: '#provider-login' } })
  })

  it('passes only the downstream state reference for a brokered authorization', async () => {
    // WHY: the downstream redirect URI and PKCE data remain authoritative in
    // the server-side /authorize transaction and must not become provider input.
    const { result } = renderHook(() => useOAuthLogin())

    await act(() => result.current.initiateOAuthLogin('realm-a', 'google', 'outer-state'))

    expect(mockOauthLogin).toHaveBeenCalledWith({
      path: { realmId: 'realm-a', provider: 'google' },
      query: { downstream_state: 'outer-state' },
    })
  })

  it('omits the downstream query for a direct Herald login', async () => {
    // WHY: direct SSO login must preserve its existing broker-only behavior.
    const { result } = renderHook(() => useOAuthLogin())

    await act(() => result.current.initiateOAuthLogin('realm-a', 'github'))

    expect(mockOauthLogin).toHaveBeenCalledWith({
      path: { realmId: 'realm-a', provider: 'github' },
      query: undefined,
    })
  })
})
