import type { LocalPointsPlanConfig } from '@/types/points-plan-config'

export const mockPointsPlanConfig: LocalPointsPlanConfig = {
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

export const mockPointsPlanConfigWithoutMax: LocalPointsPlanConfig = {
  ...mockPointsPlanConfig,
  configId: 'cfg-456',
  planId: 'plan-456',
  maxPeriods: null,
}

export const mockPointsPlanConfigWeekly: LocalPointsPlanConfig = {
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
  { id: 'plan-123', name: 'basic-monthly', title: 'Basic Monthly' },
  { id: 'plan-456', name: 'pro-yearly', title: 'Pro Yearly' },
]
