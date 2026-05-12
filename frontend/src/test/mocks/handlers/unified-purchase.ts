/**
 * MSW handlers for unified-purchase feature
 *
 * Provides mock API responses for points packages, payment attempts, and purchase history
 */

import { http, HttpResponse } from 'msw'
import {
  mockPointsPackages,
  mockPaymentAttempts,
  mockPurchaseHistory,
  getMockPackagesByEnabled,
} from '@/test/fixtures/unified-purchase'

/**
 * Points Package CRUD handlers
 */
export const pointsPackageHandlers = [
  // List points packages
  http.get('/api/bill/:realmId/points-packages', ({ params, request }) => {
    const realmId = params.realmId as string
    const url = new URL(request.url)
    const enabled = url.searchParams.get('enabled')

    let packages = mockPointsPackages.filter((pkg) => pkg.realmId === realmId)

    // Filter by enabled status if provided
    if (enabled !== null) {
      const enabledBool = enabled === 'true'
      packages = getMockPackagesByEnabled(enabledBool)
    }

    return HttpResponse.json({ packages })
  }),

  // Get single points package
  http.get('/api/bill/:realmId/points-packages/:packageId', ({ params }) => {
    const pkg = mockPointsPackages.find((p) => p.id === params.packageId)
    if (!pkg) {
      return HttpResponse.json({ message: 'Points package not found' }, { status: 404 })
    }
    return HttpResponse.json(pkg)
  }),

  // Create points package
  http.post('/api/bill/:realmId/points-packages', async ({ request, params }) => {
    const body = (await request.json()) as any
    const newPackage = {
      id: `pkg-${Date.now()}`,
      realmId: params.realmId as string,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      sortOrder: 0,
      enabled: true,
      ...body,
    }
    return HttpResponse.json(newPackage, { status: 201 })
  }),

  // Update points package
  http.put('/api/bill/:realmId/points-packages/:packageId', async ({ request, params }) => {
    const pkg = mockPointsPackages.find((p) => p.id === params.packageId)
    if (!pkg) {
      return HttpResponse.json({ message: 'Points package not found' }, { status: 404 })
    }

    const body = (await request.json()) as any
    const updated = {
      ...pkg,
      ...body,
      updatedAt: new Date().toISOString(),
    }
    return HttpResponse.json(updated)
  }),

  // Delete points package
  http.delete('/api/bill/:realmId/points-packages/:packageId', ({ params }) => {
    const pkg = mockPointsPackages.find((p) => p.id === params.packageId)
    if (!pkg) {
      return HttpResponse.json({ message: 'Points package not found' }, { status: 404 })
    }

    // Simulate packages with purchase history cannot be deleted
    const packagesWithHistory = ['pkg-1']
    if (packagesWithHistory.includes(pkg.id)) {
      return HttpResponse.json(
        {
          message: 'Cannot delete package with purchase history',
          code: 'HAS_PURCHASE_HISTORY',
          suggestion: 'Disable the package instead',
        },
        { status: 409 }
      )
    }

    return HttpResponse.json({ message: 'Deleted successfully' })
  }),
]

/**
 * Payment Provider Mapping handlers
 */
export const paymentProviderMappingHandlers = [
  // List payment provider mappings for a package
  http.get('/api/bill/:realmId/points-packages/:packageId/payment-providers', ({ params }) => {
    const mappings = [
      {
        provider: 'wechat',
        enabled: true,
        externalProductId: 'wx_prod_123',
      },
      {
        provider: 'stripe',
        enabled: true,
        externalProductId: 'price_12345',
      },
    ]
    return HttpResponse.json({ mappings })
  }),

  // Create payment provider mapping
  http.post(
    '/api/bill/:realmId/points-packages/:packageId/payment-providers',
    async ({ request }) => {
      const body = await request.json()
      return HttpResponse.json(body, { status: 201 })
    }
  ),

  // Update payment provider mapping
  http.put(
    '/api/bill/:realmId/points-packages/:packageId/payment-providers/:provider',
    async ({ request }) => {
      const body = await request.json()
      return HttpResponse.json(body)
    }
  ),

  // Delete payment provider mapping
  http.delete('/api/bill/:realmId/points-packages/:packageId/payment-providers/:provider', () => {
    return HttpResponse.json({ message: 'Mapping deleted successfully' })
  }),
]

/**
 * Payment Attempt handlers
 */
export const paymentAttemptHandlers = [
  // Create payment attempt
  http.post('/api/bill/:realmId/purchase/payment-attempts', async ({ request }) => {
    const body = (await request.json()) as any
    const attempt = {
      ...mockPaymentAttempts.pending,
      id: `attempt-${Date.now()}`,
      ...body,
      createdAt: new Date().toISOString(),
    }
    return HttpResponse.json(attempt, { status: 201 })
  }),

  // Get payment attempt status
  http.get('/api/bill/:realmId/purchase/payment-attempts/:attemptId', ({ params }) => {
    const attemptId = params.attemptId as string
    const attempt = Object.values(mockPaymentAttempts).find((a) => a.id === attemptId)

    if (!attempt) {
      return HttpResponse.json({ message: 'Payment attempt not found' }, { status: 404 })
    }

    return HttpResponse.json(attempt)
  }),

  // Cancel payment attempt
  http.post('/api/bill/:realmId/purchase/payment-attempts/:attemptId/cancel', ({ params }) => {
    const attemptId = params.attemptId as string
    const attempt = Object.values(mockPaymentAttempts).find((a) => a.id === attemptId)

    if (!attempt) {
      return HttpResponse.json({ message: 'Payment attempt not found' }, { status: 404 })
    }

    // Return cancelled attempt
    return HttpResponse.json({
      ...attempt,
      status: 'Cancelled',
      completedAt: new Date().toISOString(),
    })
  }),
]

/**
 * Purchase History handlers
 */
export const purchaseHistoryHandlers = [
  // Get points package purchase history
  http.get('/api/bill/:realmId/purchase/points-packages/history', ({ request, params }) => {
    const url = new URL(request.url)
    const page = parseInt(url.searchParams.get('page') || '1')
    const pageSize = parseInt(url.searchParams.get('pageSize') || '20')
    const userId = url.searchParams.get('userId')
    const startTime = url.searchParams.get('startTime')
    const endTime = url.searchParams.get('endTime')

    let filtered = [...mockPurchaseHistory]

    // Apply filters
    if (userId) {
      filtered = filtered.filter((h) => h.userId === userId)
    }
    if (startTime) {
      filtered = filtered.filter((h) => h.createdAt >= startTime)
    }
    if (endTime) {
      filtered = filtered.filter((h) => h.createdAt <= endTime)
    }

    // Paginate
    const start = (page - 1) * pageSize
    const paginated = filtered.slice(start, start + pageSize)

    return HttpResponse.json({
      purchases: paginated,
      pagination: {
        page,
        pageSize,
        total: filtered.length,
        totalPages: Math.ceil(filtered.length / pageSize),
      },
    })
  }),

  // Get purchase details
  http.get('/api/bill/:realmId/purchase/points-packages/:purchaseId', ({ params }) => {
    const purchase = mockPurchaseHistory.find((p) => p.id === params.purchaseId)

    if (!purchase) {
      return HttpResponse.json({ message: 'Purchase not found' }, { status: 404 })
    }

    return HttpResponse.json(purchase)
  }),
]

/**
 * Error Scenario handlers
 */
export const unifiedPurchaseErrorHandlers = [
  // 500 Internal Server Error - Server failure
  http.get('/api/bill/:realmId/points-packages', () => {
    return HttpResponse.json(
      {
        message: 'Internal server error',
        code: 'INTERNAL_ERROR',
      },
      { status: 500 }
    )
  }),

  // 422 Validation Error
  http.post('/api/bill/:realmId/points-packages', async () => {
    return HttpResponse.json(
      {
        message: 'Validation failed',
        code: 'VALIDATION_ERROR',
        errors: {
          name: ['Package name must be at least 3 characters'],
        },
      },
      { status: 422 }
    )
  }),

  // Network timeout simulation
  http.get('/api/bill/:realmId/purchase/payment-attempts/:attemptId', () => {
    return new Promise((resolve) => {
      setTimeout(() => {
        resolve(
          HttpResponse.json(
            {
              message: 'Request timeout',
              code: 'TIMEOUT',
            },
            { status: 504 }
          )
        )
      }, 30000) // 30 second timeout
    })
  }),
]

/**
 * Combined handlers for unified-purchase feature
 */
export const unifiedPurchaseHandlers = [
  ...pointsPackageHandlers,
  ...paymentProviderMappingHandlers,
  ...paymentAttemptHandlers,
  ...purchaseHistoryHandlers,
]
