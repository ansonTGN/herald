import { describe, test, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { StripeCheckoutButton } from '../stripe-checkout-button'
import { useStripeCheckout } from '@/hooks/use-stripe-checkout'
import type { PlanResponse } from '@/lib/api-generated'

vi.mock('@/hooks/use-stripe-checkout')

describe('Stripe Integration', () => {
  let queryClient: QueryClient

  const mockStripePlan: PlanResponse = {
    id: 'plan-stripe-123',
    name: 'pro-monthly',
    title: 'Pro Monthly',
    description: 'Pro plan with Stripe',
    type: 'monthly',
    price: 2000,
    currency: 'USD',
    realmId: 'realm-1',
    productId: 'product-1',
    sortOrder: 1,
    active: true,
    trialDays: 0,
    checkoutUrl: null,
    paymentProviders: [{ id: 'pp-1', paymentProvider: 'stripe', enabled: true }],
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-01-01T00:00:00Z',
  }

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        mutations: { retry: false },
        queries: { retry: false },
      },
    })
    vi.clearAllMocks()

    vi.mocked(useStripeCheckout).mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue({ checkoutUrl: 'https://checkout.stripe.com/pay' }),
      mutate: vi.fn(),
      isPending: false,
      isSuccess: false,
      isError: false,
      error: null,
      data: null,
      reset: vi.fn(),
    })
  })

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  )

  test('does not show checkout button when Stripe provider is disabled', () => {
    const disabledPlan: PlanResponse = {
      ...mockStripePlan,
      paymentProviders: [{ id: 'pp-1', paymentProvider: 'stripe', enabled: false }],
    }

    render(
      <StripeCheckoutButton realmId="test-realm" clientAppId="test-app" plan={disabledPlan} />,
      { wrapper }
    )

    const button = screen.queryByTestId('plan-stripe-checkout-button-plan-stripe-123')
    expect(button).not.toBeInTheDocument()
  })
})
