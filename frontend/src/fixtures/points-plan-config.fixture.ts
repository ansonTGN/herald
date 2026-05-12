import type { PointsPlanConfigResponse } from '@/lib/api-generated'

export const mockPointsPlanConfig: PointsPlanConfigResponse = {
  configId: 'cfg-123',
  planId: 'plan-123',
  realmId: 'realm-123',
  pointsPerPeriod: 1000,
  grantOnSubscribe: true,
  grantPeriodType: 'monthly',
  maxPeriods: 12,
  validityDays: 30,
  active: true,
  createdAt: '2025-01-01T00:00:00Z',
  updatedAt: '2025-03-15T00:00:00Z',
}

export const mockPointsPlanConfigWithoutMax: PointsPlanConfigResponse = {
  ...mockPointsPlanConfig,
  configId: 'cfg-456',
  planId: 'plan-456',
  maxPeriods: null,
}

export const mockPointsPlanConfigWeekly: PointsPlanConfigResponse = {
  ...mockPointsPlanConfig,
  configId: 'cfg-789',
  planId: 'plan-789',
  grantPeriodType: 'weekly',
  pointsPerPeriod: 250,
}

export const mockPlanConfigsList = [
  mockPointsPlanConfig,
  mockPointsPlanConfigWithoutMax,
  mockPointsPlanConfigWeekly,
]

export const mockPlansList = [
  {
    id: 'plan-123',
    name: 'basic-monthly',
    title: 'Basic Monthly',
    type: 'monthly',
    price: 1000,
    currency: 'USD',
    paymentProvider: 'creem',
    externalProductId: 'prod_basic_monthly',
    active: true,
    trialDays: 0,
    sortOrder: 0,
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-03-15T00:00:00Z',
    realmId: 'realm-123',
  },
  {
    id: 'plan-456',
    name: 'pro-yearly',
    title: 'Pro Yearly',
    type: 'yearly',
    price: 10000,
    currency: 'USD',
    paymentProvider: 'creem',
    externalProductId: 'prod_pro_yearly',
    active: true,
    trialDays: 0,
    sortOrder: 1,
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-03-15T00:00:00Z',
    realmId: 'realm-123',
  },
]
