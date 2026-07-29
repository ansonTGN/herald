import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/lib/api-generated', () => ({
  switchClient: vi.fn(),
}))

import { switchClient } from '@/lib/api-generated'
import { ClientSwitchError, switchFirstPartyClient } from '@/lib/auth-service'

describe('switchFirstPartyClient', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns the replacement token family bound to the target product', async () => {
    const tokenSet = {
      accessToken: 'access',
      refreshToken: 'refresh',
      expiresIn: 900,
      refreshExpiresIn: 3600,
      tokenType: 'Bearer',
      clientId: 'user-account-center',
    }
    vi.mocked(switchClient).mockResolvedValue({
      data: tokenSet,
      error: undefined,
      response: new Response(null, { status: 200 }),
      request: new Request('http://localhost/api/auth/browser-token/switch-client'),
    })

    await expect(switchFirstPartyClient('user-account-center')).resolves.toEqual(tokenSet)
    expect(switchClient).toHaveBeenCalledWith({
      body: { targetClientId: 'user-account-center' },
    })
  })

  it('preserves the HTTP status so an admin denial is distinct from a broken switch', async () => {
    vi.mocked(switchClient).mockResolvedValue({
      data: undefined,
      error: undefined,
      response: new Response(null, { status: 403 }),
      request: new Request('http://localhost/api/auth/browser-token/switch-client'),
    })

    const error = await switchFirstPartyClient('admin-web-console').catch((cause) => cause)
    expect(error).toBeInstanceOf(ClientSwitchError)
    expect(error.status).toBe(403)
  })
})
