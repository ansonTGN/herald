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
  })
})
