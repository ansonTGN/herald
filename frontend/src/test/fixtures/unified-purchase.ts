/**
 * Test fixtures for unified-purchase feature
 *
 * Provides mock data for points packages, payment attempts, and purchase history
 */

import type {
  PointsPackageResponse,
  PaymentAttemptStatusResponse,
  PurchaseHistoryItemDto,
  PaymentContextDto,
} from '@/lib/api-generated'

/**
 * Mock points packages for testing
 */
export const mockPointsPackages: PointsPackageResponse[] = [
  {
    id: 'pkg-1',
    name: 'starter_pack',
    title: 'Starter Pack',
    description: 'Perfect for beginners',
    points: 1000,
    price: 9.99,
    currency: 'USD',
    sortOrder: 1,
    enabled: true,
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-01-01T00:00:00Z',
    realmId: 'realm-1',
  },
  {
    id: 'pkg-2',
    name: 'pro_pack',
    title: 'Professional Pack',
    description: 'Best value for power users',
    points: 5000,
    price: 39.99,
    currency: 'USD',
    sortOrder: 2,
    enabled: true,
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-01-01T00:00:00Z',
    realmId: 'realm-1',
  },
  {
    id: 'pkg-3',
    name: 'enterprise_pack',
    title: 'Enterprise Pack',
    description: 'Maximum points for businesses',
    points: 10000,
    price: 79.99,
    currency: 'USD',
    sortOrder: 3,
    enabled: true,
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-01-01T00:00:00Z',
    realmId: 'realm-1',
  },
  {
    id: 'pkg-4',
    name: 'disabled_pack',
    title: 'Disabled Pack',
    description: 'This package is disabled',
    points: 2000,
    price: 19.99,
    currency: 'USD',
    sortOrder: 4,
    enabled: false,
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-01-01T00:00:00Z',
    realmId: 'realm-1',
  },
]

/**
 * Mock payment attempts with different statuses
 */
export const mockPaymentAttempts: Record<string, PaymentAttemptStatusResponse> = {
  pending: {
    id: 'attempt-pending',
    status: 'Pending',
    targetType: 'points_package',
    targetId: 'pkg-1',
    amount: 9.99,
    currency: 'USD',
    createdAt: '2025-01-01T00:00:00Z',
    expiresAt: '2025-01-01T02:00:00Z',
    completedAt: null,
    fulfillment: null,
    providerStatus: null,
  },
  requires_action: {
    id: 'attempt-action',
    status: 'RequiresAction',
    targetType: 'points_package',
    targetId: 'pkg-2',
    amount: 39.99,
    currency: 'USD',
    createdAt: '2025-01-01T00:00:00Z',
    expiresAt: '2025-01-01T02:00:00Z',
    completedAt: null,
    fulfillment: null,
    providerStatus: null,
  },
  succeeded: {
    id: 'attempt-success',
    status: 'Succeeded',
    targetType: 'points_package',
    targetId: 'pkg-1',
    amount: 9.99,
    currency: 'USD',
    createdAt: '2025-01-01T00:00:00Z',
    expiresAt: '2025-01-01T02:00:00Z',
    completedAt: '2025-01-01T00:05:00Z',
    fulfillment: {
      transactionId: 'txn-123',
      pointsGranted: 1000,
    },
    providerStatus: 'succeeded',
  },
  failed: {
    id: 'attempt-failed',
    status: 'Failed',
    targetType: 'points_package',
    targetId: 'pkg-1',
    amount: 9.99,
    currency: 'USD',
    createdAt: '2025-01-01T00:00:00Z',
    expiresAt: '2025-01-01T02:00:00Z',
    completedAt: '2025-01-01T00:03:00Z',
    fulfillment: null,
    providerStatus: 'failed',
  },
  cancelled: {
    id: 'attempt-cancelled',
    status: 'Cancelled',
    targetType: 'points_package',
    targetId: 'pkg-1',
    amount: 9.99,
    currency: 'USD',
    createdAt: '2025-01-01T00:00:00Z',
    expiresAt: '2025-01-01T02:00:00Z',
    completedAt: '2025-01-01T00:02:00Z',
    fulfillment: null,
    providerStatus: 'cancelled',
  },
  expired: {
    id: 'attempt-expired',
    status: 'Expired',
    targetType: 'points_package',
    targetId: 'pkg-1',
    amount: 9.99,
    currency: 'USD',
    createdAt: '2025-01-01T00:00:00Z',
    expiresAt: '2025-01-01T02:00:00Z',
    completedAt: null,
    fulfillment: null,
    providerStatus: 'expired',
  },
}

/**
 * Mock purchase history
 */
export const mockPurchaseHistory: PurchaseHistoryItemDto[] = [
  {
    id: 'purchase-1',
    userId: 'user-1',
    pointsPackageId: 'pkg-1',
    points: 1000,
    amount: 9.99,
    currency: 'USD',
    paymentProvider: 'wechat',
    realmId: 'realm-1',
    pointsTransactionId: 'txn-123',
    createdAt: '2025-01-01T00:00:00Z',
  },
  {
    id: 'purchase-2',
    userId: 'user-1',
    pointsPackageId: 'pkg-2',
    points: 5000,
    amount: 39.99,
    currency: 'USD',
    paymentProvider: 'stripe',
    realmId: 'realm-1',
    pointsTransactionId: 'txn-456',
    createdAt: '2025-01-02T00:00:00Z',
  },
  {
    id: 'purchase-3',
    userId: 'user-1',
    pointsPackageId: 'pkg-3',
    points: 10000,
    amount: 79.99,
    currency: 'USD',
    paymentProvider: 'creem',
    realmId: 'realm-1',
    pointsTransactionId: null,
    createdAt: '2025-01-03T00:00:00Z',
  },
  {
    id: 'purchase-4',
    userId: 'user-2',
    pointsPackageId: 'pkg-1',
    points: 1000,
    amount: 9.99,
    currency: 'USD',
    paymentProvider: 'wechat',
    realmId: 'realm-1',
    pointsTransactionId: null,
    createdAt: '2025-01-04T00:00:00Z',
  },
]

/**
 * Helper function to get a mock payment attempt by status
 */
export function getMockPaymentAttemptByStatus(status: string): PaymentAttemptStatusResponse {
  const attempt = Object.values(mockPaymentAttempts).find(
    (a) => a.status.toLowerCase() === status.toLowerCase()
  )
  if (!attempt) {
    throw new Error(`No mock payment attempt found with status: ${status}`)
  }
  return attempt
}

/**
 * Helper function to get mock packages by enabled status
 */
export function getMockPackagesByEnabled(enabled: boolean): PointsPackageResponse[] {
  return mockPointsPackages.filter((pkg) => pkg.enabled === enabled)
}

/**
 * Helper function to get mock payment attempt by ID
 */
export function getMockPaymentAttemptById(attemptId: string): PaymentAttemptStatusResponse {
  const attempt = Object.values(mockPaymentAttempts).find((a) => a.id === attemptId)
  if (!attempt) {
    throw new Error(`No mock payment attempt found with ID: ${attemptId}`)
  }
  return attempt
}

/**
 * Factory for PaymentContextDto test data
 */
export function makePaymentContext(overrides?: Partial<PaymentContextDto>): PaymentContextDto {
  return {
    wechatCodeUrl: 'weixin://wxpay/test',
    stripeCheckoutUrl: null,
    creemCheckoutUrl: null,
    clientSecret: null,
    ...overrides,
  }
}
