/**
 * Billing and Subscription Types
 *
 * Type definitions for billing-related functionality including subscription history.
 */

// Subscription history event types
export type SubscriptionHistoryEventType =
  | 'created'
  | 'upgraded'
  | 'downgraded'
  | 'canceled'
  | 'expired'
  | 'renewed'
  | 'reactivated'
  | 'billing_period_changed'
  | 'past_due'
  | 'disputed'
  | 'payment_succeeded'
  | 'payment_failed'
  | 'invoice_created'

// Subscription status
export type SubscriptionStatus =
  | 'active'
  | 'trialing'
  | 'past_due'
  | 'canceled'
  | 'incomplete'
  | 'expired'
  | 'paused'
  | 'disputed'
  | 'scheduled_cancel'

// Subscription history event
export interface SubscriptionHistoryEvent {
  id: string
  subscriptionId: string
  eventType: SubscriptionHistoryEventType
  timestamp: string
  actor?: string
  changes?: SubscriptionChanges
  previousState?: SubscriptionState
  newState?: SubscriptionState
}

// Change details for subscription modifications
export interface SubscriptionChanges {
  changedFields?: string[]
  previousPlanId?: string
  newPlanId?: string
  previousStatus?: string
  newStatus?: string
  previousTier?: string
  newTier?: string
  previousBillingPeriod?: string
  newBillingPeriod?: string
}

// Subscription state snapshot
export interface SubscriptionState {
  id: string
  realmId: string
  status: SubscriptionStatus
  tier: string
  planId?: string
  clientAppId?: string
  billingPeriod: string
  currentPeriodStart?: string
  currentPeriodEnd?: string
  cancelAtPeriodEnd: boolean
  cancelAt?: string
}

// User information in subscription history
export interface UserInfo {
  id: string
  email: string
}

// Plan summary in subscription history
export interface PlanSummary {
  id: string
  title: string
  description?: string
  price?: number
  currency?: string
  interval?: string
  tier?: string
}

// Subscription summary in history
export interface SubscriptionSummary {
  id: string
  status: SubscriptionStatus
  plan?: PlanSummary
}

// Subscription history event with user and subscription details
export interface SubscriptionHistoryEventWithUser extends SubscriptionHistoryEvent {
  user?: UserInfo
  subscription: SubscriptionSummary
}

// Filter conditions for history queries
export interface HistoryFilters {
  userId?: string
  planId?: string
  eventType?: SubscriptionHistoryEventType
  subscriptionStatus?: SubscriptionStatus
  fromDate?: string
  toDate?: string
  sortBy?: string
  sortOrder?: 'asc' | 'desc'
}

// Pagination information
export interface PaginationInfo {
  page: number
  pageSize: number
  totalCount: number
  totalPages: number
}

// Single subscription history response
export interface SingleSubscriptionHistoryResponse {
  subscriptionId: string
  events: SubscriptionHistoryEvent[]
  total: number
}

// Global subscription history response
export interface GlobalSubscriptionHistoryResponse {
  events: SubscriptionHistoryEventWithUser[]
  pagination: PaginationInfo
}

// Event type labels for display
export const EventTypeLabels: Readonly<Record<SubscriptionHistoryEventType, string>> =
  Object.freeze({
    created: 'Created',
    upgraded: 'Upgraded',
    downgraded: 'Downgraded',
    canceled: 'Canceled',
    expired: 'Expired',
    renewed: 'Renewed',
    reactivated: 'Reactivated',
    billing_period_changed: 'Billing Period Changed',
    past_due: 'Payment Failed',
    disputed: 'Dispute Started',
    payment_succeeded: 'Payment Succeeded',
    payment_failed: 'Payment Failed',
    invoice_created: 'Invoice Created',
  } as const)

// Status labels for display
export const SubscriptionStatusLabels: Readonly<Record<SubscriptionStatus, string>> = Object.freeze(
  {
    active: 'Active',
    trialing: 'Trialing',
    past_due: 'Past Due',
    canceled: 'Canceled',
    incomplete: 'Incomplete',
    expired: 'Expired',
    paused: 'Paused',
    disputed: 'Disputed',
    scheduled_cancel: 'Scheduled to Cancel',
  } as const
)

/**
 * Returns the badge variant for a subscription status
 * @param status - The subscription status
 * @returns The badge variant ('default', 'secondary', or 'destructive')
 */
export function getStatusBadgeVariant(
  status: SubscriptionStatus
): 'default' | 'secondary' | 'destructive' {
  switch (status) {
    case 'active':
    case 'trialing':
      return 'default'
    case 'past_due':
    case 'disputed':
    case 'paused':
      return 'destructive'
    default:
      return 'secondary'
  }
}

/**
 * Returns a user-friendly message for a subscription status
 * @param status - The subscription status
 * @returns The status message or empty string if no message
 */
export function getStatusMessage(status: SubscriptionStatus): string {
  switch (status) {
    case 'past_due':
      return 'Payment failed - please update payment method'
    case 'disputed':
      return 'Dispute in progress - contact support'
    case 'scheduled_cancel':
      return 'Will cancel at period end'
    case 'paused':
      return 'Subscription paused'
    default:
      return ''
  }
}
