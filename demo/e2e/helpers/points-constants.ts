/**
 * Points System Constants
 *
 * Shared constants for Points System demo tests
 */

// ============================================================================
// Transaction Types
// ============================================================================

export const TRANSACTION_TYPES = {
  RECHARGE: 'recharge',
  CONSUME: 'consume',
} as const

export type TransactionType = typeof TRANSACTION_TYPES[keyof typeof TRANSACTION_TYPES]

// ============================================================================
// Renewal Period Types
// ============================================================================

export const RENEWAL_PERIOD_TYPES = {
  MONTHLY: 'monthly',
  YEARLY: 'yearly',
} as const

export type RenewalPeriodType = typeof RENEWAL_PERIOD_TYPES[keyof typeof RENEWAL_PERIOD_TYPES]

// ============================================================================
// Wait Times (in milliseconds)
// ============================================================================

export const WAIT_TIMES = {
  DEBOUNCE: 500,
  EXPORT: 1000,
  FILTER_APPLY: 500,
} as const

// ============================================================================
// Defaults
// ============================================================================

export const PLACEHOLDER_USER_ID = 'placeholder-user-id'
