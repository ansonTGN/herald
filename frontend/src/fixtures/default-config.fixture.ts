/**
 * Test fixtures for realm default configuration
 */

export interface RealmDefaultConfig {
  realmId: string
  registrationBonusPoints: number
  freePeriodicPointsAmount: number
  freePeriodicGrantPeriodType: 'once' | 'daily' | 'weekly' | 'monthly'
  freePeriodicValidityDays: number
  createdAt: string
  updatedAt: string
}

export const mockRealmDefaultConfig: RealmDefaultConfig = {
  realmId: 'test-realm',
  registrationBonusPoints: 1000,
  freePeriodicPointsAmount: 50,
  freePeriodicGrantPeriodType: 'daily',
  freePeriodicValidityDays: 1,
  createdAt: '2026-03-23T00:00:00Z',
  updatedAt: '2026-03-23T00:00:00Z',
}

export const mockRealmConfigWithOncePeriod: RealmDefaultConfig = {
  ...mockRealmDefaultConfig,
  freePeriodicGrantPeriodType: 'once',
  freePeriodicValidityDays: 0, // once allows 0 (permanent validity)
}

export const mockRealmConfigWithWeeklyPeriod: RealmDefaultConfig = {
  ...mockRealmDefaultConfig,
  freePeriodicGrantPeriodType: 'weekly',
  freePeriodicValidityDays: 7,
}

export const mockRealmConfigWithMonthlyPeriod: RealmDefaultConfig = {
  ...mockRealmDefaultConfig,
  freePeriodicGrantPeriodType: 'monthly',
  freePeriodicValidityDays: 30,
}

export const mockRealmConfigWithNegativeBonus: RealmDefaultConfig = {
  ...mockRealmDefaultConfig,
  registrationBonusPoints: -100,
}

export const mockRealmConfigWithLargeValues: RealmDefaultConfig = {
  ...mockRealmDefaultConfig,
  registrationBonusPoints: 1000000,
  freePeriodicPointsAmount: 10000,
  freePeriodicValidityDays: 365,
}
