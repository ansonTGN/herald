import { describe, it, expect, vi } from 'vitest'
import {
  subscriptionHistoryQueryOptions,
  globalSubscriptionHistoryQueryOptions,
  getGlobalSubscriptionHistory,
} from '@/data/query-options'
import type { HistoryFilters } from '@/types/billing'
import { QUERY_KEYS } from '@/lib/constants'

describe('subscriptionHistoryQueryOptions', () => {
  it('should create correct query key for subscription history', () => {
    const options = subscriptionHistoryQueryOptions('realm-1', 'sub-1')

    expect(options.queryKey).toEqual([QUERY_KEYS.SUBSCRIPTION_HISTORY, 'realm-1', 'sub-1'])
  })
})

describe('globalSubscriptionHistoryQueryOptions', () => {
  it('should create correct query key for global history', () => {
    const filters: HistoryFilters = {
      eventType: 'upgraded',
      userId: 'user-1',
    }

    const options = globalSubscriptionHistoryQueryOptions('realm-1', filters, 1, 20)

    expect(options.queryKey).toEqual([
      QUERY_KEYS.GLOBAL_SUBSCRIPTION_HISTORY,
      'realm-1',
      filters,
      1,
      20,
    ])
  })

  describe('Cache Key Generation', () => {
    it('should create unique cache keys for different filter combinations', () => {
      const options1 = globalSubscriptionHistoryQueryOptions('realm-1', { eventType: 'upgraded' })
      const options2 = globalSubscriptionHistoryQueryOptions('realm-1', { eventType: 'canceled' })

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should create unique cache keys for different pages', () => {
      const options1 = globalSubscriptionHistoryQueryOptions('realm-1', {}, 1)
      const options2 = globalSubscriptionHistoryQueryOptions('realm-1', {}, 2)

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should create unique cache keys for different realms', () => {
      const options1 = globalSubscriptionHistoryQueryOptions('realm-1', {})
      const options2 = globalSubscriptionHistoryQueryOptions('realm-2', {})

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should create unique cache keys for different page sizes', () => {
      const options1 = globalSubscriptionHistoryQueryOptions('realm-1', {}, 1, 20)
      const options2 = globalSubscriptionHistoryQueryOptions('realm-1', {}, 1, 50)

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })
  })
})

describe('Filter Parameter Handling', () => {
  it('should handle default pagination', () => {
    const options = globalSubscriptionHistoryQueryOptions('realm-1', {})

    expect(options.queryKey).toContain(1) // default page
    expect(options.queryKey).toContain(20) // default pageSize
  })

  it('should handle custom pagination', () => {
    const options = globalSubscriptionHistoryQueryOptions('realm-1', {}, 3, 100)

    expect(options.queryKey).toContain(3)
    expect(options.queryKey).toContain(100)
  })

  it('should send camelCase query params that match the backend contract', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          events: [],
          pagination: { page: 1, pageSize: 50, total: 0, totalPages: 0 },
        }),
        {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }
      )
    )
    vi.stubGlobal('fetch', fetchMock)

    await getGlobalSubscriptionHistory(
      'realm-1',
      {
        userId: 'user-1',
        entitlementKey: 'basic',
        eventType: 'upgraded',
        subscriptionStatus: 'active',
        fromDate: '2025-01-01T00:00:00.000Z',
        toDate: '2025-01-31T23:59:59.999Z',
        sortBy: 'timestamp',
        sortOrder: 'desc',
      },
      2,
      50
    )

    expect(fetchMock).toHaveBeenCalledTimes(1)
    const requestUrl = new URL(fetchMock.mock.calls[0][0], 'http://localhost')

    expect(requestUrl.searchParams.get('userId')).toBe('user-1')
    expect(requestUrl.searchParams.get('entitlementKey')).toBe('basic')
    expect(requestUrl.searchParams.get('eventType')).toBe('upgraded')
    expect(requestUrl.searchParams.get('subscriptionStatus')).toBe('active')
    expect(requestUrl.searchParams.get('fromDate')).toBe('2025-01-01T00:00:00.000Z')
    expect(requestUrl.searchParams.get('toDate')).toBe('2025-01-31T23:59:59.999Z')
    expect(requestUrl.searchParams.get('sortBy')).toBe('timestamp')
    expect(requestUrl.searchParams.get('sortOrder')).toBe('desc')
    expect(requestUrl.searchParams.get('page')).toBe('2')
    expect(requestUrl.searchParams.get('pageSize')).toBe('50')

    expect(requestUrl.searchParams.get('user_id')).toBeNull()
    expect(requestUrl.searchParams.get('event_type')).toBeNull()
    expect(requestUrl.searchParams.get('page_size')).toBeNull()
  })
})
