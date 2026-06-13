/**
 * Test fixtures for realm default configuration and free user statistics
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

export interface FreeUserStatistics {
  totalFreeUsers: number
  activeFreeUsers: number
  totalRegistrationBonusGranted: number
  totalPeriodicPointsGranted: number
  averagePeriodicPointsPerUser: number
  upgradeRate: number
  lastUpdatedAt: string
}

export const mockFreeUserStatistics: FreeUserStatistics = {
  totalFreeUsers: 1000,
  activeFreeUsers: 800,
  totalRegistrationBonusGranted: 1000000,
  totalPeriodicPointsGranted: 40000,
  averagePeriodicPointsPerUser: 50,
  upgradeRate: 0.15,
  lastUpdatedAt: '2026-03-23T15:30:00Z',
}

export const mockFreeUserStatisticsWithZeroUsers: FreeUserStatistics = {
  ...mockFreeUserStatistics,
  totalFreeUsers: 0,
  activeFreeUsers: 0,
  totalRegistrationBonusGranted: 0,
  totalPeriodicPointsGranted: 0,
  averagePeriodicPointsPerUser: 0,
  upgradeRate: 0,
}

export const mockFreeUserStatisticsWithHighUpgradeRate: FreeUserStatistics = {
  ...mockFreeUserStatistics,
  upgradeRate: 0.1537, // 15.37%
}

export const mockFreeUserStatisticsWithPartialData: FreeUserStatistics = {
  totalFreeUsers: 1000,
  activeFreeUsers: null as unknown as number,
  totalRegistrationBonusGranted: 1000000,
  totalPeriodicPointsGranted: null as unknown as number,
  averagePeriodicPointsPerUser: null as unknown as number,
  upgradeRate: 0.15,
  lastUpdatedAt: '2026-03-23T15:30:00Z',
}
