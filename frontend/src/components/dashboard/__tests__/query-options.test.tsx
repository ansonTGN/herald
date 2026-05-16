import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { http, HttpResponse } from 'msw'
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import { server } from '@/test/mocks/server'
import { makeDashboardStats, getDashboardStatsHandler } from '@/test/mocks/handlers/dashboard'
import { dashboardStatsQueryOptions, queryKeys } from '@/data/query-options'
import { QUERY_KEYS } from '@/lib/constants'
import { getDashboardStats } from '@/lib/api-generated'

const API_BASE_URL = 'http://localhost:3000'

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  })
}

function renderWithQueryClient(ui: React.ReactNode) {
  const queryClient = createTestQueryClient()
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>)
}

// Minimal component to exercise query options through React Query
function DashboardStatsTestComponent({ realmId }: { realmId: string }) {
  const { data, isLoading, error } = useQuery({
    ...dashboardStatsQueryOptions(realmId),
    retry: false,
  })

  if (isLoading) return <div data-testid="loading">Loading...</div>
  if (error) return <div data-testid="dashboard-error">{error.message}</div>
  if (data)
    return (
      <div data-testid="dashboard-data">
        {data.userStats.totalUsers} users
      </div>
    )
  return null
}

describe('dashboardStatsQueryOptions', () => {
  // ==================== Query Key Isolation ====================

  describe('query key isolation', () => {
    it('should produce a query key containing the dashboard key constant', () => {
      const options = dashboardStatsQueryOptions('realm-1')
      expect(options.queryKey).toEqual([QUERY_KEYS.DASHBOARD_STATS, 'realm-1'])
    })

    it('should produce different query keys for different realmIds', () => {
      const options1 = dashboardStatsQueryOptions('realm-1')
      const options2 = dashboardStatsQueryOptions('realm-2')

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })

    it('should use queryKeys.dashboardStats helper consistently', () => {
      const realmId = 'test-realm'
      const options = dashboardStatsQueryOptions(realmId)

      expect(options.queryKey).toEqual(queryKeys.dashboardStats(realmId))
    })
  })

  // ==================== MSW Contract Tests ====================

  describe('MSW contract: GET /api/dashboard/{realmId}/stats', () => {
    beforeEach(() => {
      server.resetHandlers()
    })

    it('should return response matching DashboardStatsResponse shape', async () => {
      const mockData = makeDashboardStats()
      let capturedRequest: Request | undefined

      server.use(
        http.get(`${API_BASE_URL}/api/dashboard/:realmId/stats`, async ({ request }) => {
          capturedRequest = request as Request
          return HttpResponse.json(mockData)
        })
      )

      const response = await getDashboardStats({ path: { realmId: 'test-realm' } })

      // Verify the request URL contains the realmId
      expect(capturedRequest).toBeDefined()
      const requestUrl = new URL(capturedRequest!.url)
      expect(requestUrl.pathname).toBe('/api/dashboard/test-realm/stats')

      // Verify response body shape
      expect(response.data).toEqual(mockData)
      expect(response.data!.userStats.totalUsers).toBe(100)
      expect(response.data!.userStats.newUsers).toBe(5)
      expect(response.data!.userStats.activeUsers).toBe(20)

      // Verify authTrend array items
      expect(response.data!.authTrend).toHaveLength(2)
      const firstPoint = response.data!.authTrend[0]
      expect(firstPoint).toHaveProperty('date')
      expect(firstPoint).toHaveProperty('successCount')
      expect(firstPoint).toHaveProperty('failureCount')
    })

    it('should use default handler from makeDashboardStats factory', async () => {
      server.use(getDashboardStatsHandler)

      const response = await getDashboardStats({ path: { realmId: 'any-realm' } })

      expect(response.data).toBeDefined()
      expect(response.data!.userStats).toBeDefined()
      expect(Array.isArray(response.data!.authTrend)).toBe(true)
    })
  })

  // ==================== Error State Tests ====================

  describe('error states', () => {
    beforeEach(() => {
      server.resetHandlers()
    })

    it('should enter error state on 403 Forbidden', async () => {
      server.use(
        http.get(`${API_BASE_URL}/api/dashboard/:realmId/stats`, () => {
          return HttpResponse.json({ message: 'Forbidden' }, { status: 403 })
        })
      )

      renderWithQueryClient(<DashboardStatsTestComponent realmId="forbidden-realm" />)

      expect(screen.getByTestId('loading')).toBeInTheDocument()

      const errorElement = await screen.findByTestId('dashboard-error', undefined, {
        timeout: 5000,
      })
      expect(errorElement).toBeInTheDocument()
    })

    it('should enter error state on 500 Internal Server Error', async () => {
      server.use(
        http.get(`${API_BASE_URL}/api/dashboard/:realmId/stats`, () => {
          return HttpResponse.json(
            { message: 'Internal Server Error' },
            { status: 500 }
          )
        })
      )

      renderWithQueryClient(<DashboardStatsTestComponent realmId="test-realm" />)

      const errorElement = await screen.findByTestId('dashboard-error', undefined, {
        timeout: 5000,
      })
      expect(errorElement).toBeInTheDocument()
    })

    it('should enter error state on network error', async () => {
      server.use(
        http.get(`${API_BASE_URL}/api/dashboard/:realmId/stats`, () => {
          return HttpResponse.error()
        })
      )

      renderWithQueryClient(<DashboardStatsTestComponent realmId="test-realm" />)

      const errorElement = await screen.findByTestId('dashboard-error', undefined, {
        timeout: 5000,
      })
      expect(errorElement).toBeInTheDocument()
    })
  })
})
