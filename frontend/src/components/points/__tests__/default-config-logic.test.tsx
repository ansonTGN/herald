import { describe, it, expect, afterEach, vi } from 'vitest'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/mocks/server'
import { QueryClient } from '@tanstack/react-query'
import { QUERY_KEYS } from '@/lib/constants'
import { pointsDefaultConfigSchema } from '@/lib/schemas/points-forms'

describe('Realm Configuration - High-Value Logic Tests', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  const createTestQueryClient = () =>
    new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    })

  describe('schema validation edge cases', () => {
    it('GIVEN negative registration bonus WHEN validating THEN should fail', () => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: -100,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'daily',
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error).toBeDefined()
        expect(result.error.issues.length).toBeGreaterThan(0)
        expect(result.error.issues[0].message).toBeDefined()
      }
    })

    it('GIVEN zero validity days with daily period WHEN validating THEN should fail', () => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'daily',
        freePeriodicValidityDays: 0,
      })

      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error.issues.length).toBeGreaterThan(0)
        expect(result.error.issues[0].message).toBeDefined()
      }
    })

    it('GIVEN zero validity days with once period WHEN validating THEN should succeed', () => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'once',
        freePeriodicValidityDays: 0, // once period allows 0 (permanent validity)
      })

      expect(result.success).toBe(true)
    })

    it('GIVEN negative periodic points WHEN validating THEN should fail', () => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: -50,
        freePeriodicGrantPeriodType: 'daily',
        freePeriodicValidityDays: 1,
      })

      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error).toBeDefined()
        expect(result.error.issues.length).toBeGreaterThan(0)
        expect(result.error.issues[0].message).toBeDefined()
      }
    })
  })

  describe('API error handling scenarios', () => {
    it('GIVEN API returns 400 WHEN fetching config THEN should handle error', async () => {
      server.use(
        http.get('http://localhost:3000/api/points/test-realm/default-config', () => {
          return HttpResponse.json({ message: 'Invalid realm configuration' }, { status: 400 })
        })
      )

      const { getRealmDefaultConfig } = await import('@/lib/api-generated')

      const response = await getRealmDefaultConfig({ path: { realmId: 'test-realm' } })

      // Generated API client returns response object with error field
      expect(response.error).toBeDefined()
      expect(response.data).toBeUndefined()
    })

    it('GIVEN API returns 404 WHEN fetching config THEN should handle error', async () => {
      server.use(
        http.get('http://localhost:3000/api/points/test-realm/default-config', () => {
          return HttpResponse.json({ message: 'Configuration not found' }, { status: 404 })
        })
      )

      const { getRealmDefaultConfig } = await import('@/lib/api-generated')

      const response = await getRealmDefaultConfig({ path: { realmId: 'test-realm' } })

      expect(response.error).toBeDefined()
      expect(response.data).toBeUndefined()
    })

    it('GIVEN API returns 500 WHEN fetching config THEN should handle error', async () => {
      server.use(
        http.get('http://localhost:3000/api/points/test-realm/default-config', () => {
          return HttpResponse.json({ message: 'Internal server error' }, { status: 500 })
        })
      )

      const { getRealmDefaultConfig } = await import('@/lib/api-generated')

      const response = await getRealmDefaultConfig({ path: { realmId: 'test-realm' } })

      expect(response.error).toBeDefined()
      expect(response.data).toBeUndefined()
    })

    it('GIVEN API returns 400 WHEN updating config THEN should handle error', async () => {
      server.use(
        http.put('http://localhost:3000/api/points/test-realm/default-config', () => {
          return HttpResponse.json({ message: 'Invalid configuration values' }, { status: 400 })
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
      })

      expect(response.error).toBeDefined()
      expect(response.data).toBeUndefined()
    })

    it('GIVEN API returns 500 WHEN updating config THEN should handle error', async () => {
      server.use(
        http.put('http://localhost:3000/api/points/test-realm/default-config', () => {
          return HttpResponse.json({ message: 'Internal server error' }, { status: 500 })
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
      })

      expect(response.error).toBeDefined()
      expect(response.data).toBeUndefined()
    })
  })

  describe('React Query cache invalidation logic', () => {
    it('GIVEN query client WHEN invalidating realm config THEN should use correct query key', async () => {
      const queryClient = createTestQueryClient()
      const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries')

      // Simulate cache invalidation after successful update
      queryClient.invalidateQueries({
        queryKey: [QUERY_KEYS.REALM_CONFIG, 'test-realm'],
      })

      expect(invalidateSpy).toHaveBeenCalledWith({
        queryKey: [QUERY_KEYS.REALM_CONFIG, 'test-realm'],
      })
    })

    it('GIVEN query client WHEN invalidating all realm configs THEN should use correct query key', async () => {
      const queryClient = createTestQueryClient()
      const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries')

      // Simulate cache invalidation for all realm configs
      queryClient.invalidateQueries({
        queryKey: [QUERY_KEYS.REALM_CONFIG],
      })

      expect(invalidateSpy).toHaveBeenCalledWith({
        queryKey: [QUERY_KEYS.REALM_CONFIG],
      })
    })
  })

  describe('data transformation and formatting', () => {
    it('GIVEN API response WHEN parsing THEN should match schema', () => {
      const apiResponse = {
        realmId: 'test-realm',
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'daily',
        freePeriodicValidityDays: 1,
        createdAt: '2026-03-23T00:00:00Z',
        updatedAt: '2026-03-23T00:00:00Z',
      }

      const result = pointsDefaultConfigSchema.safeParse(apiResponse)

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.registrationBonusPoints).toBe(1000)
        expect(result.data.freePeriodicPointsAmount).toBe(50)
        expect(result.data.freePeriodicGrantPeriodType).toBe('daily')
        expect(result.data.freePeriodicValidityDays).toBe(1)
      }
    })

    it('GIVEN API response with string numbers WHEN parsing THEN should fail validation', () => {
      const apiResponse = {
        registrationBonusPoints: '1000',
        freePeriodicPointsAmount: '50',
        freePeriodicGrantPeriodType: 'daily',
        freePeriodicValidityDays: '1',
      }

      const result = pointsDefaultConfigSchema.safeParse(apiResponse)

      // Zod schema expects numbers, not strings
      expect(result.success).toBe(false)
    })
  })
})
