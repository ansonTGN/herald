import { describe, it, expect, afterEach } from 'vitest'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/mocks/server'
import { getFreeUserStatistics } from '@/lib/api-generated'
import { mockFreeUserStatistics } from '@/fixtures/realm-config.fixture'

describe('Test 3.3: Free User Statistics API Contract Tests (P0)', () => {
  afterEach(() => {
    server.resetHandlers()
  })

  describe('GET /api/points/{realmId}/statistics/free-users', () => {
    it('GIVEN date range parameters WHEN calling statistics API THEN should include in request', async () => {
      server.use(
        http.get('/api/points/test-realm/statistics/free-users', ({ request }) => {
          const url = new URL(request.url)
          const startDate = url.searchParams.get('startDate')
          const endDate = url.searchParams.get('endDate')

          // Verify date parameters are passed correctly
          expect(startDate).toBe('2026-01-01')
          expect(endDate).toBe('2026-03-31')

          return HttpResponse.json(mockFreeUserStatistics)
        })
      )

      const response = await getFreeUserStatistics({
        path: { realmId: 'test-realm' },
        query: {
          startDate: '2026-01-01',
          endDate: '2026-03-31',
        },
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()
    })

    it('GIVEN no date range parameters WHEN calling statistics API THEN should return all-time statistics', async () => {
      server.use(
        http.get('/api/points/test-realm/statistics/free-users', ({ request }) => {
          const url = new URL(request.url)
          const startDate = url.searchParams.get('startDate')
          const endDate = url.searchParams.get('endDate')

          // Verify no date parameters are passed
          expect(startDate).toBeNull()
          expect(endDate).toBeNull()

          return HttpResponse.json(mockFreeUserStatistics)
        })
      )

      const response = await getFreeUserStatistics({
        path: { realmId: 'test-realm' },
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()
    })
  })

  describe('Statistics Field Values', () => {
    it('GIVEN zero users statistics WHEN parsing THEN should handle correctly', async () => {
      server.use(
        http.get('/api/points/test-realm/statistics/free-users', () => {
          return HttpResponse.json({
            totalFreeUsers: 0,
            activeFreeUsers: 0,
            totalRegistrationBonusGranted: 0,
            totalPeriodicPointsGranted: 0,
            averagePeriodicPointsPerUser: 0,
            upgradeRate: 0,
            lastUpdatedAt: '2026-03-23T15:30:00Z',
          })
        })
      )

      const response = await getFreeUserStatistics({
        path: { realmId: 'test-realm' },
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()

      if (response.data) {
        expect(response.data.totalFreeUsers).toBe(0)
        expect(response.data.totalPeriodicPointsGranted).toBe(0)
        expect(response.data.averagePeriodicPointsPerUser).toBe(0)
        expect(response.data.upgradeRate).toBe(0)
      }
    })
  })

  describe('API Error Handling', () => {
    it('GIVEN invalid date range WHEN requesting statistics THEN should return validation error', async () => {
      server.use(
        http.get('/api/points/test-realm/statistics/free-users', ({ request }) => {
          const url = new URL(request.url)
          const startDate = url.searchParams.get('startDate')
          const endDate = url.searchParams.get('endDate')

          if (startDate && endDate && startDate > endDate) {
            return HttpResponse.json(
              {
                message: 'Invalid date range: start date must be before end date',
                code: 'INVALID_DATE_RANGE',
              },
              { status: 400 }
            )
          }

          return HttpResponse.json(mockFreeUserStatistics)
        })
      )

      const response = await getFreeUserStatistics({
        path: { realmId: 'test-realm' },
        query: {
          startDate: '2026-12-31',
          endDate: '2026-01-01', // End date before start date
        },
      })

      expect(response.error).toBeDefined()
      expect(response.data).toBeUndefined()
    })

    it('GIVEN API returns 404 WHEN requesting statistics THEN should handle not found error', async () => {
      server.use(
        http.get('/api/points/non-existent-realm/statistics/free-users', () => {
          return HttpResponse.json(
            {
              message: 'Realm not found',
              code: 'REALM_NOT_FOUND',
            },
            { status: 404 }
          )
        })
      )

      const response = await getFreeUserStatistics({
        path: { realmId: 'non-existent-realm' },
      })

      expect(response.error).toBeDefined()
      expect(response.data).toBeUndefined()
    })

    it('GIVEN API returns 500 WHEN requesting statistics THEN should handle server error', async () => {
      server.use(
        http.get('/api/points/test-realm/statistics/free-users', () => {
          return HttpResponse.json(
            {
              message: 'Internal server error',
              code: 'INTERNAL_ERROR',
            },
            { status: 500 }
          )
        })
      )

      const response = await getFreeUserStatistics({
        path: { realmId: 'test-realm' },
      })

      expect(response.error).toBeDefined()
      expect(response.data).toBeUndefined()
    })
  })

  describe('Field Renaming Verification', () => {
    it('GIVEN statistics response WHEN verifying field names THEN should use periodic naming', async () => {
      const response = await getFreeUserStatistics({
        path: { realmId: 'test-realm' },
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()

      if (response.data) {
        // Verify old daily fields are renamed to periodic
        const hasOldDailyFields =
          'totalDailyPointsGranted' in response.data || 'averageDailyPointsPerUser' in response.data

        const hasNewPeriodicFields =
          'totalPeriodicPointsGranted' in response.data &&
          'averagePeriodicPointsPerUser' in response.data

        expect(hasOldDailyFields).toBe(false)
        expect(hasNewPeriodicFields).toBe(true)
      }
    })
  })
})
