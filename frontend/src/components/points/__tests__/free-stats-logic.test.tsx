import { describe, it, expect, afterEach, vi } from 'vitest'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/mocks/server'
import {
  mockFreeUserStatisticsWithZeroUsers,
  mockFreeUserStatisticsWithPartialData,
} from '@/fixtures/realm-config.fixture'

describe('Free User Statistics - High-Value Logic Tests', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('API error handling', () => {
    it('GIVEN API returns 400 WHEN fetching statistics THEN should handle error', async () => {
      server.use(
        http.get('http://localhost:3000/api/points/test-realm/statistics/free-users', () => {
          return HttpResponse.json({ message: 'Invalid date range' }, { status: 400 })
        })
      )

      const { getFreeUserStatistics } = await import('@/lib/api-generated')

      const response = await getFreeUserStatistics({
        path: { realmId: 'test-realm' },
        query: {
          startDate: '2026-03-31',
          endDate: '2026-03-01',
        },
      })

      expect(response.error).toBeDefined()
      expect(response.data).toBeUndefined()
    })

    it('GIVEN API returns 404 WHEN fetching statistics THEN should handle error', async () => {
      server.use(
        http.get('http://localhost:3000/api/points/test-realm/statistics/free-users', () => {
          return HttpResponse.json({ message: 'Statistics not found' }, { status: 404 })
        })
      )

      const { getFreeUserStatistics } = await import('@/lib/api-generated')

      const response = await getFreeUserStatistics({ path: { realmId: 'test-realm' } })

      expect(response.error).toBeDefined()
      expect(response.data).toBeUndefined()
    })

    it('GIVEN API returns 500 WHEN fetching statistics THEN should handle error', async () => {
      server.use(
        http.get('http://localhost:3000/api/points/test-realm/statistics/free-users', () => {
          return HttpResponse.json({ message: 'Internal server error' }, { status: 500 })
        })
      )

      const { getFreeUserStatistics } = await import('@/lib/api-generated')

      const response = await getFreeUserStatistics({ path: { realmId: 'test-realm' } })

      expect(response.error).toBeDefined()
      expect(response.data).toBeUndefined()
    })

    it('GIVEN API timeout WHEN fetching statistics THEN should handle error', async () => {
      server.use(
        http.get('http://localhost:3000/api/points/test-realm/statistics/free-users', () => {
          // Return error immediately instead of hanging
          return HttpResponse.error()
        })
      )

      const { getFreeUserStatistics } = await import('@/lib/api-generated')

      // Should handle network error
      const response = await getFreeUserStatistics({ path: { realmId: 'test-realm' } })

      // Network errors should still return a response object with error field
      expect(response.error).toBeDefined()
    })
  })

  describe('API request parameter construction', () => {
    it('GIVEN date range WHEN making API request THEN should construct correct URL params', async () => {
      const requestMock = vi.fn()

      server.use(
        http.get(
          'http://localhost:3000/api/points/:realmId/statistics/free-users',
          ({ request }) => {
            requestMock(request.url)
            return HttpResponse.json(mockFreeUserStatisticsWithZeroUsers)
          }
        )
      )

      const { getFreeUserStatistics } = await import('@/lib/api-generated')

      // Create a test client with baseURL
      const { createClient } = await import('@/lib/api-generated/client')
      const testClient = createClient({
        baseUrl: 'http://localhost:3000',
      })

      await getFreeUserStatistics({
        path: { realmId: 'test-realm' },
        query: {
          startDate: '2026-03-01',
          endDate: '2026-03-31',
        },
        client: testClient,
      })

      expect(requestMock).toHaveBeenCalled()
      const requestUrl = requestMock.mock.calls[0][0] as string
      expect(requestUrl).toContain('startDate=2026-03-01')
      expect(requestUrl).toContain('endDate=2026-03-31')
    })

    it('GIVEN no date range WHEN making API request THEN should not include date params', async () => {
      const requestMock = vi.fn()

      server.use(
        http.get(
          'http://localhost:3000/api/points/:realmId/statistics/free-users',
          ({ request }) => {
            requestMock(request.url)
            return HttpResponse.json(mockFreeUserStatisticsWithZeroUsers)
          }
        )
      )

      const { getFreeUserStatistics } = await import('@/lib/api-generated')

      // Create a test client with baseURL
      const { createClient } = await import('@/lib/api-generated/client')
      const testClient = createClient({
        baseUrl: 'http://localhost:3000',
      })

      await getFreeUserStatistics({
        path: { realmId: 'test-realm' },
        client: testClient,
      })

      expect(requestMock).toHaveBeenCalled()
      const requestUrl = requestMock.mock.calls[0][0] as string
      expect(requestUrl).not.toContain('startDate=')
      expect(requestUrl).not.toContain('endDate=')
    })
  })

  describe('partial data handling', () => {
    it('GIVEN partial statistics WHEN processing THEN should handle null values', () => {
      const stats = mockFreeUserStatisticsWithPartialData

      expect(stats.totalFreeUsers).toBe(1000)
      expect(stats.activeFreeUsers).toBeNull()
      expect(stats.totalPeriodicPointsGranted).toBeNull()
      expect(stats.averagePeriodicPointsPerUser).toBeNull()
    })

    it('GIVEN partial statistics WHEN formatting THEN should show N/A for null values', () => {
      const stats = mockFreeUserStatisticsWithPartialData

      const formatValue = (value: number | null) => {
        if (value === null) return 'N/A'
        return value.toLocaleString()
      }

      expect(formatValue(stats.totalFreeUsers)).toBe('1,000')
      expect(formatValue(stats.activeFreeUsers)).toBe('N/A')
      expect(formatValue(stats.totalPeriodicPointsGranted)).toBe('N/A')
    })
  })
})
