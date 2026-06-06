// TODO: Points plan config types should be migrated to entitlement-based config.
export interface LocalPointsPlanConfig {
  configId: string
  realmId: string
  planId: string
  pointsPerPeriod: number
  grantOnSubscribe: boolean
  grantPeriodType: string
  maxPeriods: number | null
  validityDays: number
  active: boolean
  createdAt: string
  updatedAt: string
}
