import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  pointsAccountsQueryOptions,
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

  describe('Query Data Transformation', () => {
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

  describe('Cache Key Isolation', () => {
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

    it('should create same cache key for same realm', () => {
      const options1 = pointsPlanConfigsQueryOptions('realm-1')
      const options2 = pointsPlanConfigsQueryOptions('realm-1')

      expect(options1.queryKey).toEqual(options2.queryKey)
    })

    it('should create unique cache keys for different transaction types', () => {
      const options1 = pointsTransactionsQueryOptions('realm-1', { transaction_type: 'recharge' })
      const options2 = pointsTransactionsQueryOptions('realm-1', { transaction_type: 'consume' })

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })
  })
})
