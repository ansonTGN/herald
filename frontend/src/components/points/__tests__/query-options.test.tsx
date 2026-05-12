import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  pointsAccountsQueryOptions,
  pointsAccountQueryOptions,
  pointsTransactionsQueryOptions,
  pointsPlanConfigsQueryOptions,
} from '@/data/query-options'
import { listAccounts } from '@/lib/api-generated'

vi.mock('@/lib/api-generated', async () => {
  const actual = await vi.importActual<typeof import('@/lib/api-generated')>('@/lib/api-generated')
  return {
    ...actual,
    listAccounts: vi.fn(),
  }
})

const mockedListAccounts = vi.mocked(listAccounts)

describe('pointsAccountsQueryOptions', () => {
  beforeEach(() => {
    mockedListAccounts.mockReset()
  })

  describe('Query Configuration', () => {
    it('should create correct query key for accounts list', () => {
      const filters = { page: 1, page_size: 20 }
      const options = pointsAccountsQueryOptions('realm-1', filters)

      expect(options.queryKey).toEqual(['points-accounts', 'realm-1', filters])
    })

    it('should include realmId in query key', () => {
      const options = pointsAccountsQueryOptions('test-realm', {})

      expect(options.queryKey).toContain('test-realm')
    })

    it('should include filters in query key', () => {
      const filters = {
        page: 1,
        page_size: 20,
        search: 'test@example.com',
        status: 'active',
      }
      const options = pointsAccountsQueryOptions('realm-1', filters)

      expect(options.queryKey).toContain(filters)
    })

    it('should configure retry count', () => {
      const options = pointsAccountsQueryOptions('realm-1', {})

      expect(options.retry).toBe(1)
    })

    it('should configure stale time', () => {
      const options = pointsAccountsQueryOptions('realm-1', {})

      expect(options.staleTime).toBe(2 * 60 * 1000) // 2 minutes
    })

    it('should have a query function', () => {
      const options = pointsAccountsQueryOptions('realm-1', {})

      expect(options.queryFn).toBeDefined()
      expect(typeof options.queryFn).toBe('function')
    })

    it('should preserve user name and email from API response', async () => {
      mockedListAccounts.mockResolvedValue({
        data: {
          total: 1,
          page: 1,
          pageSize: 20,
          data: [
            {
              id: 'acc-1',
              userId: 'user-1',
              userName: 'Alice',
              userEmail: 'alice@example.com',
              realmId: 'realm-1',
              balance: 500,
              totalRecharged: 1000,
              totalConsumed: 500,
              status: 'active',
              createdAt: '2025-01-01T00:00:00Z',
              updatedAt: '2025-01-02T00:00:00Z',
              currency: 'USD',
            },
          ],
        },
      } as Awaited<ReturnType<typeof listAccounts>>)

      const options = pointsAccountsQueryOptions('realm-1', {})
      const result = await options.queryFn?.()

      expect(result?.accounts[0]).toMatchObject({
        userName: 'Alice',
        userEmail: 'alice@example.com',
      })
    })
  })

  describe('Cache Key Generation', () => {
    it('should create unique cache keys for different filter combinations', () => {
      const options1 = pointsAccountsQueryOptions('realm-1', { status: 'active' })
      const options2 = pointsAccountsQueryOptions('realm-1', { status: 'inactive' })

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should create unique cache keys for different pages', () => {
      const options1 = pointsAccountsQueryOptions('realm-1', { page: 1, page_size: 20 })
      const options2 = pointsAccountsQueryOptions('realm-1', { page: 2, page_size: 20 })

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should create unique cache keys for different realms', () => {
      const options1 = pointsAccountsQueryOptions('realm-1', {})
      const options2 = pointsAccountsQueryOptions('realm-2', {})

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should create unique cache keys for different search terms', () => {
      const options1 = pointsAccountsQueryOptions('realm-1', { search: 'test1@example.com' })
      const options2 = pointsAccountsQueryOptions('realm-1', { search: 'test2@example.com' })

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })
  })

  describe('Filter Parameter Handling', () => {
    it('should handle empty filters', () => {
      const filters = {}
      const options = pointsAccountsQueryOptions('realm-1', filters)

      expect(options.queryKey).toContain(filters)
    })

    it('should handle partial filters', () => {
      const filters = {
        page: 1,
        page_size: 20,
        // search and status are undefined
      }
      const options = pointsAccountsQueryOptions('realm-1', filters)

      expect(options.queryKey).toContain(filters)
    })

    it('should handle complete filters', () => {
      const filters = {
        page: 2,
        page_size: 50,
        search: 'test@example.com',
        status: 'active',
      }
      const options = pointsAccountsQueryOptions('realm-1', filters)

      expect(options.queryKey).toContain(filters)
    })

    it('should handle optional filter parameters', () => {
      const filters = { page: 1 }
      const options = pointsAccountsQueryOptions('realm-1', filters)

      expect(options.queryKey[2]).toEqual({ page: 1 })
    })
  })
})

describe('pointsAccountQueryOptions', () => {
  describe('Query Configuration', () => {
    it('should create correct query key for single account', () => {
      const options = pointsAccountQueryOptions('realm-1', 'user-1')

      expect(options.queryKey).toEqual(['points-account', 'realm-1', 'user-1'])
    })

    it('should include realmId and userId in query key', () => {
      const options = pointsAccountQueryOptions('test-realm', 'test-user')

      expect(options.queryKey).toContain('test-realm')
      expect(options.queryKey).toContain('test-user')
    })

    it('should configure retry count', () => {
      const options = pointsAccountQueryOptions('realm-1', 'user-1')

      expect(options.retry).toBe(1)
    })

    it('should configure stale time', () => {
      const options = pointsAccountQueryOptions('realm-1', 'user-1')

      expect(options.staleTime).toBe(2 * 60 * 1000) // 2 minutes
    })

    it('should have a query function', () => {
      const options = pointsAccountQueryOptions('realm-1', 'user-1')

      expect(options.queryFn).toBeDefined()
      expect(typeof options.queryFn).toBe('function')
    })
  })

  describe('Cache Key Generation', () => {
    it('should create unique cache keys for different users', () => {
      const options1 = pointsAccountQueryOptions('realm-1', 'user-1')
      const options2 = pointsAccountQueryOptions('realm-1', 'user-2')

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should create unique cache keys for different realms', () => {
      const options1 = pointsAccountQueryOptions('realm-1', 'user-1')
      const options2 = pointsAccountQueryOptions('realm-2', 'user-1')

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })
  })
})

describe('pointsTransactionsQueryOptions', () => {
  describe('Query Configuration', () => {
    it('should create correct query key for transactions list', () => {
      const filters = { page: 1, page_size: 20 }
      const options = pointsTransactionsQueryOptions('realm-1', filters)

      expect(options.queryKey).toEqual(['points-transactions', 'realm-1', filters])
    })

    it('should include realmId in query key', () => {
      const options = pointsTransactionsQueryOptions('test-realm', {})

      expect(options.queryKey).toContain('test-realm')
    })

    it('should include filters in query key', () => {
      const filters = {
        user_id: 'user-1',
        client_app_id: 'app-1',
        subscription_id: 'sub-1',
        transaction_type: 'recharge',
        start_time: '2025-01-01T00:00:00Z',
        end_time: '2025-01-31T23:59:59Z',
        page: 1,
        page_size: 20,
      }
      const options = pointsTransactionsQueryOptions('realm-1', filters)

      expect(options.queryKey).toContain(filters)
    })

    it('should configure retry count', () => {
      const options = pointsTransactionsQueryOptions('realm-1', {})

      expect(options.retry).toBe(1)
    })

    it('should configure stale time', () => {
      const options = pointsTransactionsQueryOptions('realm-1', {})

      expect(options.staleTime).toBe(2 * 60 * 1000) // 2 minutes
    })

    it('should have a query function', () => {
      const options = pointsTransactionsQueryOptions('realm-1', {})

      expect(options.queryFn).toBeDefined()
      expect(typeof options.queryFn).toBe('function')
    })
  })

  describe('Cache Key Generation', () => {
    it('should create unique cache keys for different filter combinations', () => {
      const options1 = pointsTransactionsQueryOptions('realm-1', { user_id: 'user-1' })
      const options2 = pointsTransactionsQueryOptions('realm-1', { user_id: 'user-2' })

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should create unique cache keys for different pages', () => {
      const options1 = pointsTransactionsQueryOptions('realm-1', { page: 1, page_size: 20 })
      const options2 = pointsTransactionsQueryOptions('realm-1', { page: 2, page_size: 20 })

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should create unique cache keys for different transaction types', () => {
      const options1 = pointsTransactionsQueryOptions('realm-1', { transaction_type: 'recharge' })
      const options2 = pointsTransactionsQueryOptions('realm-1', { transaction_type: 'consume' })

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should create unique cache keys for different users', () => {
      const options1 = pointsTransactionsQueryOptions('realm-1', { user_id: 'user-1' })
      const options2 = pointsTransactionsQueryOptions('realm-1', { user_id: 'user-2' })

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should create unique cache keys for different client apps', () => {
      const options1 = pointsTransactionsQueryOptions('realm-1', { client_app_id: 'app-1' })
      const options2 = pointsTransactionsQueryOptions('realm-1', { client_app_id: 'app-2' })

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })
  })

  describe('Filter Parameter Handling', () => {
    it('should handle empty filters', () => {
      const filters = {}
      const options = pointsTransactionsQueryOptions('realm-1', filters)

      expect(options.queryKey).toContain(filters)
    })

    it('should handle partial filters', () => {
      const filters = {
        user_id: 'user-1',
        page: 1,
        page_size: 20,
        // other fields are undefined
      }
      const options = pointsTransactionsQueryOptions('realm-1', filters)

      expect(options.queryKey).toContain(filters)
    })

    it('should handle complete filters', () => {
      const filters = {
        user_id: 'user-1',
        client_app_id: 'app-1',
        subscription_id: 'sub-1',
        transaction_type: 'recharge',
        start_time: '2025-01-01T00:00:00Z',
        end_time: '2025-01-31T23:59:59Z',
        page: 2,
        page_size: 50,
      }
      const options = pointsTransactionsQueryOptions('realm-1', filters)

      expect(options.queryKey).toContain(filters)
    })

    it('should handle time range filters', () => {
      const filters = {
        start_time: '2025-01-01T00:00:00Z',
        end_time: '2025-01-31T23:59:59Z',
      }
      const options = pointsTransactionsQueryOptions('realm-1', filters)

      expect(options.queryKey).toContain(filters)
    })
  })
})

describe('pointsPlanConfigsQueryOptions', () => {
  describe('Query Configuration', () => {
    it('should create correct query key for plan configs', () => {
      const options = pointsPlanConfigsQueryOptions('realm-1')

      expect(options.queryKey).toEqual(['points-plan-configs', 'realm-1'])
    })

    it('should include realmId in query key', () => {
      const options = pointsPlanConfigsQueryOptions('test-realm')

      expect(options.queryKey).toContain('test-realm')
    })

    it('should configure retry count', () => {
      const options = pointsPlanConfigsQueryOptions('realm-1')

      expect(options.retry).toBe(1)
    })

    it('should configure stale time', () => {
      const options = pointsPlanConfigsQueryOptions('realm-1')

      expect(options.staleTime).toBe(2 * 60 * 1000) // 2 minutes
    })

    it('should have a query function', () => {
      const options = pointsPlanConfigsQueryOptions('realm-1')

      expect(options.queryFn).toBeDefined()
      expect(typeof options.queryFn).toBe('function')
    })
  })

  describe('Cache Key Generation', () => {
    it('should create unique cache keys for different realms', () => {
      const options1 = pointsPlanConfigsQueryOptions('realm-1')
      const options2 = pointsPlanConfigsQueryOptions('realm-2')

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should create same cache key for same realm', () => {
      const options1 = pointsPlanConfigsQueryOptions('realm-1')
      const options2 = pointsPlanConfigsQueryOptions('realm-1')

      expect(options1.queryKey).toEqual(options2.queryKey)
    })
  })
})

describe('Query Key Structure', () => {
  it('should follow consistent naming pattern for accounts', () => {
    const filters = { page: 1, page_size: 20 }
    const options = pointsAccountsQueryOptions('realm-1', filters)

    expect(options.queryKey[0]).toBe('points-accounts')
    expect(options.queryKey[1]).toBe('realm-1')
    expect(options.queryKey[2]).toBe(filters)
  })

  it('should follow consistent naming pattern for single account', () => {
    const options = pointsAccountQueryOptions('realm-1', 'user-1')

    expect(options.queryKey[0]).toBe('points-account')
    expect(options.queryKey[1]).toBe('realm-1')
    expect(options.queryKey[2]).toBe('user-1')
  })

  it('should follow consistent naming pattern for transactions', () => {
    const filters = { page: 1, page_size: 20 }
    const options = pointsTransactionsQueryOptions('realm-1', filters)

    expect(options.queryKey[0]).toBe('points-transactions')
    expect(options.queryKey[1]).toBe('realm-1')
    expect(options.queryKey[2]).toBe(filters)
  })

  it('should follow consistent naming pattern for plan configs', () => {
    const options = pointsPlanConfigsQueryOptions('realm-1')

    expect(options.queryKey[0]).toBe('points-plan-configs')
    expect(options.queryKey[1]).toBe('realm-1')
  })

  it('should maintain correct order of parameters in query keys', () => {
    const filters = { page: 1, page_size: 20 }
    const accountsOptions = pointsAccountsQueryOptions('realm-1', filters)
    const accountOptions = pointsAccountQueryOptions('realm-1', 'user-1')
    const transactionsOptions = pointsTransactionsQueryOptions('realm-1', filters)
    const configsOptions = pointsPlanConfigsQueryOptions('realm-1')

    expect(accountsOptions.queryKey).toHaveLength(3)
    expect(accountOptions.queryKey).toHaveLength(3)
    expect(transactionsOptions.queryKey).toHaveLength(3)
    expect(configsOptions.queryKey).toHaveLength(2)
  })
})
