import { describe, it, expect, afterEach, beforeEach } from 'vitest'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/mocks/server'
import { createClient } from '@/lib/api-generated/client'

describe('Test 3.1: Realm Config API Contract Tests (P0)', () => {
  let testClient: ReturnType<typeof createClient>

  beforeEach(() => {
    // Create a test client with baseURL for all tests
    testClient = createClient({
      baseUrl: 'http://localhost:3000',
    })
  })

  afterEach(() => {
    server.resetHandlers()
  })

  describe('GET /api/points/{realmId}/default-config', () => {
    it('GIVEN MSW mock Realm config API WHEN calling getRealmDefaultConfig THEN returns data with periodic fields', async () => {
      const { getRealmDefaultConfig } = await import('@/lib/api-generated')
      const realmId = 'test-realm'

      const response = await getRealmDefaultConfig({
        path: { realmId },
        client: testClient,
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()

      if (response.data) {
        // Verify response structure
        expect(response.data.realmId).toBe(realmId)
        expect(response.data.registrationBonusPoints).toBeDefined()
        expect(typeof response.data.registrationBonusPoints).toBe('number')

        // Verify periodic fields exist (new fields)
        expect(response.data.freePeriodicPointsAmount).toBeDefined()
        expect(typeof response.data.freePeriodicPointsAmount).toBe('number')

        expect(response.data.freePeriodicGrantPeriodType).toBeDefined()
        expect(typeof response.data.freePeriodicGrantPeriodType).toBe('string')
        expect(['once', 'daily', 'weekly', 'monthly']).toContain(
          response.data.freePeriodicGrantPeriodType
        )

        expect(response.data.freePeriodicValidityDays).toBeDefined()
        expect(typeof response.data.freePeriodicValidityDays).toBe('number')

        expect(response.data.createdAt).toBeDefined()
        expect(typeof response.data.createdAt).toBe('string')

        expect(response.data.updatedAt).toBeDefined()
        expect(typeof response.data.updatedAt).toBe('string')
      }
    })

    it('GIVEN API returns periodic config WHEN parsing THEN fields have correct types', async () => {
      const { getRealmDefaultConfig } = await import('@/lib/api-generated')

      const response = await getRealmDefaultConfig({
        path: { realmId: 'test-realm' },
        client: testClient,
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()

      if (response.data) {
        // Verify field types match TypeScript definition
        expect(response.data.freePeriodicGrantPeriodType).toMatch(/once|daily|weekly|monthly/)

        // Verify numeric fields
        expect(response.data.freePeriodicPointsAmount).toBeGreaterThanOrEqual(0)
        expect(response.data.freePeriodicValidityDays).toBeGreaterThanOrEqual(0)

        // Verify ISO 8601 date format
        expect(response.data.createdAt).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/)
        expect(response.data.updatedAt).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/)
      }
    })

    it('GIVEN realmId with special characters WHEN calling API THEN should handle correctly', async () => {
      const { getRealmDefaultConfig } = await import('@/lib/api-generated')
      const realmId = 'test-realm-123'

      const response = await getRealmDefaultConfig({
        path: { realmId },
        client: testClient,
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()

      if (response.data) {
        expect(response.data.realmId).toBe(realmId)
      }
    })
  })

  describe('PUT /api/points/{realmId}/default-config', () => {
    it('GIVEN valid periodic config WHEN updating THEN accepts periodic fields', async () => {
      const { updateRealmDefaultConfig } = await import('@/lib/api-generated')
      const realmId = 'test-realm'
      const updateData = {
        registrationBonusPoints: 1500,
        freePeriodicPointsAmount: 75,
        freePeriodicGrantPeriodType: 'weekly' as const,
        freePeriodicValidityDays: 7,
      }

      const response = await updateRealmDefaultConfig({
        path: { realmId },
        body: updateData,
        client: testClient,
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()

      if (response.data) {
        // Verify updated values are returned
        expect(response.data.registrationBonusPoints).toBe(updateData.registrationBonusPoints)
        expect(response.data.freePeriodicPointsAmount).toBe(updateData.freePeriodicPointsAmount)
        expect(response.data.freePeriodicGrantPeriodType).toBe(
          updateData.freePeriodicGrantPeriodType
        )
        expect(response.data.freePeriodicValidityDays).toBe(updateData.freePeriodicValidityDays)
      }
    })

    it('GIVEN once period type with validityDays=0 WHEN updating THEN should accept', async () => {
      const { updateRealmDefaultConfig } = await import('@/lib/api-generated')
      const realmId = 'test-realm'
      const updateData = {
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 100,
        freePeriodicGrantPeriodType: 'once' as const,
        freePeriodicValidityDays: 0, // Permanent validity for once period
      }

      const response = await updateRealmDefaultConfig({
        path: { realmId },
        body: updateData,
        client: testClient,
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()

      if (response.data) {
        expect(response.data.freePeriodicGrantPeriodType).toBe('once')
        expect(response.data.freePeriodicValidityDays).toBe(0)
      }
    })

    it('GIVEN monthly period type WHEN updating THEN should accept', async () => {
      const { updateRealmDefaultConfig } = await import('@/lib/api-generated')
      const realmId = 'test-realm'
      const updateData = {
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 200,
        freePeriodicGrantPeriodType: 'monthly' as const,
        freePeriodicValidityDays: 30,
      }

      const response = await updateRealmDefaultConfig({
        path: { realmId },
        body: updateData,
        client: testClient,
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()

      if (response.data) {
        expect(response.data.freePeriodicGrantPeriodType).toBe('monthly')
        expect(response.data.freePeriodicValidityDays).toBe(30)
      }
    })
  })

  describe('API Error Handling', () => {
    it('GIVEN API returns 400 WHEN updating config THEN should handle validation error', async () => {
      server.use(
        http.put('http://localhost:3000/api/points/test-realm/default-config', () => {
          return HttpResponse.json(
            {
              message: 'Invalid configuration values',
              code: 'INVALID_CONFIG',
            },
            { status: 400 }
          )
        })
      )

      const { updateRealmDefaultConfig } = await import('@/lib/api-generated')

      const response = await updateRealmDefaultConfig({
        path: { realmId: 'test-realm' },
        body: {
          registrationBonusPoints: 1000,
          freePeriodicPointsAmount: 50,
          freePeriodicGrantPeriodType: 'daily',
          freePeriodicValidityDays: 1,
        },
        client: testClient,
      })

      expect(response.error).toBeDefined()
      expect(response.data).toBeUndefined()
    })

    it('GIVEN API returns 404 WHEN fetching config THEN should handle not found error', async () => {
      server.use(
        http.get('http://localhost:3000/api/points/non-existent-realm/default-config', () => {
          return HttpResponse.json(
            {
              message: 'Realm configuration not found',
              code: 'REALM_CONFIG_NOT_FOUND',
            },
            { status: 404 }
          )
        })
      )

      const { getRealmDefaultConfig } = await import('@/lib/api-generated')

      const response = await getRealmDefaultConfig({
        path: { realmId: 'non-existent-realm' },
        client: testClient,
      })

      expect(response.error).toBeDefined()
      expect(response.data).toBeUndefined()
    })

    it('GIVEN API returns 500 WHEN fetching config THEN should handle server error', async () => {
      server.use(
        http.get('http://localhost:3000/api/points/test-realm/default-config', () => {
          return HttpResponse.json(
            {
              message: 'Internal server error',
              code: 'INTERNAL_ERROR',
            },
            { status: 500 }
          )
        })
      )

      const { getRealmDefaultConfig } = await import('@/lib/api-generated')

      const response = await getRealmDefaultConfig({
        path: { realmId: 'test-realm' },
        client: testClient,
      })

      expect(response.error).toBeDefined()
      expect(response.data).toBeUndefined()
    })

    it('GIVEN invalid periodic period type WHEN updating THEN should return validation error', async () => {
      server.use(
        http.put(
          'http://localhost:3000/api/points/test-realm/default-config',
          async ({ request }) => {
            const body = (await request.json()) as any

            if (
              !['once', 'daily', 'weekly', 'monthly'].includes(body.freePeriodicGrantPeriodType)
            ) {
              return HttpResponse.json(
                {
                  message:
                    'Invalid grant period type. Must be one of: once, daily, weekly, monthly',
                  code: 'INVALID_PERIOD_TYPE',
                },
                { status: 400 }
              )
            }

            return HttpResponse.json({ status: 200 })
          }
        )
      )

      const { updateRealmDefaultConfig } = await import('@/lib/api-generated')

      const response = await updateRealmDefaultConfig({
        path: { realmId: 'test-realm' },
        body: {
          registrationBonusPoints: 1000,
          freePeriodicPointsAmount: 50,
          freePeriodicGrantPeriodType: 'yearly' as any, // Invalid
          freePeriodicValidityDays: 365,
        },
        client: testClient,
      })

      expect(response.error).toBeDefined()
    })

    it('GIVEN non-once period with validityDays=0 WHEN updating THEN should return validation error', async () => {
      server.use(
        http.put(
          'http://localhost:3000/api/points/test-realm/default-config',
          async ({ request }) => {
            const body = (await request.json()) as any

            if (body.freePeriodicGrantPeriodType !== 'once' && body.freePeriodicValidityDays < 1) {
              return HttpResponse.json(
                {
                  message: 'Validity days must be at least 1 for non-once periods',
                  code: 'INVALID_VALIDITY_FOR_PERIOD',
                },
                { status: 400 }
              )
            }

            return HttpResponse.json({ status: 200 })
          }
        )
      )

      const { updateRealmDefaultConfig } = await import('@/lib/api-generated')

      const response = await updateRealmDefaultConfig({
        path: { realmId: 'test-realm' },
        body: {
          registrationBonusPoints: 1000,
          freePeriodicPointsAmount: 50,
          freePeriodicGrantPeriodType: 'daily',
          freePeriodicValidityDays: 0, // Invalid for daily period
        },
        client: testClient,
      })

      expect(response.error).toBeDefined()
    })
  })

  describe('Field Name Mapping', () => {
    it('GIVEN API request WHEN sending data THEN uses camelCase (frontend convention)', async () => {
      const { updateRealmDefaultConfig } = await import('@/lib/api-generated')
      const realmId = 'test-realm'
      const updateData = {
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50, // camelCase
        freePeriodicGrantPeriodType: 'daily' as const, // camelCase
        freePeriodicValidityDays: 1, // camelCase
      }

      const response = await updateRealmDefaultConfig({
        path: { realmId },
        body: updateData,
        client: testClient,
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()

      // Verify the API accepts camelCase field names
      if (response.data) {
        expect(response.data.freePeriodicPointsAmount).toBeDefined()
        expect(response.data.freePeriodicGrantPeriodType).toBeDefined()
        expect(response.data.freePeriodicValidityDays).toBeDefined()
      }
    })

    it('GIVEN API response WHEN receiving data THEN uses camelCase (frontend convention)', async () => {
      const { getRealmDefaultConfig } = await import('@/lib/api-generated')

      const response = await getRealmDefaultConfig({
        path: { realmId: 'test-realm' },
        client: testClient,
      })

      expect(response.error).toBeUndefined()
      expect(response.data).toBeDefined()

      if (response.data) {
        // Verify response uses camelCase
        expect(response.data).toHaveProperty('registrationBonusPoints')
        expect(response.data).toHaveProperty('freePeriodicPointsAmount')
        expect(response.data).toHaveProperty('freePeriodicGrantPeriodType')
        expect(response.data).toHaveProperty('freePeriodicValidityDays')

        // Should NOT have snake_case versions
        expect(response.data).not.toHaveProperty('registration_bonus_points')
        expect(response.data).not.toHaveProperty('free_periodic_points_amount')
        expect(response.data).not.toHaveProperty('free_periodic_grant_period_type')
        expect(response.data).not.toHaveProperty('free_periodic_validity_days')
      }
    })
  })
})
