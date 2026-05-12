import { describe, test, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { StripeCheckoutButton } from '../stripe-checkout-button'
import { useStripeCheckout } from '@/hooks/use-stripe-checkout'
import type { PlanResponse } from '@/lib/api-generated'

vi.mock('@/hooks/use-stripe-checkout')

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}))

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

  const mockCreemPlan: PlanResponse = {
    ...mockStripePlan,
    id: 'plan-creem-456',
    paymentProviders: [{ id: 'pp-2', paymentProvider: 'creem', enabled: true }],
  }

  const mockCheckoutUrl = 'https://checkout.stripe.com/pay/cs_test_123'

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        mutations: { retry: false },
        queries: { retry: false },
      },
    })
    vi.clearAllMocks()

    vi.mocked(useStripeCheckout).mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue({ checkoutUrl: mockCheckoutUrl }),
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

  test('shows checkout button for Stripe plans', () => {
    render(
      <StripeCheckoutButton realmId="test-realm" clientAppId="test-app" plan={mockStripePlan} />,
      { wrapper }
    )

    const button = screen.getByTestId('plan-stripe-checkout-button-plan-stripe-123')
    expect(button).toBeInTheDocument()
    expect(button).toHaveTextContent('Subscribe with Stripe')
  })

  test('does not show checkout button for non-Stripe plans', () => {
    render(
      <StripeCheckoutButton realmId="test-realm" clientAppId="test-app" plan={mockCreemPlan} />,
      { wrapper }
    )

    const button = screen.queryByTestId('plan-stripe-checkout-button-plan-creem-456')
    expect(button).not.toBeInTheDocument()
  })

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

  test('initiates checkout when button is clicked', async () => {
    const user = userEvent.setup()
    const mutateAsync = vi.fn().mockResolvedValue({ checkoutUrl: mockCheckoutUrl })

    vi.mocked(useStripeCheckout).mockReturnValue({
      mutateAsync,
      mutate: vi.fn(),
      isPending: false,
      isSuccess: false,
      isError: false,
      error: null,
      data: null,
      reset: vi.fn(),
    })

    render(
      <StripeCheckoutButton
        realmId="test-realm"
        clientAppId="test-app"
        plan={mockStripePlan}
        billingPeriod="monthly"
      />,
      { wrapper }
    )

    const button = screen.getByTestId('plan-stripe-checkout-button-plan-stripe-123')
    await user.click(button)

    await waitFor(() => {
      expect(mutateAsync).toHaveBeenCalledWith({
        realmId: 'test-realm',
        clientAppId: 'test-app',
        planId: 'plan-stripe-123',
        paymentProvider: 'stripe',
        billingPeriod: 'monthly',
      })
    })
  })

  test('shows loading state during checkout', () => {
    vi.mocked(useStripeCheckout).mockReturnValue({
      mutateAsync: vi.fn(),
      mutate: vi.fn(),
      isPending: true,
      isSuccess: false,
      isError: false,
      error: null,
      data: null,
      reset: vi.fn(),
    })

    render(
      <StripeCheckoutButton realmId="test-realm" clientAppId="test-app" plan={mockStripePlan} />,
      { wrapper }
    )

    const button = screen.getByTestId('plan-stripe-checkout-button-plan-stripe-123')
    expect(button).toBeDisabled()
    expect(button).toHaveTextContent('Creating...')
  })

  test('handles checkout errors gracefully', async () => {
    const user = userEvent.setup()
    const mutateAsync = vi.fn().mockRejectedValue(new Error('Checkout failed'))

    vi.mocked(useStripeCheckout).mockReturnValue({
      mutateAsync,
      mutate: vi.fn(),
      isPending: false,
      isSuccess: false,
      isError: false,
      error: null,
      data: null,
      reset: vi.fn(),
    } as any)

    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

    render(
      <StripeCheckoutButton realmId="test-realm" clientAppId="test-app" plan={mockStripePlan} />,
      { wrapper }
    )

    const button = screen.getByTestId('plan-stripe-checkout-button-plan-stripe-123')
    await user.click(button)

    await waitFor(() => {
      expect(mutateAsync).toHaveBeenCalled()
      expect(consoleSpy).toHaveBeenCalledWith('Checkout error:', expect.any(Error))
    })

    consoleSpy.mockRestore()
  })
})
