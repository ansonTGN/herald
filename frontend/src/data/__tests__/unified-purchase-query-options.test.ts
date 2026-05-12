/**
 * Query Options Tests for Unified Purchase Feature
 *
 * Tests query configuration, caching strategies, and error handling
 * for points packages, payment attempts, and purchase history.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { QueryClient } from '@tanstack/react-query'
import {
  pointsPackagesQueryOptions,
  pointsPackageQueryOptions,
  pointsPackagePurchaseHistoryQueryOptions,
  paymentAttemptStatusQueryOptions,
  paymentProvidersQueryOptions,
  paymentProviderMappingsQueryOptions,
  queryKeys,
} from '../query-options'

// Mock the API functions
vi.mock('@/lib/api-generated', async () => {
  const actual = await vi.importActual<typeof import('@/lib/api-generated')>('@/lib/api-generated')
  return {
    ...actual,
    listPointsPackages: vi.fn(),
    getPointsPackage: vi.fn(),
    getPointsPackagePurchaseHistory: vi.fn(),
    getPaymentAttemptStatus: vi.fn(),
    listPaymentProviders: vi.fn(),
    listPaymentProviderMappings: vi.fn(),
  }
})

import {
  listPointsPackages,
  getPointsPackage,
  getPointsPackagePurchaseHistory,
  getPaymentAttemptStatus,
  listPaymentProviders,
  listPaymentProviderMappings,
} from '@/lib/api-generated'

describe('Unified Purchase Query Options', () => {
  let queryClient: QueryClient

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    })
    vi.clearAllMocks()
  })

  describe('pointsPackagesQueryOptions', () => {
    describe('query configuration', () => {
      it('should create correct query key', () => {
        const options = pointsPackagesQueryOptions('realm-1')
        expect(options.queryKey).toEqual(queryKeys.pointsPackages('realm-1'))
      })

      it('should create unique keys for different realms', () => {
        const options1 = pointsPackagesQueryOptions('realm-1')
        const options2 = pointsPackagesQueryOptions('realm-2')
        expect(options1.queryKey).not.toEqual(options2.queryKey)
      })

      it('should configure retry count', () => {
        const options = pointsPackagesQueryOptions('realm-1')
        expect(options.retry).toBe(1)
      })

      it('should configure stale time', () => {
        const options = pointsPackagesQueryOptions('realm-1')
        expect(options.staleTime).toBe(5 * 60 * 1000) // 5 minutes
      })

      it('should have a query function', () => {
        const options = pointsPackagesQueryOptions('realm-1')
        expect(options.queryFn).toBeDefined()
        expect(typeof options.queryFn).toBe('function')
      })
    })

    describe('query execution', () => {
      it('should call listPointsPackages with correct parameters', async () => {
        const mockResponse = {
          data: {
            packages: [
              {
                id: 'pkg-1',
                name: 'starter_pack',
                title: 'Starter Pack',
                points: 1000,
                price: 9.99,
                currency: 'USD',
                enabled: true,
                createdAt: '2025-01-01T00:00:00Z',
                updatedAt: '2025-01-01T00:00:00Z',
                realmId: 'realm-1',
              },
            ],
          },
          error: undefined,
        }

        vi.mocked(listPointsPackages).mockResolvedValueOnce(mockResponse as any)

        const options = pointsPackagesQueryOptions('realm-1')
        const result = await options.queryFn!()

        expect(listPointsPackages).toHaveBeenCalledWith({
          path: { realmId: 'realm-1' },
        })
        expect(result).toEqual(mockResponse.data.packages)
      })

      it('should throw error when API returns error', async () => {
        const mockError = new Error('API Error')
        vi.mocked(listPointsPackages).mockResolvedValueOnce({
          data: undefined,
          error: mockError,
        } as any)

        const options = pointsPackagesQueryOptions('realm-1')
        await expect(options.queryFn!()).rejects.toThrow(mockError)
      })

      it('should handle empty packages list', async () => {
        const mockResponse = {
          data: {
            packages: [],
          },
          error: undefined,
        }

        vi.mocked(listPointsPackages).mockResolvedValueOnce(mockResponse as any)

        const options = pointsPackagesQueryOptions('realm-1')
        const result = await options.queryFn!()

        expect(result).toEqual([])
      })

      it('should handle multiple packages', async () => {
        const mockResponse = {
          data: {
            packages: [
              {
                id: 'pkg-1',
                name: 'starter_pack',
                title: 'Starter Pack',
                points: 1000,
                price: 9.99,
                currency: 'USD',
                enabled: true,
                createdAt: '2025-01-01T00:00:00Z',
                updatedAt: '2025-01-01T00:00:00Z',
                realmId: 'realm-1',
              },
              {
                id: 'pkg-2',
                name: 'premium_pack',
                title: 'Premium Pack',
                points: 5000,
                price: 39.99,
                currency: 'USD',
                enabled: true,
                createdAt: '2025-01-01T00:00:00Z',
                updatedAt: '2025-01-01T00:00:00Z',
                realmId: 'realm-1',
              },
            ],
          },
          error: undefined,
        }

        vi.mocked(listPointsPackages).mockResolvedValueOnce(mockResponse as any)

        const options = pointsPackagesQueryOptions('realm-1')
        const result = await options.queryFn!()

        expect(result).toHaveLength(2)
      })

      it('should handle listPointsPackages errors', async () => {
        const mockError = new Error('Network error')
        vi.mocked(listPointsPackages).mockRejectedValueOnce(mockError)

        const options = pointsPackagesQueryOptions('realm-1')
        await expect(options.queryFn!()).rejects.toThrow('Network error')
      })
    })
  })

  describe('pointsPackageQueryOptions', () => {
    describe('query configuration', () => {
      it('should include realmId and packageId in query key', () => {
        const options = pointsPackageQueryOptions('realm-1', 'pkg-1')
        expect(options.queryKey).toEqual(queryKeys.pointsPackage('realm-1', 'pkg-1'))
      })

      it('should create unique keys for different packages', () => {
        const options1 = pointsPackageQueryOptions('realm-1', 'pkg-1')
        const options2 = pointsPackageQueryOptions('realm-1', 'pkg-2')
        expect(options1.queryKey).not.toEqual(options2.queryKey)
      })

      it('should configure stale time for single package', () => {
        const options = pointsPackageQueryOptions('realm-1', 'pkg-1')
        expect(options.staleTime).toBe(5 * 60 * 1000) // 5 minutes
      })
    })

    describe('query execution', () => {
      it('should call getPointsPackage with correct parameters', async () => {
        const mockResponse = {
          data: {
            id: 'pkg-1',
            name: 'starter_pack',
            title: 'Starter Pack',
            points: 1000,
            price: 9.99,
            currency: 'USD',
            enabled: true,
            realmId: 'realm-1',
          },
          error: undefined,
        }

        vi.mocked(getPointsPackage).mockResolvedValueOnce(mockResponse as any)

        const options = pointsPackageQueryOptions('realm-1', 'pkg-1')
        const result = await options.queryFn!()

        expect(getPointsPackage).toHaveBeenCalledWith({
          path: { realmId: 'realm-1', packageId: 'pkg-1' },
        })
        expect(result).toEqual(mockResponse.data)
      })

      it('should handle getPointsPackage errors', async () => {
        const mockError = new Error('Package not found')
        vi.mocked(getPointsPackage).mockRejectedValueOnce(mockError)

        const options = pointsPackageQueryOptions('realm-1', 'pkg-1')
        await expect(options.queryFn!()).rejects.toThrow('Package not found')
      })
    })
  })

  describe('pointsPackagePurchaseHistoryQueryOptions', () => {
    describe('query configuration', () => {
      it('should include pagination in query key', () => {
        const options = pointsPackagePurchaseHistoryQueryOptions('realm-1', {
          page: 1,
          pageSize: 20,
        })
        expect(options.queryKey).toContainEqual({ page: 1, pageSize: 20 })
      })

      it('should include filters in query key', () => {
        const filters = { status: 'Succeeded', userId: 'user-1' }
        const options = pointsPackagePurchaseHistoryQueryOptions('realm-1', filters)
        expect(options.queryKey).toContainEqual(filters)
      })

      it('should create unique keys for different filters', () => {
        const options1 = pointsPackagePurchaseHistoryQueryOptions('realm-1', {
          status: 'Succeeded',
        })
        const options2 = pointsPackagePurchaseHistoryQueryOptions('realm-1', {
          status: 'Failed',
        })
        expect(options1.queryKey).not.toEqual(options2.queryKey)
      })

      it('should configure moderate stale time for history', () => {
        const options = pointsPackagePurchaseHistoryQueryOptions('realm-1', {})
        expect(options.staleTime).toBe(2 * 60 * 1000) // 2 minutes
      })
    })

    describe('filter parameter handling', () => {
      it('should handle empty filters', () => {
        const options = pointsPackagePurchaseHistoryQueryOptions('realm-1', {})
        expect(options.queryKey).toBeDefined()
      })

      it('should handle partial filters', () => {
        const options = pointsPackagePurchaseHistoryQueryOptions('realm-1', {
          status: 'Succeeded',
        })
        expect(options.queryKey).toContainEqual({ status: 'Succeeded' })
      })

      it('should handle complete filters', () => {
        const filters = {
          status: 'Succeeded' as const,
          userId: 'user-1',
          startTime: '2025-01-01',
          endTime: '2025-01-31',
        }
        const options = pointsPackagePurchaseHistoryQueryOptions('realm-1', filters)
        expect(options.queryKey).toContainEqual(filters)
      })
    })

    describe('query execution', () => {
      it('should call getPointsPackagePurchaseHistory with correct parameters', async () => {
        const mockResponse = {
          data: {
            purchases: [],
            pagination: {
              page: 1,
              pageSize: 20,
              total: 0,
              totalPages: 0,
            },
          },
          error: undefined,
        }

        vi.mocked(getPointsPackagePurchaseHistory).mockResolvedValueOnce(mockResponse as any)

        const filters = {
          page: 1,
          pageSize: 20,
          status: 'Succeeded',
        }
        const options = pointsPackagePurchaseHistoryQueryOptions('realm-1', filters)
        const result = await options.queryFn!()

        expect(getPointsPackagePurchaseHistory).toHaveBeenCalledWith({
          path: { realmId: 'realm-1' },
          query: {
            limit: 20,
            offset: 0,
          },
        })
        expect(result).toEqual(mockResponse.data)
      })

      it('should handle default pagination values', async () => {
        const mockResponse = {
          data: {
            purchases: [],
            pagination: {
              page: 1,
              pageSize: 20,
              total: 0,
              totalPages: 0,
            },
          },
          error: undefined,
        }

        vi.mocked(getPointsPackagePurchaseHistory).mockResolvedValueOnce(mockResponse as any)

        const options = pointsPackagePurchaseHistoryQueryOptions('realm-1', {})
        await options.queryFn!()

        expect(getPointsPackagePurchaseHistory).toHaveBeenCalledWith({
          path: { realmId: 'realm-1' },
          query: {
            limit: 20,
            offset: 0,
          },
        })
      })

      it('should handle different page sizes', async () => {
        const mockResponse = {
          data: {
            purchases: [],
            pagination: {
              page: 1,
              pageSize: 50,
              total: 0,
              totalPages: 0,
            },
          },
          error: undefined,
        }

        vi.mocked(getPointsPackagePurchaseHistory).mockResolvedValueOnce(mockResponse as any)

        const options = pointsPackagePurchaseHistoryQueryOptions('realm-1', { pageSize: 50 })
        await options.queryFn!()

        expect(getPointsPackagePurchaseHistory).toHaveBeenCalledWith({
          path: { realmId: 'realm-1' },
          query: expect.objectContaining({
            limit: 50,
          }),
        })
      })

      it('should handle different page numbers', async () => {
        const mockResponse = {
          data: {
            purchases: [],
            pagination: {
              page: 2,
              pageSize: 20,
              total: 0,
              totalPages: 0,
            },
          },
          error: undefined,
        }

        vi.mocked(getPointsPackagePurchaseHistory).mockResolvedValueOnce(mockResponse as any)

        const options = pointsPackagePurchaseHistoryQueryOptions('realm-1', { page: 2 })
        await options.queryFn!()

        expect(getPointsPackagePurchaseHistory).toHaveBeenCalledWith({
          path: { realmId: 'realm-1' },
          query: expect.objectContaining({
            offset: 20,
          }),
        })
      })

      it('should handle filtered purchase history', async () => {
        const mockResponse = {
          data: {
            purchases: [
              {
                id: 'purchase-1',
                status: 'Succeeded',
                userId: 'user-1',
              },
            ],
            pagination: {
              page: 1,
              pageSize: 20,
              total: 1,
              totalPages: 1,
            },
          },
          error: undefined,
        }

        vi.mocked(getPointsPackagePurchaseHistory).mockResolvedValueOnce(mockResponse as any)

        const filters = { status: 'Succeeded', userId: 'user-1' }
        const options = pointsPackagePurchaseHistoryQueryOptions('realm-1', filters)
        const result = await options.queryFn!()

        expect(result.purchases).toHaveLength(1)
        expect(result.purchases[0].status).toBe('Succeeded')
      })

      it('should handle purchase history errors', async () => {
        const mockError = new Error('History service unavailable')
        vi.mocked(getPointsPackagePurchaseHistory).mockRejectedValueOnce(mockError)

        const options = pointsPackagePurchaseHistoryQueryOptions('realm-1', {})
        await expect(options.queryFn!()).rejects.toThrow('History service unavailable')
      })

      it('should handle missing pagination data', async () => {
        const mockResponse = {
          data: {
            purchases: [],
            pagination: undefined,
          },
          error: undefined,
        }

        vi.mocked(getPointsPackagePurchaseHistory).mockResolvedValueOnce(mockResponse as any)

        const options = pointsPackagePurchaseHistoryQueryOptions('realm-1', {})
        const result = await options.queryFn!()

        expect(result.purchases).toEqual([])
      })
    })
  })

  describe('paymentAttemptStatusQueryOptions', () => {
    describe('query configuration', () => {
      it('should include realmId and attemptId in query key', () => {
        const options = paymentAttemptStatusQueryOptions('realm-1', 'attempt-1')
        expect(options.queryKey).toEqual(queryKeys.paymentAttemptStatus('realm-1', 'attempt-1'))
      })

      it('should create unique keys for different attempts', () => {
        const options1 = paymentAttemptStatusQueryOptions('realm-1', 'attempt-1')
        const options2 = paymentAttemptStatusQueryOptions('realm-1', 'attempt-2')
        expect(options1.queryKey).not.toEqual(options2.queryKey)
      })

      it('should configure short stale time for polling', () => {
        const options = paymentAttemptStatusQueryOptions('realm-1', 'attempt-1')
        expect(options.staleTime).toBe(60 * 1000) // 1 minute
      })
    })

    describe('polling behavior', () => {
      it('should poll for pending status', () => {
        const options = paymentAttemptStatusQueryOptions('realm-1', 'attempt-1')
        const mockQuery = {
          state: {
            data: {
              status: 'Pending',
              attemptId: 'attempt-1',
              realmId: 'realm-1',
            },
          },
        }
        const interval = options.refetchInterval?.(mockQuery as any)
        expect(interval).toBe(60 * 1000) // Poll every minute
      })

      it('should poll for requires_action status', () => {
        const options = paymentAttemptStatusQueryOptions('realm-1', 'attempt-1')
        const mockQuery = {
          state: {
            data: {
              status: 'RequiresAction',
              attemptId: 'attempt-1',
              realmId: 'realm-1',
            },
          },
        }
        const interval = options.refetchInterval?.(mockQuery as any)
        expect(interval).toBe(60 * 1000) // Poll every minute
      })

      it('should stop polling for succeeded status', () => {
        const options = paymentAttemptStatusQueryOptions('realm-1', 'attempt-1')
        const mockData = {
          status: 'Succeeded',
          attemptId: 'attempt-1',
          realmId: 'realm-1',
        }
        const interval = options.refetchInterval?.(mockData as any)
        expect(interval).toBe(false) // Stop polling
      })

      it('should stop polling for failed status', () => {
        const options = paymentAttemptStatusQueryOptions('realm-1', 'attempt-1')
        const mockData = {
          status: 'Failed',
          attemptId: 'attempt-1',
          realmId: 'realm-1',
        }
        const interval = options.refetchInterval?.(mockData as any)
        expect(interval).toBe(false) // Stop polling
      })

      it('should stop polling for cancelled status', () => {
        const options = paymentAttemptStatusQueryOptions('realm-1', 'attempt-1')
        const mockData = {
          status: 'Cancelled',
          attemptId: 'attempt-1',
          realmId: 'realm-1',
        }
        const interval = options.refetchInterval?.(mockData as any)
        expect(interval).toBe(false) // Stop polling
      })

      it('should handle undefined data', () => {
        const options = paymentAttemptStatusQueryOptions('realm-1', 'attempt-1')
        const interval = options.refetchInterval?.(undefined as any)
        expect(interval).toBe(false) // Stop polling
      })
    })

    describe('query execution', () => {
      it('should call getPaymentAttemptStatus with correct parameters', async () => {
        const mockResponse = {
          data: {
            attemptId: 'attempt-1',
            status: 'Pending',
            targetType: 'points_package',
            targetId: 'pkg-1',
            paymentProvider: 'wechat',
            createdAt: '2025-01-01T00:00:00Z',
            expiresAt: '2025-01-01T02:00:00Z',
          },
          error: undefined,
        }

        vi.mocked(getPaymentAttemptStatus).mockResolvedValueOnce(mockResponse as any)

        const options = paymentAttemptStatusQueryOptions('realm-1', 'attempt-1')
        const result = await options.queryFn!()

        expect(getPaymentAttemptStatus).toHaveBeenCalledWith({
          path: { realmId: 'realm-1', attemptId: 'attempt-1' },
        })
        expect(result).toEqual(mockResponse.data)
      })

      it('should handle payment attempt status errors', async () => {
        const mockError = new Error('Payment service error')
        vi.mocked(getPaymentAttemptStatus).mockRejectedValueOnce(mockError)

        const options = paymentAttemptStatusQueryOptions('realm-1', 'attempt-1')
        await expect(options.queryFn!()).rejects.toThrow('Payment service error')
      })
    })
  })

  describe('paymentProvidersQueryOptions', () => {
    describe('query configuration', () => {
      it('should create correct query key', () => {
        const options = paymentProvidersQueryOptions('realm-1')
        expect(options.queryKey).toEqual(['payment-providers', 'realm-1'])
      })

      it('should configure retry count', () => {
        const options = paymentProvidersQueryOptions('realm-1')
        expect(options.retry).toBe(1)
      })

      it('should configure stale time', () => {
        const options = paymentProvidersQueryOptions('realm-1')
        expect(options.staleTime).toBe(5 * 60 * 1000) // 5 minutes
      })
    })

    describe('query execution', () => {
      it('should call listPaymentProviders with correct parameters', async () => {
        const mockResponse = {
          data: {
            providers: [
              {
                paymentProvider: 'wechat',
                enabled: true,
              },
              {
                paymentProvider: 'stripe',
                enabled: true,
              },
            ],
          },
          error: undefined,
        }

        vi.mocked(listPaymentProviders).mockResolvedValueOnce(mockResponse as any)

        const options = paymentProvidersQueryOptions('realm-1')
        const result = await options.queryFn!()

        expect(listPaymentProviders).toHaveBeenCalledWith({
          path: { realmId: 'realm-1' },
        })
        expect(result).toEqual(mockResponse.data?.providers ?? [])
      })

      it('should handle empty providers list', async () => {
        const mockResponse = {
          data: {
            providers: undefined,
          },
          error: undefined,
        }

        vi.mocked(listPaymentProviders).mockResolvedValueOnce(mockResponse as any)

        const options = paymentProvidersQueryOptions('realm-1')
        const result = await options.queryFn!()

        expect(result).toEqual([])
      })

      it('should handle undefined payment providers', async () => {
        const mockResponse = {
          data: {
            providers: undefined,
          },
          error: undefined,
        }

        vi.mocked(listPaymentProviders).mockResolvedValueOnce(mockResponse as any)

        const options = paymentProvidersQueryOptions('realm-1')
        const result = await options.queryFn!()

        expect(result).toEqual([])
      })

      it('should handle payment providers errors', async () => {
        const mockError = new Error('Providers API error')
        vi.mocked(listPaymentProviders).mockRejectedValueOnce(mockError)

        const options = paymentProvidersQueryOptions('realm-1')
        await expect(options.queryFn!()).rejects.toThrow('Providers API error')
      })
    })
  })

  describe('paymentProviderMappingsQueryOptions', () => {
    describe('query configuration', () => {
      it('should include realmId and packageId in query key', () => {
        const options = paymentProviderMappingsQueryOptions('realm-1', 'pkg-1')
        expect(options.queryKey).toEqual(['payment-provider-mappings', 'realm-1', 'pkg-1'])
      })

      it('should create unique keys for different packages', () => {
        const options1 = paymentProviderMappingsQueryOptions('realm-1', 'pkg-1')
        const options2 = paymentProviderMappingsQueryOptions('realm-1', 'pkg-2')
        expect(options1.queryKey).not.toEqual(options2.queryKey)
      })

      it('should configure stale time', () => {
        const options = paymentProviderMappingsQueryOptions('realm-1', 'pkg-1')
        expect(options.staleTime).toBe(5 * 60 * 1000) // 5 minutes
      })
    })

    describe('query execution', () => {
      it('should call listPaymentProviderMappings with correct parameters', async () => {
        const mockResponse = {
          data: {
            mappings: [
              {
                paymentProvider: 'wechat',
                enabled: true,
                externalProductId: 'wx_prod_123',
              },
            ],
          },
          error: undefined,
        }

        vi.mocked(listPaymentProviderMappings).mockResolvedValueOnce(mockResponse as any)

        const options = paymentProviderMappingsQueryOptions('realm-1', 'pkg-1')
        const result = await options.queryFn!()

        expect(listPaymentProviderMappings).toHaveBeenCalledWith({
          path: { realmId: 'realm-1', packageId: 'pkg-1' },
        })
        expect(result).toEqual(mockResponse.data?.mappings ?? [])
      })

      it('should handle empty mappings list', async () => {
        const mockResponse = {
          data: {
            mappings: undefined,
          },
          error: undefined,
        }

        vi.mocked(listPaymentProviderMappings).mockResolvedValueOnce(mockResponse as any)

        const options = paymentProviderMappingsQueryOptions('realm-1', 'pkg-1')
        const result = await options.queryFn!()

        expect(result).toEqual([])
      })

      it('should handle undefined provider mappings', async () => {
        const mockResponse = {
          data: {
            mappings: undefined,
          },
          error: undefined,
        }

        vi.mocked(listPaymentProviderMappings).mockResolvedValueOnce(mockResponse as any)

        const options = paymentProviderMappingsQueryOptions('realm-1', 'pkg-1')
        const result = await options.queryFn!()

        expect(result).toEqual([])
      })

      it('should handle payment provider mappings errors', async () => {
        const mockError = new Error('Mappings API error')
        vi.mocked(listPaymentProviderMappings).mockRejectedValueOnce(mockError)

        const options = paymentProviderMappingsQueryOptions('realm-1', 'pkg-1')
        await expect(options.queryFn!()).rejects.toThrow('Mappings API error')
      })
    })
  })
})
