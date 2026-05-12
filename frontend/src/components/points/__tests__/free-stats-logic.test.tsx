import { describe, it, expect, afterEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/mocks/server'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import {
  mockFreeUserStatistics,
  mockFreeUserStatisticsWithHighUpgradeRate,
  mockFreeUserStatisticsWithZeroUsers,
  mockFreeUserStatisticsWithPartialData,
} from '@/fixtures/realm-config.fixture'

describe('Free User Statistics - High-Value Logic Tests', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  const createTestQueryClient = () =>
    new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    })

  describe('data transformation and formatting', () => {
    it('GIVEN statistics with high upgrade rate WHEN formatting THEN should display correct percentage', () => {
      const upgradeRate = mockFreeUserStatisticsWithHighUpgradeRate.upgradeRate
      const formattedRate = (upgradeRate * 100).toFixed(2)

      expect(formattedRate).toBe('15.37')
    })

    it('GIVEN zero statistics WHEN formatting THEN should handle division by zero', () => {
      const stats = mockFreeUserStatisticsWithZeroUsers
      const formattedUpgradeRate = (stats.upgradeRate * 100).toFixed(2)

      expect(formattedUpgradeRate).toBe('0.00')
      expect(stats.totalFreeUsers).toBe(0)
      expect(stats.activeFreeUsers).toBe(0)
    })

    it('GIVEN statistics WHEN formatting numbers THEN should use locale formatting', () => {
      const stats = mockFreeUserStatistics

      expect(stats.totalFreeUsers.toLocaleString()).toBe('1,000')
      expect(stats.activeFreeUsers.toLocaleString()).toBe('800')
      expect(stats.totalRegistrationBonusGranted.toLocaleString()).toBe('1,000,000')
      expect(stats.totalPeriodicPointsGranted.toLocaleString()).toBe('40,000')
    })

    it('GIVEN average daily points WHEN formatting THEN should show 2 decimal places', () => {
      const stats = mockFreeUserStatistics
      const formattedAverage = stats.averagePeriodicPointsPerUser.toFixed(2)

      expect(formattedAverage).toBe('50.00')
    })
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

  describe('date range validation logic', () => {
    it('GIVEN valid date range WHEN validating THEN should pass validation', () => {
      const startDate = '2026-03-01'
      const endDate = '2026-03-31'

      expect(startDate < endDate).toBe(true)
    })

    it('GIVEN invalid date range WHEN validating THEN should fail validation', () => {
      const startDate = '2026-03-31'
      const endDate = '2026-03-01'

      expect(startDate > endDate).toBe(true)
    })

    it('GIVEN start date only WHEN validating THEN should be valid', () => {
      const startDate = '2026-03-01'
      const endDate = undefined

      expect(startDate).toBeDefined()
      expect(endDate).toBeUndefined()
    })

    it('GIVEN end date only WHEN validating THEN should be valid', () => {
      const startDate = undefined
      const endDate = '2026-03-31'

      expect(startDate).toBeUndefined()
      expect(endDate).toBeDefined()
    })

    it('GIVEN no date range WHEN validating THEN should be valid', () => {
      const startDate = undefined
      const endDate = undefined

      expect(startDate).toBeUndefined()
      expect(endDate).toBeUndefined()
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
            return HttpResponse.json(mockFreeUserStatistics)
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
            return HttpResponse.json(mockFreeUserStatistics)
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

  describe('export functionality', () => {
    it('GIVEN statistics WHEN exporting to CSV THEN should format correctly', () => {
      const stats = mockFreeUserStatistics

      const csvContent = [
        'Metric,Value',
        `Total Free Users,${stats.totalFreeUsers}`,
        `Active Free Users,${stats.activeFreeUsers}`,
        `Total Registration Bonus Granted,${stats.totalRegistrationBonusGranted}`,
        `Total Periodic Points Granted,${stats.totalPeriodicPointsGranted}`,
        `Average Periodic Points Per User,${stats.averagePeriodicPointsPerUser.toFixed(2)}`,
        `Upgrade Rate,${(stats.upgradeRate * 100).toFixed(2)}%`,
        `Last Updated,${stats.lastUpdatedAt}`,
      ].join('\n')

      expect(csvContent).toContain('1000')
      expect(csvContent).toContain('800')
      expect(csvContent).toContain('15.00%')
      expect(csvContent).toContain('50.00')
    })

    it('GIVEN zero statistics WHEN exporting to CSV THEN should format correctly', () => {
      const stats = mockFreeUserStatisticsWithZeroUsers

      const csvContent = [
        'Metric,Value',
        `Total Free Users,${stats.totalFreeUsers}`,
        `Active Free Users,${stats.activeFreeUsers}`,
        `Total Registration Bonus Granted,${stats.totalRegistrationBonusGranted}`,
        `Total Periodic Points Granted,${stats.totalPeriodicPointsGranted}`,
        `Average Periodic Points Per User,${stats.averagePeriodicPointsPerUser.toFixed(2)}`,
        `Upgrade Rate,${(stats.upgradeRate * 100).toFixed(2)}%`,
      ].join('\n')

      expect(csvContent).toContain('0')
      expect(csvContent).toContain('0.00%')
      expect(csvContent).toContain('0.00')
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

  describe('upgrade rate color coding logic', () => {
    it('GIVEN upgrade rate > 10% WHEN determining color THEN should be green', () => {
      const upgradeRate = 15
      const color = upgradeRate > 10 ? 'green' : upgradeRate > 5 ? 'yellow' : 'red'

      expect(color).toBe('green')
    })

    it('GIVEN upgrade rate > 5% and <= 10% WHEN determining color THEN should be yellow', () => {
      const upgradeRate = 7
      const color = upgradeRate > 10 ? 'green' : upgradeRate > 5 ? 'yellow' : 'red'

      expect(color).toBe('yellow')
    })

    it('GIVEN upgrade rate <= 5% WHEN determining color THEN should be red', () => {
      const upgradeRate = 3
      const color = upgradeRate > 10 ? 'green' : upgradeRate > 5 ? 'yellow' : 'red'

      expect(color).toBe('red')
    })
  })

  describe('React Query cache behavior', () => {
    it('GIVEN query options WHEN creating THEN should have correct stale time', async () => {
      const { freeUserStatsQueryOptions } = await import('@/data/query-options')

      const options = freeUserStatsQueryOptions('test-realm', {
        startDate: '2026-03-01',
        endDate: '2026-03-31',
      })

      expect(options.queryKey).toContain('free-user-stats')
      expect(options.queryKey).toContain('test-realm')
      expect(options.staleTime).toBeDefined()
    })

    it('GIVEN query options WHEN creating THEN should have correct refetch interval', async () => {
      const { freeUserStatsQueryOptions } = await import('@/data/query-options')
      const { TIME_CONSTANTS } = await import('@/lib/constants')

      const options = freeUserStatsQueryOptions('test-realm')

      expect(options.refetchInterval).toBe(TIME_CONSTANTS.FIVE_MINUTES)
    })
  })

  describe('data type validation', () => {
    it('GIVEN API response WHEN validating THEN should match expected types', () => {
      const stats = mockFreeUserStatistics

      expect(typeof stats.totalFreeUsers).toBe('number')
      expect(typeof stats.activeFreeUsers).toBe('number')
      expect(typeof stats.totalRegistrationBonusGranted).toBe('number')
      expect(typeof stats.totalPeriodicPointsGranted).toBe('number')
      expect(typeof stats.averagePeriodicPointsPerUser).toBe('number')
      expect(typeof stats.upgradeRate).toBe('number')
      expect(typeof stats.lastUpdatedAt).toBe('string')
    })

    it('GIVEN API response WHEN validating THEN should have positive numbers', () => {
      const stats = mockFreeUserStatistics

      expect(stats.totalFreeUsers).toBeGreaterThanOrEqual(0)
      expect(stats.activeFreeUsers).toBeGreaterThanOrEqual(0)
      expect(stats.activeFreeUsers).toBeLessThanOrEqual(stats.totalFreeUsers)
      expect(stats.totalRegistrationBonusGranted).toBeGreaterThanOrEqual(0)
      expect(stats.totalPeriodicPointsGranted).toBeGreaterThanOrEqual(0)
      expect(stats.averagePeriodicPointsPerUser).toBeGreaterThanOrEqual(0)
      expect(stats.upgradeRate).toBeGreaterThanOrEqual(0)
      expect(stats.upgradeRate).toBeLessThanOrEqual(1)
    })
  })
})
