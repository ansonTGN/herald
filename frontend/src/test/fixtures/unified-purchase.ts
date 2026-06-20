/**
 * Test fixtures for unified-purchase feature
 *
 * Provides mock data for payment attempts
 */

import type { PaymentAttemptStatusResponse, PaymentContextDto } from '@/lib/api-generated'

/**
 * Mock payment attempts with different statuses
 */
export const mockPaymentAttempts: Record<string, PaymentAttemptStatusResponse> = {
  pending: {
    id: 'attempt-pending',
    status: 'Pending',
    targetType: 'entitlement_mapping',
    targetId: 'mapping-550e8400-e29b-41d4-a716-446655440001',
    amount: 999,
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
    targetType: 'entitlement_mapping',
    targetId: 'mapping-550e8400-e29b-41d4-a716-446655440002',
    amount: 3999,
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
    targetType: 'entitlement_mapping',
    targetId: 'mapping-550e8400-e29b-41d4-a716-446655440001',
    amount: 999,
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
    targetType: 'entitlement_mapping',
    targetId: 'mapping-550e8400-e29b-41d4-a716-446655440001',
    amount: 999,
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
    targetType: 'entitlement_mapping',
    targetId: 'mapping-550e8400-e29b-41d4-a716-446655440001',
    amount: 999,
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
    targetType: 'entitlement_mapping',
    targetId: 'mapping-550e8400-e29b-41d4-a716-446655440001',
    amount: 999,
    currency: 'USD',
    createdAt: '2025-01-01T00:00:00Z',
    expiresAt: '2025-01-01T02:00:00Z',
    completedAt: null,
    fulfillment: null,
    providerStatus: 'expired',
  },
}

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
    wechatCodeUrl: null,
    stripeCheckoutUrl: null,
    creemCheckoutUrl: null,
    clientSecret: null,
    ...overrides,
  }
}
