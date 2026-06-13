import { beforeEach, describe, expect, it, vi } from 'vitest'
import { pointsWalletsQueryOptions, pointsTransactionsQueryOptions } from '@/data/query-options'
import { listWallets } from '@/lib/api-generated'

vi.mock('@/lib/api-generated', async () => {
  const actual = await vi.importActual<typeof import('@/lib/api-generated')>('@/lib/api-generated')
  return {
    ...actual,
    listWallets: vi.fn(),
  }
})

const mockedListWallets = vi.mocked(listWallets)

describe('pointsWalletsQueryOptions', () => {
  beforeEach(() => {
    mockedListWallets.mockReset()
  })

  describe('Query Data Transformation', () => {
    it('should map items to wallets', async () => {
      mockedListWallets.mockResolvedValue({
        data: {
          total: 1,
          page: 1,
          pageSize: 20,
          items: [
            {
              id: 'acc-1',
              userId: 'user-1',
              realmId: 'realm-1',
              balance: 500,
              totalPaidGranted: 1000,
              totalRecharged: 1000,
              totalConsumed: 500,
              status: 'active',
              createdAt: '2025-01-01T00:00:00Z',
              updatedAt: '2025-01-02T00:00:00Z',
              unit: 'points',
              currency: 'points',
            },
          ],
        },
      } as Awaited<ReturnType<typeof listWallets>>)

      const options = pointsWalletsQueryOptions('realm-1', {})
      const result = await options.queryFn?.()

      expect(result?.wallets).toHaveLength(1)
      expect(result?.wallets[0].userId).toBe('user-1')
    })
  })

  describe('Cache Key Isolation', () => {
    it('should create unique cache keys for different filter combinations', () => {
      const options1 = pointsWalletsQueryOptions('realm-1', { status: 'active' })
      const options2 = pointsWalletsQueryOptions('realm-1', { status: 'inactive' })

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should create unique cache keys for different pages', () => {
      const options1 = pointsWalletsQueryOptions('realm-1', { page: 1, pageSize: 20 })
      const options2 = pointsWalletsQueryOptions('realm-1', { page: 2, pageSize: 20 })

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should create unique cache keys for different realms', () => {
      const options1 = pointsWalletsQueryOptions('realm-1', {})
      const options2 = pointsWalletsQueryOptions('realm-2', {})

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should create unique cache keys for different transaction types', () => {
      const options1 = pointsTransactionsQueryOptions('realm-1', { transactionType: 'recharge' })
      const options2 = pointsTransactionsQueryOptions('realm-1', { transactionType: 'consume' })

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })
  })
})
