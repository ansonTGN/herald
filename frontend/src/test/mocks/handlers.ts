import { http, HttpResponse } from 'msw'
import type {
  SingleSubscriptionHistoryResponse,
  GlobalSubscriptionHistoryResponse,
  SubscriptionHistoryEvent,
  SubscriptionHistoryEventWithUser,
} from '@/types/billing'
import { stripeHandlers } from './handlers/stripe'
import { pointsHandlers } from './handlers/points'
import { wechatPayHandlers } from './handlers/wechat'
import { unifiedPurchaseHandlers } from './handlers/unified-purchase'
import { auditHandlers } from './handlers/audit'
import { deviceHandlers } from './handlers/device'

// Add API handlers here and override with `server.use(...)` in specific tests when needed.
export const handlers = [
  // Stripe handlers
  ...stripeHandlers,

  // Points handlers
  ...pointsHandlers,

  // WeChat Pay handlers
  ...wechatPayHandlers,

  // Unified Purchase handlers
  ...unifiedPurchaseHandlers,

  // Audit handlers
  ...auditHandlers,

  // Device code handlers
  ...deviceHandlers,
  http.get('/__msw_health__', () => {
    return new Response(null, { status: 204 })
  }),

  // Subscription History Handlers
  http.get('/api/bill/:realmId/subscriptions/:subscriptionId/history', ({ params }) => {
    const { realmId, subscriptionId } = params

    const mockEvents: SubscriptionHistoryEvent[] = [
      {
        id: 'evt-1',
        subscriptionId: subscriptionId as string,
        eventType: 'created',
        timestamp: '2025-01-01T09:00:00Z',
        actor: 'user@example.com',
        previousState: {
          id: subscriptionId as string,
          realmId: realmId as string,
          status: 'active',
          tier: 'basic',
          planId: 'basic-plan',
          clientAppId: 'app-1',
          billingPeriod: 'monthly',
          cancelAtPeriodEnd: false,
        },
      },
      {
        id: 'evt-2',
        subscriptionId: subscriptionId as string,
        eventType: 'upgraded',
        timestamp: '2025-01-20T10:30:00Z',
        actor: 'user@example.com',
        changes: {
          changedFields: ['planId', 'tier'],
          previousPlanId: 'basic-plan',
          newPlanId: 'pro-plan',
          previousTier: 'basic',
          newTier: 'pro',
        },
        previousState: {
          id: subscriptionId as string,
          realmId: realmId as string,
          status: 'active',
          tier: 'basic',
          planId: 'basic-plan',
          clientAppId: 'app-1',
          billingPeriod: 'monthly',
          cancelAtPeriodEnd: false,
        },
        newState: {
          id: subscriptionId as string,
          realmId: realmId as string,
          status: 'active',
          tier: 'pro',
          planId: 'pro-plan',
          clientAppId: 'app-1',
          billingPeriod: 'monthly',
          cancelAtPeriodEnd: false,
        },
      },
      {
        id: 'evt-3',
        subscriptionId: subscriptionId as string,
        eventType: 'canceled',
        timestamp: '2025-02-15T14:00:00Z',
        actor: 'user@example.com',
        changes: {
          changedFields: ['status'],
          previousStatus: 'active',
          newStatus: 'canceled',
        },
        previousState: {
          id: subscriptionId as string,
          realmId: realmId as string,
          status: 'active',
          tier: 'pro',
          planId: 'pro-plan',
          clientAppId: 'app-1',
          billingPeriod: 'monthly',
          cancelAtPeriodEnd: false,
        },
        newState: {
          id: subscriptionId as string,
          realmId: realmId as string,
          status: 'canceled',
          tier: 'pro',
          planId: 'pro-plan',
          clientAppId: 'app-1',
          billingPeriod: 'monthly',
          cancelAtPeriodEnd: true,
          cancelAt: '2025-03-15T00:00:00Z',
        },
      },
    ]

    const response: SingleSubscriptionHistoryResponse = {
      subscriptionId: subscriptionId as string,
      events: mockEvents,
      total: mockEvents.length,
    }

    return HttpResponse.json(response)
  }),

  http.get('/api/bill/:realmId/subscriptions/history', ({ request, params }) => {
    const url = new URL(request.url)
    const eventType = url.searchParams.get('eventType') as any
    const userId = url.searchParams.get('userId')
    const planId = url.searchParams.get('planId')
    const subscriptionStatus = url.searchParams.get('subscriptionStatus')
    const page = parseInt(url.searchParams.get('page') || '1')
    const pageSize = parseInt(url.searchParams.get('pageSize') || '20')

    const mockEvents: SubscriptionHistoryEventWithUser[] = [
      {
        id: 'evt-1',
        subscriptionId: 'sub-1',
        eventType: eventType || 'created',
        timestamp: '2025-01-01T09:00:00Z',
        actor: 'user@example.com',
        user: {
          id: 'user-1',
          email: 'user@example.com',
        },
        subscription: {
          id: 'sub-1',
          status: 'active',
          plan: {
            id: 'basic-plan',
            title: 'Basic Plan',
            price: 9.99,
            currency: 'USD',
            interval: 'monthly',
            tier: 'basic',
          },
        },
      },
      {
        id: 'evt-2',
        subscriptionId: 'sub-2',
        eventType: 'upgraded',
        timestamp: '2025-01-20T10:30:00Z',
        actor: 'user2@example.com',
        user: {
          id: 'user-2',
          email: 'user2@example.com',
        },
        subscription: {
          id: 'sub-2',
          status: 'active',
          plan: {
            id: 'pro-plan',
            title: 'Pro Plan',
            price: 29.99,
            currency: 'USD',
            interval: 'monthly',
            tier: 'pro',
          },
        },
      },
    ]

    // Apply filters
    let filteredEvents = mockEvents
    if (eventType) {
      filteredEvents = filteredEvents.filter((e) => e.eventType === eventType)
    }
    if (userId) {
      filteredEvents = filteredEvents.filter((e) => e.user?.id === userId)
    }
    if (planId) {
      filteredEvents = filteredEvents.filter((e) => e.subscription.plan?.id === planId)
    }
    if (subscriptionStatus) {
      filteredEvents = filteredEvents.filter((e) => e.subscription.status === subscriptionStatus)
    }

    const response: GlobalSubscriptionHistoryResponse = {
      events: filteredEvents,
      pagination: {
        page,
        pageSize,
        total: filteredEvents.length,
        totalPages: Math.ceil(filteredEvents.length / pageSize),
      },
    }

    return HttpResponse.json(response)
  }),
]
