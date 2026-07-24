import { describe, expect, it, vi } from 'vitest'
import { resolveCustomDomain } from '@/lib/api-generated'
import { resolveRealmContext } from '@/lib/realm-routing'

vi.mock('@/lib/api-generated', () => ({
  resolveCustomDomain: vi.fn(),
}))

describe('resolveRealmContext', () => {
  it('should resolve custom domains through the generated SDK', async () => {
    vi.mocked(resolveCustomDomain).mockResolvedValue({
      data: {
        realmId: 'realm-1',
        publicConfig: {},
      },
      error: undefined,
    } as never)

    const context = await resolveRealmContext('/auth/login')

    expect(resolveCustomDomain).toHaveBeenCalledWith({
      query: { host: window.location.host },
    })
    expect(context).toMatchObject({
      realmId: 'realm-1',
      isCustomDomain: true,
    })
  })
})
