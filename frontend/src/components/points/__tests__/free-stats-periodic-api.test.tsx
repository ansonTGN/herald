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
    it('GIVEN MSW mock statistics API WHEN calling getFreeUserStatistics THEN returns data with periodic naming', async () => {
      const realmId = 'test-realm'

      const response = await getFreeUserStatistics({
        path: { realmId },
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()

      if (response.data) {
        // Verify response structure
        expect(response.data.totalFreeUsers).toBeDefined()
        expect(typeof response.data.totalFreeUsers).toBe('number')

        expect(response.data.activeFreeUsers).toBeDefined()
        expect(typeof response.data.activeFreeUsers).toBe('number')

        expect(response.data.totalRegistrationBonusGranted).toBeDefined()
        expect(typeof response.data.totalRegistrationBonusGranted).toBe('number')

        // Verify periodic naming (NOT daily naming)
        expect(response.data.totalPeriodicPointsGranted).toBeDefined()
        expect(typeof response.data.totalPeriodicPointsGranted).toBe('number')

        expect(response.data.averagePeriodicPointsPerUser).toBeDefined()
        expect(typeof response.data.averagePeriodicPointsPerUser).toBe('number')

        expect(response.data.upgradeRate).toBeDefined()
        expect(typeof response.data.upgradeRate).toBe('number')

        expect(response.data.lastUpdatedAt).toBeDefined()
        expect(typeof response.data.lastUpdatedAt).toBe('string')
      }
    })

    it('GIVEN statistics response WHEN parsing THEN should use periodic field names', async () => {
      const response = await getFreeUserStatistics({
        path: { realmId: 'test-realm' },
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()

      if (response.data) {
        // Verify periodic naming is used
        expect(response.data).toHaveProperty('totalPeriodicPointsGranted')
        expect(response.data).toHaveProperty('averagePeriodicPointsPerUser')

        // Verify old daily naming is NOT used
        expect(response.data).not.toHaveProperty('totalDailyPointsGranted')
        expect(response.data).not.toHaveProperty('averageDailyPointsPerUser')

        // Verify values are reasonable
        expect(response.data.totalPeriodicPointsGranted).toBeGreaterThanOrEqual(0)
        expect(response.data.averagePeriodicPointsPerUser).toBeGreaterThanOrEqual(0)
        expect(response.data.upgradeRate).toBeGreaterThanOrEqual(0)
        expect(response.data.upgradeRate).toBeLessThanOrEqual(1) // Percentage as decimal
      }
    })

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
    it('GIVEN statistics response WHEN parsing THEN numeric fields should have correct types', async () => {
      const response = await getFreeUserStatistics({
        path: { realmId: 'test-realm' },
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()

      if (response.data) {
        // Verify integer fields
        expect(Number.isInteger(response.data.totalFreeUsers)).toBe(true)
        expect(Number.isInteger(response.data.activeFreeUsers)).toBe(true)
        expect(Number.isInteger(response.data.totalRegistrationBonusGranted)).toBe(true)
        expect(Number.isInteger(response.data.totalPeriodicPointsGranted)).toBe(true)

        // Verify decimal fields
        expect(response.data.averagePeriodicPointsPerUser).toBeGreaterThanOrEqual(0)
        expect(response.data.upgradeRate).toBeGreaterThanOrEqual(0)
        expect(response.data.upgradeRate).toBeLessThanOrEqual(1)

        // Verify date format
        expect(response.data.lastUpdatedAt).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/)
      }
    })

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

    it('GIVEN high upgrade rate statistics WHEN parsing THEN should handle percentages correctly', async () => {
      server.use(
        http.get('/api/points/test-realm/statistics/free-users', () => {
          return HttpResponse.json({
            totalFreeUsers: 1000,
            activeFreeUsers: 800,
            totalRegistrationBonusGranted: 1000000,
            totalPeriodicPointsGranted: 40000,
            averagePeriodicPointsPerUser: 50,
            upgradeRate: 0.1537, // 15.37%
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
        expect(response.data.upgradeRate).toBe(0.1537)
        expect(response.data.upgradeRate).toBeGreaterThan(0.15)
        expect(response.data.upgradeRate).toBeLessThan(0.16)
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

    it('GIVEN statistics response WHEN checking values THEN should be consistent with periodic naming', async () => {
      const response = await getFreeUserStatistics({
        path: { realmId: 'test-realm' },
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()

      if (response.data) {
        // Verify the periodic fields have reasonable values
        expect(response.data.totalFreeUsers).toBe(1000)
        expect(response.data.totalPeriodicPointsGranted).toBe(40000)

        // Note: averagePeriodicPointsPerUser in mock is 50, but based on mock data:
        // totalPeriodicPointsGranted (40000) / totalFreeUsers (1000) = 40
        // This is intentional - backend may use activeFreeUsers (800) for calculation: 40000/800=50
        const expectedAverage =
          response.data.totalPeriodicPointsGranted / response.data.activeFreeUsers
        expect(response.data.averagePeriodicPointsPerUser).toBeCloseTo(expectedAverage, 2)
      }
    })
  })
})
