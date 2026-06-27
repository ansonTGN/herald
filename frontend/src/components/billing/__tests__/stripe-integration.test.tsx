import { describe, test, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { StripeCheckoutButton } from '../stripe-checkout-button'
import { useStripeCheckout } from '@/hooks/use-stripe-checkout'

vi.mock('@/hooks/use-stripe-checkout')

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}))

describe('Stripe Integration', () => {
  let queryClient: QueryClient

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

  test('renders checkout button with mapping id', () => {
    render(
      <StripeCheckoutButton realmId="test-realm" clientAppId="test-app" mappingId="pro-map-1" />,
      { wrapper }
    )

    expect(screen.getByTestId('stripe-checkout-button-pro-map-1')).toBeInTheDocument()
  })

  test('calls mutation with correct payload on click', async () => {
    const user = userEvent.setup()
    const mutateAsync = vi
      .fn()
      .mockResolvedValue({ checkoutUrl: 'https://checkout.stripe.com/pay' })

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
      <StripeCheckoutButton realmId="test-realm" clientAppId="test-app" mappingId="pro-map-1" />,
      { wrapper }
    )

    await user.click(screen.getByTestId('stripe-checkout-button-pro-map-1'))

    await waitFor(() => {
      expect(mutateAsync).toHaveBeenCalledWith({
        realmId: 'test-realm',
        clientAppId: 'test-app',
        mappingId: 'pro-map-1',
        paymentProvider: 'stripe',
      })
    })
  })
})
