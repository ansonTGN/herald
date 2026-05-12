import { describe, test, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { StripeCheckoutButton } from '../stripe-checkout-button'
import { useStripeCheckout } from '@/hooks/use-stripe-checkout'
import type { PlanResponse } from '@/lib/api-generated'

// Mock the hook
vi.mock('@/hooks/use-stripe-checkout')

// Mock toast
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}))

describe('StripeCheckoutButton', () => {
  let queryClient: QueryClient

  const mockPlan: PlanResponse = {
    id: 'plan-123',
    name: 'pro-monthly',
    title: 'Pro Plan',
    description: 'Pro plan description',
    type: 'monthly',
    price: 2999,
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

  const mockCheckoutUrl = 'https://checkout.stripe.com/pay/cs_test_123'

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        mutations: { retry: false },
        queries: { retry: false },
      },
    })
    vi.clearAllMocks()

    // Default mock for successful checkout
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

  describe('rendering', () => {
    test('renders button correctly', () => {
      render(<StripeCheckoutButton realmId="realm1" clientAppId="app1" plan={mockPlan} />, {
        wrapper,
      })

      const button = screen.getByTestId('plan-stripe-checkout-button-plan-123')
      expect(button).toBeInTheDocument()
      expect(button).toHaveTextContent('Subscribe with Stripe')
    })

    test('shows loading state when checkout is pending', () => {
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

      render(<StripeCheckoutButton realmId="realm1" clientAppId="app1" plan={mockPlan} />, {
        wrapper,
      })

      const button = screen.getByTestId('plan-stripe-checkout-button-plan-123')
      expect(button).toBeDisabled()
      expect(button).toHaveTextContent('Creating...')
    })

    test('is disabled when mutation is pending', () => {
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

      render(<StripeCheckoutButton realmId="realm1" clientAppId="app1" plan={mockPlan} />, {
        wrapper,
      })

      const button = screen.getByTestId('plan-stripe-checkout-button-plan-123')
      expect(button).toBeDisabled()
    })
  })

  describe('user interactions', () => {
    test('initiates checkout when clicked', async () => {
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

      render(<StripeCheckoutButton realmId="realm1" clientAppId="app1" plan={mockPlan} />, {
        wrapper,
      })

      const button = screen.getByTestId('plan-stripe-checkout-button-plan-123')
      await user.click(button)

      await waitFor(() => {
        expect(mutateAsync).toHaveBeenCalledWith({
          realmId: 'realm1',
          clientAppId: 'app1',
          planId: 'plan-123',
          paymentProvider: 'stripe',
          billingPeriod: 'monthly',
        })
      })
    })

    test('does not render for non-Stripe plans', async () => {
      const nonStripePlan: PlanResponse = {
        ...mockPlan,
        id: 'plan-creem',
        paymentProviders: [{ id: 'pp-2', paymentProvider: 'creem', enabled: true }],
      }

      render(<StripeCheckoutButton realmId="realm1" clientAppId="app1" plan={nonStripePlan} />, {
        wrapper,
      })

      const button = screen.queryByTestId('plan-stripe-checkout-button-plan-creem')
      expect(button).not.toBeInTheDocument()
    })

    test('handles checkout errors gracefully', async () => {
      const user = userEvent.setup()
      const { toast } = await import('sonner')

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

      render(<StripeCheckoutButton realmId="realm1" clientAppId="app1" plan={mockPlan} />, {
        wrapper,
      })

      const button = screen.getByTestId('plan-stripe-checkout-button-plan-123')
      await user.click(button)

      await waitFor(() => {
        expect(mutateAsync).toHaveBeenCalled()
        expect(consoleSpy).toHaveBeenCalledWith('Checkout error:', expect.any(Error))
      })

      consoleSpy.mockRestore()
    })
  })

  describe('billing periods', () => {
    test('uses monthly billing period by default', async () => {
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

      render(<StripeCheckoutButton realmId="realm1" clientAppId="app1" plan={mockPlan} />, {
        wrapper,
      })

      const button = screen.getByTestId('plan-stripe-checkout-button-plan-123')
      await user.click(button)

      await waitFor(() => {
        expect(mutateAsync).toHaveBeenCalledWith({
          realmId: 'realm1',
          clientAppId: 'app1',
          planId: 'plan-123',
          paymentProvider: 'stripe',
          billingPeriod: 'monthly',
        })
      })
    })

    test('uses yearly billing period when specified', async () => {
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
          realmId="realm1"
          clientAppId="app1"
          plan={mockPlan}
          billingPeriod="yearly"
        />,
        { wrapper }
      )

      const button = screen.getByTestId('plan-stripe-checkout-button-plan-123')
      await user.click(button)

      await waitFor(() => {
        expect(mutateAsync).toHaveBeenCalledWith({
          realmId: 'realm1',
          clientAppId: 'app1',
          planId: 'plan-123',
          paymentProvider: 'stripe',
          billingPeriod: 'yearly',
        })
      })
    })
  })

  describe('button variants and sizes', () => {
    test('applies custom variant', () => {
      render(
        <StripeCheckoutButton
          realmId="realm1"
          clientAppId="app1"
          plan={mockPlan}
          variant="outline"
        />,
        { wrapper }
      )

      const button = screen.getByTestId('plan-stripe-checkout-button-plan-123')
      expect(button).toBeInTheDocument()
      // Variant is applied via Button component's variant prop
      expect(button).toHaveAttribute('data-variant', 'outline')
    })

    test('applies custom size', () => {
      render(
        <StripeCheckoutButton realmId="realm1" clientAppId="app1" plan={mockPlan} size="lg" />,
        { wrapper }
      )

      const button = screen.getByTestId('plan-stripe-checkout-button-plan-123')
      expect(button).toBeInTheDocument()
      // Size is applied via Button component's size prop
      expect(button).toHaveAttribute('data-size', 'lg')
    })

    test('applies custom className', () => {
      render(
        <StripeCheckoutButton
          realmId="realm1"
          clientAppId="app1"
          plan={mockPlan}
          className="custom-class"
        />,
        { wrapper }
      )

      const button = screen.getByTestId('plan-stripe-checkout-button-plan-123')
      expect(button).toHaveClass('custom-class')
    })
  })

  describe('success and error callbacks', () => {
    test('shows success toast on successful checkout', async () => {
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
      } as any)

      render(<StripeCheckoutButton realmId="realm1" clientAppId="app1" plan={mockPlan} />, {
        wrapper,
      })

      const button = screen.getByTestId('plan-stripe-checkout-button-plan-123')
      await user.click(button)

      await waitFor(() => {
        expect(mutateAsync).toHaveBeenCalled()
      })
    })
  })
})
