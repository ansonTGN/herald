import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import type { SubscriptionListItemResponse } from '@/lib/api-generated'

// --- Mocks ----------------------------------------------------------------

// Query options: canned subscription list driven by a mutable holder so each
// test seeds its own rows before render (same queryFn-returns-data pattern the
// entitlement-mappings-page test uses). The list page is read-only — no
// mutations to mock.
const { subscriptionsHolder } = vi.hoisted(() => ({
  subscriptionsHolder: {
    current: { items: [] as SubscriptionListItemResponse[], total: 0 },
  },
}))

vi.mock('@/data/query-options', () => ({
  subscriptionsQueryOptions: () => ({
    queryKey: ['admin-subscriptions', 'realm-1'],
    queryFn: async () => subscriptionsHolder.current,
  }),
}))

import { AdminSubscriptionListPage } from '../admin-subscription-list-page'
import { m } from '@/paraglide/messages'

// --- Fixtures --------------------------------------------------------------

/**
 * Build a single subscription list row. The id is test-supplied so the
 * data-testid suffixes (`billing-type-{id}` / `service-period-end-{id}`) are
 * stable and predictable.
 */
function makeSubscription(
  overrides: Partial<SubscriptionListItemResponse> & { id: string }
): SubscriptionListItemResponse {
  return {
    id: overrides.id,
    entitlementKey: 'pro-plan',
    paymentProvider: 'stripe',
    externalPriceId: null,
    syncedAt: '2026-01-01T00:00:00Z',
    billingType: 'recurring',
    status: 'active',
    currentPeriodStart: null,
    currentPeriodEnd: null,
    clientAppId: null,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    ...overrides,
  } as SubscriptionListItemResponse
}

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
}

function makeWrapper(client: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>
  }
}

function renderPage(items: SubscriptionListItemResponse[]) {
  const client = makeQueryClient()
  subscriptionsHolder.current = { items, total: items.length }
  const wrapper = makeWrapper(client)
  return render(<AdminSubscriptionListPage realmId="realm-1" search={{}} />, { wrapper })
}

beforeEach(() => {
  vi.clearAllMocks()
  subscriptionsHolder.current = { items: [], total: 0 }
})

// --- Tests -----------------------------------------------------------------

describe('AdminSubscriptionListPage — billing type column', () => {
  // DEC-pay_model-007: the subscription list reads `billingType` from the list
  // snapshot column to render the billing-type label and the non-renewing
  // service-period-end. These branches are list-render contracts the Demo does
  // not isolate, so Vitest pins the three-branch behavior.

  it('renders the recurring billing-type label for a recurring subscription', async () => {
    renderPage([makeSubscription({ id: 'sub-recurring', billingType: 'recurring' })])

    const cell = await screen.findByTestId('billing-type-sub-recurring')
    // The localized recurring label (not a hardcoded en string).
    expect(cell.textContent).toContain(String(m['billing.billing_type_recurring']()))
    // Recurring has no special end-of-service semantics: the service-period-end
    // cell renders the '---' placeholder (no date), so its text does NOT carry
    // a formatted date fragment.
    const periodCell = screen.getByTestId('service-period-end-sub-recurring')
    expect(periodCell.textContent).toBe('---')
  })

  it('renders the non_renewing label and service-period-end for a non_renewing subscription (DEC-pay_model-007)', async () => {
    renderPage([
      makeSubscription({
        id: 'sub-nr',
        billingType: 'non_renewing',
        currentPeriodEnd: '2026-08-31T00:00:00Z',
      }),
    ])

    const typeCell = await screen.findByTestId('billing-type-sub-nr')
    expect(typeCell.textContent).toContain(String(m['billing.billing_type_non_renewing']()))

    // The non-renewing service-period-end snapshot column drives a real date
    // render (DEC-pay_model-007). Assert a date fragment (year + month) rather
    // than the full locale-specific string so the assertion is locale-stable.
    const periodCell = screen.getByTestId('service-period-end-sub-nr')
    expect(periodCell.textContent).toContain('2026')
    expect(periodCell.textContent).toMatch(/Aug/i)
  })

  it('renders the raw billingType as a fallback for an unknown value', async () => {
    // An unrecognized billing-type string falls through `formatBillingTypeLabel`
    // verbatim (graceful fallback). The column must surface the raw value, not
    // a blank, so a future billing type is still visible to the admin.
    renderPage([
      makeSubscription({
        id: 'sub-future',
        billingType: 'future_type' as unknown as string,
      }),
    ])

    const cell = await screen.findByTestId('billing-type-sub-future')
    expect(cell.textContent).toContain('future_type')
    // Unknown type has no special end-of-service semantics → placeholder.
    expect(screen.getByTestId('service-period-end-sub-future').textContent).toBe('---')
  })
})
