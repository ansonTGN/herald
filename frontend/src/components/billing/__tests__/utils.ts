import React from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import type { SubscriptionHistoryEvent } from '@/types/billing'
import type { SubscriptionStatus } from '@/types/billing'

/**
 * Creates a test QueryClient wrapper for React Query hooks testing
 */
export function createQueryClientWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        staleTime: Infinity,
        gcTime: 0,
      },
      mutations: {
        retry: false,
      },
    },
  })

  return function QueryWrapper({ children }: { children: ReactNode }) {
    return React.createElement(QueryClientProvider, { client: queryClient }, children)
  }
}

/**
 * Creates a mock subscription history event with optional overrides
 */
export function mockSubscriptionHistoryEvent(
  overrides: Partial<SubscriptionHistoryEvent> = {}
): SubscriptionHistoryEvent {
  return {
    id: 'evt-1',
    subscriptionId: 'sub-1',
    eventType: 'created',
    timestamp: '2025-01-20T10:30:00Z',
    actor: 'user@example.com',
    previousState: {
      id: 'sub-1',
      realmId: 'realm-1',
      status: 'active',
      entitlementKey: 'basic',
      paymentProvider: 'stripe',
      cancelAtPeriodEnd: false,
    },
    ...overrides,
  }
}

/**
 * Creates a mock subscription state
 */
export function mockSubscriptionState(
  overrides: Partial<SubscriptionState> = {}
): SubscriptionState {
  return {
    id: 'sub-1',
    realmId: 'realm-1',
    status: 'active',
    entitlementKey: 'basic',
    cancelAtPeriodEnd: false,
    ...overrides,
  }
}

/**
 * Type for SubscriptionState test mock (re-export for test convenience)
 */
export type SubscriptionState = {
  id: string
  realmId: string
  status: SubscriptionStatus
  entitlementKey: string
  paymentProvider?: string
  externalPriceId?: string
  clientAppId?: string
  currentPeriodStart?: string
  currentPeriodEnd?: string
  cancelAtPeriodEnd: boolean
  cancelAt?: string
}
