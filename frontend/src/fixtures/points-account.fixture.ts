import type { PointsAccountResponse } from '@/lib/api-generated'

export const mockPointsAccount: PointsAccountResponse = {
  id: 'acc-123',
  userId: 'user-123',
  realmId: 'realm-123',
  balance: 5000,
  currency: 'points',
  unit: 'points',
  status: 'active',
  totalRecharged: 10000,
  totalConsumed: 5000,
  createdAt: '2025-01-01T00:00:00Z',
  updatedAt: '2025-03-15T00:00:00Z',
}

export const mockPointsAccountFrozen: PointsAccountResponse = {
  ...mockPointsAccount,
  id: 'acc-456',
  userId: 'user-456',
  status: 'frozen',
  balance: 2500,
}

export const mockPointsAccountClosed: PointsAccountResponse = {
  ...mockPointsAccount,
  id: 'acc-789',
  userId: 'user-789',
  status: 'closed',
  balance: 0,
  totalRecharged: 1000,
  totalConsumed: 1000,
}

export const mockPointsAccountsList = [
  mockPointsAccount,
  mockPointsAccountFrozen,
  mockPointsAccountClosed,
]
