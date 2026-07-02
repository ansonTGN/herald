import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import type { PurchaseOptionView } from '@/lib/api-generated'

// --- Mocks ----------------------------------------------------------------

// The purchase-flow store is a persisted zustand store; stub its actions so the
// page never touches localStorage / cross-test state.
vi.mock('@/stores/purchase-flow-store', () => ({
  usePurchaseFlowActions: () => ({
    setPurchaseState: vi.fn(),
    setPaymentAttempt: vi.fn(),
    clearPurchaseState: vi.fn(),
    canRecover: () => false,
  }),
  usePaymentAttempt: () => ({ attemptId: null, paymentContext: null }),
  usePurchaseFlowStore: () => null,
}))

// Auth store: minimal user identity.
vi.mock('@/stores/auth-store', () => ({
  useAuthStore: (selector: (s: { user: { id: string } | null }) => unknown) =>
    selector({ user: { id: 'user-1' } }),
}))

// Payment attempt API: capture the submitted target so we can assert mappingId.
const { mockCreatePaymentAttempt } = vi.hoisted(() => ({
  mockCreatePaymentAttempt: vi.fn(),
}))
vi.mock('@/lib/api-generated', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/api-generated')>()
  return {
    ...actual,
    createPaymentAttempt: mockCreatePaymentAttempt,
    cancelPaymentAttempt: vi.fn(),
  }
})

// PaymentAttemptStatus / PaymentMethodSelector: stub to avoid their query trees.
vi.mock('@/components/purchase/payment-attempt-status', () => ({
  PaymentAttemptStatus: () => <div data-testid="payment-attempt-status" />,
}))
vi.mock('@/components/purchase/payment-method-selector', () => ({
  PaymentMethodSelector: ({ selectedProvider }: { selectedProvider: string | null }) => (
    <div data-testid="payment-method-selector">{selectedProvider}</div>
  ),
}))

import { PurchasePointsPage, disabledReason } from '../purchase-points'

// --- Fixtures --------------------------------------------------------------

function makeOption(overrides: Partial<PurchaseOptionView>): PurchaseOptionView {
  return {
    mappingId: overrides.mappingId ?? 'map-1',
    externalProductId: 'prod_pro',
    externalPriceId: overrides.externalPriceId ?? 'price_monthly',
    paymentProvider: overrides.paymentProvider ?? 'stripe',
    entitlementKey: overrides.entitlementKey ?? 'pro-plan',
    billingType: overrides.billingType ?? 'recurring',
    billingPeriod: overrides.billingPeriod ?? 'month',
    displayName: overrides.displayName ?? 'Pro',
    amount: overrides.amount ?? 1000,
    currency: overrides.currency ?? 'usd',
    pointsPerPeriod: overrides.pointsPerPeriod ?? 1000,
    enabled: overrides.enabled ?? true,
    ...overrides,
  } as PurchaseOptionView
}

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
}

function seedOptions(client: QueryClient, items: PurchaseOptionView[]) {
  client.setQueryData(['purchase-options', 'realm-1', 'app-1'], items)
  client.setQueryData(['payment-providers', 'realm-1'], [{ platform: 'stripe', name: 'Stripe' }])
}

function makeWrapper(client: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>
  }
}

function renderPage(items: PurchaseOptionView[], client?: QueryClient) {
  const qc = client ?? makeQueryClient()
  seedOptions(qc, items)
  const view = render(<PurchasePointsPage realmId="realm-1" clientAppId="app-1" />, {
    wrapper: makeWrapper(qc),
  })
  return { qc, ...view }
}

beforeEach(() => {
  vi.clearAllMocks()
  // Default: payment attempt creation resolves with an id.
  mockCreatePaymentAttempt.mockResolvedValue({
    data: { id: 'att-1', paymentContext: {}, expiresAt: '2026-01-01T00:00:00Z' },
  })
})

// --- Pure helper tests (pinned contract) ----------------------------------

describe('disabledReason', () => {
  it('returns null for an enabled option with a provider', () => {
    expect(disabledReason(makeOption({ enabled: true, paymentProvider: 'stripe' }))).toBeNull()
  })

  it('returns the not_enabled reason when disabled', () => {
    const reason = disabledReason(makeOption({ enabled: false }))
    expect(reason).toEqual({ key: 'purchase.not_enabled_reason' })
  })

  it('returns the not_enabled reason when no payment provider is wired', () => {
    const reason = disabledReason(makeOption({ paymentProvider: '' }))
    expect(reason).toEqual({ key: 'purchase.not_enabled_reason' })
  })
})

// --- Component tests -------------------------------------------------------

describe('PurchasePointsPage', () => {
  it('renders all recurring cards together with no period toggle', async () => {
    renderPage([
      makeOption({
        mappingId: 'm-month',
        externalPriceId: 'price_monthly',
        billingType: 'recurring',
        billingPeriod: 'month',
      }),
      makeOption({
        mappingId: 'm-year',
        externalPriceId: 'price_annual',
        billingType: 'recurring',
        billingPeriod: 'year',
      }),
    ])

    // Both recurring cards render in a single view — no period tab to toggle.
    expect(screen.getByTestId('purchase-price-card-price_monthly')).toBeTruthy()
    expect(screen.getByTestId('purchase-price-card-price_annual')).toBeTruthy()
    expect(screen.queryByTestId('purchase-period-toggle')).toBeNull()
    expect(screen.queryByTestId('purchase-period-toggle-month')).toBeNull()
    expect(screen.queryByTestId('purchase-period-toggle-year')).toBeNull()
  })

  it('shows a disabled reason and disabled CTA for a not-enabled price', () => {
    renderPage([
      makeOption({
        mappingId: 'm-disabled',
        externalPriceId: 'price_disabled',
        enabled: false,
      }),
    ])

    const card = screen.getByTestId('purchase-price-card-price_disabled')
    expect(card).toBeTruthy()
    expect(screen.getByTestId('purchase-price-card-price_disabled-reason')).toBeTruthy()
    expect(screen.getByTestId('purchase-next-button')).toBeDisabled()
  })

  it('selects a card and submits the mappingId as the purchase target (single provider skips payment step)', async () => {
    const user = userEvent.setup()
    renderPage([
      makeOption({ mappingId: 'm-1', externalPriceId: 'price_m', billingPeriod: 'month' }),
    ])

    await user.click(screen.getByTestId('purchase-price-card-price_m'))
    await waitFor(() => {
      expect(screen.getByTestId('purchase-next-button')).not.toBeDisabled()
    })

    // With a single matching provider the payment-method step is skipped:
    // Next creates the payment attempt directly.
    await user.click(screen.getByTestId('purchase-next-button'))
    expect(screen.queryByTestId('purchase-step-payment')).toBeNull()
    await waitFor(() => {
      expect(mockCreatePaymentAttempt).toHaveBeenCalledTimes(1)
    })
    const call = mockCreatePaymentAttempt.mock.calls[0][0] as {
      body: { targetType: string; targetId: string; paymentProvider: string }
    }
    expect(call.body.targetType).toBe('entitlement_mapping')
    expect(call.body.targetId).toBe('m-1')
    expect(call.body.paymentProvider).toBe('stripe')
  })
})
