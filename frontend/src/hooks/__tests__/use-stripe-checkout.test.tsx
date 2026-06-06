import { describe, test, expect, vi, beforeEach } from 'vitest'
import { renderHook, waitFor, act } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useStripeCheckout } from '../use-stripe-checkout'
import { createCheckoutSession } from '@/lib/api-generated'

// Mock the API modules
vi.mock('@/lib/api-generated', () => ({
  createCheckoutSession: vi.fn(),
}))

// Mock window.location.href
const mockLocation = { href: '' }
Object.defineProperty(window, 'location', {
  value: mockLocation,
  writable: true,
})

describe('useStripeCheckout', () => {
  const mockRealmId = 'test-realm'
  const mockClientAppId = 'test-app'
  const mockEntitlementKey = 'basic'
  const mockPaymentProvider = 'stripe'
  const mockCheckoutUrl = 'https://checkout.stripe.com/pay/test'

  let queryClient: QueryClient

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        mutations: {
          retry: false,
        },
      },
    })

    // Reset mocks
    vi.mocked(createCheckoutSession).mockReset()
    mockLocation.href = ''
  })

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  )

  test('creates checkout session and redirects on success', async () => {
    const mockResponse = {
      data: { checkoutUrl: mockCheckoutUrl },
      error: undefined,
    }

    vi.mocked(createCheckoutSession).mockResolvedValue(mockResponse as any)

    const onSuccess = vi.fn()
    const { result } = renderHook(() => useStripeCheckout({ onSuccess }), { wrapper })

    result.current.mutate({
      realmId: mockRealmId,
      clientAppId: mockClientAppId,
      entitlementKey: mockEntitlementKey,
      paymentProvider: mockPaymentProvider,
    })

    await waitFor(() => {
      expect(createCheckoutSession).toHaveBeenCalledWith({
        path: { realmId: mockRealmId, clientAppId: mockClientAppId },
        body: { entitlementKey: mockEntitlementKey, paymentProvider: mockPaymentProvider },
      })
    })

    await waitFor(() => {
      expect(mockLocation.href).toBe(mockCheckoutUrl)
      expect(onSuccess).toHaveBeenCalledWith(mockCheckoutUrl)
    })
  })

  test('calls onError callback on failure', async () => {
    const mockErrorResponse = {
      data: undefined,
      error: { message: 'Checkout failed' },
    }

    vi.mocked(createCheckoutSession).mockResolvedValue(mockErrorResponse as any)

    const onError = vi.fn()
    const { result } = renderHook(() => useStripeCheckout({ onError }), { wrapper })

    result.current.mutate({
      realmId: mockRealmId,
      clientAppId: mockClientAppId,
      entitlementKey: 'pro',
      paymentProvider: 'stripe',
    })

    await waitFor(() => {
      expect(onError).toHaveBeenCalled()
      expect(onError.mock.calls[0][0].message).toBe('Checkout failed')
    })
  })

  test('handles missing checkout URL in response', async () => {
    const mockResponse = {
      data: {}, // Missing checkoutUrl
      error: undefined,
    }

    vi.mocked(createCheckoutSession).mockResolvedValue(mockResponse as any)

    const onError = vi.fn()
    const { result } = renderHook(() => useStripeCheckout({ onError }), { wrapper })

    result.current.mutate({
      realmId: mockRealmId,
      clientAppId: mockClientAppId,
      entitlementKey: mockEntitlementKey,
      paymentProvider: mockPaymentProvider,
    })

    await waitFor(() => {
      expect(onError).toHaveBeenCalled()
      expect(onError.mock.calls[0][0].message).toBe('No checkout URL returned from server')
    })
  })

  test('passes correct parameters for different entitlement keys', async () => {
    const mockResponse = {
      data: { checkoutUrl: mockCheckoutUrl },
      error: undefined,
    }

    vi.mocked(createCheckoutSession).mockResolvedValue(mockResponse as any)

    const { result } = renderHook(() => useStripeCheckout(), { wrapper })

    // Test with a different entitlement key
    result.current.mutate({
      realmId: mockRealmId,
      clientAppId: mockClientAppId,
      entitlementKey: 'pro',
      paymentProvider: 'stripe',
    })

    await waitFor(() => {
      expect(createCheckoutSession).toHaveBeenCalledWith({
        path: { realmId: mockRealmId, clientAppId: mockClientAppId },
        body: { entitlementKey: 'pro', paymentProvider: 'stripe' },
      })
    })
  })

  describe('error handling', () => {
    test('handles API error response', async () => {
      const mockError = {
        message: 'Invalid Stripe API key',
        code: 'INVALID_API_KEY',
      }

      vi.mocked(createCheckoutSession).mockResolvedValue({
        data: undefined,
        error: mockError,
      } as any)

      const onError = vi.fn()
      const { result } = renderHook(() => useStripeCheckout({ onError }), { wrapper })

      result.current.mutate({
        realmId: mockRealmId,
        clientAppId: mockClientAppId,
        entitlementKey: mockEntitlementKey,
        paymentProvider: mockPaymentProvider,
      })

      await waitFor(() => {
        expect(onError).toHaveBeenCalled()
        expect(onError.mock.calls[0][0].message).toContain('Invalid Stripe API key')
      })
    })

    test('handles webhook not configured error', async () => {
      const mockError = {
        message:
          'Stripe webhook not configured. Please configure webhook secret in realm settings.',
        code: 'WEBHOOK_NOT_CONFIGURED',
      }

      vi.mocked(createCheckoutSession).mockResolvedValue({
        data: undefined,
        error: mockError,
      } as any)

      const onError = vi.fn()
      const { result } = renderHook(() => useStripeCheckout({ onError }), { wrapper })

      result.current.mutate({
        realmId: mockRealmId,
        clientAppId: mockClientAppId,
        entitlementKey: mockEntitlementKey,
        paymentProvider: mockPaymentProvider,
      })

      await waitFor(() => {
        expect(onError).toHaveBeenCalled()
        expect(onError.mock.calls[0][0].message).toContain('Stripe webhook not configured')
      })
    })

    test('handles unauthorized error', async () => {
      const mockError = {
        message: 'Unauthorized: Invalid or missing credentials',
        code: 'UNAUTHORIZED',
      }

      vi.mocked(createCheckoutSession).mockResolvedValue({
        data: undefined,
        error: mockError,
      } as any)

      const onError = vi.fn()
      const { result } = renderHook(() => useStripeCheckout({ onError }), { wrapper })

      result.current.mutate({
        realmId: mockRealmId,
        clientAppId: mockClientAppId,
        entitlementKey: mockEntitlementKey,
        paymentProvider: mockPaymentProvider,
      })

      await waitFor(() => {
        expect(onError).toHaveBeenCalled()
        expect(onError.mock.calls[0][0].message).toContain('Unauthorized')
      })
    })
  })

  describe('mutation state management', () => {
    test('sets isPending during mutation', async () => {
      let resolveMutation: (value: any) => void
      const pendingPromise = new Promise((resolve) => {
        resolveMutation = resolve
      })

      vi.mocked(createCheckoutSession).mockReturnValue(
        new Promise((resolve) => {
          setTimeout(() => {
            resolve({
              data: { checkoutUrl: mockCheckoutUrl },
              error: undefined,
            } as any)
            resolveMutation!({})
          }, 100)
        }) as any
      )

      const { result } = renderHook(() => useStripeCheckout(), { wrapper })

      // Start mutation but don't await
      result.current.mutate({
        realmId: mockRealmId,
        clientAppId: mockClientAppId,
        entitlementKey: mockEntitlementKey,
        paymentProvider: mockPaymentProvider,
      })

      // Wait a bit for mutation to start
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 10))
      })

      // Should be pending
      expect(result.current.isPending).toBe(true)

      // Wait for completion
      await waitFor(() => {
        expect(result.current.isPending).toBe(false)
      })
    })

    test('sets error state on failure', async () => {
      const mockErrorResponse = {
        data: undefined,
        error: { message: 'Network error' },
      }

      vi.mocked(createCheckoutSession).mockResolvedValue(mockErrorResponse as any)

      const { result } = renderHook(() => useStripeCheckout(), { wrapper })

      result.current.mutate({
        realmId: mockRealmId,
        clientAppId: mockClientAppId,
        entitlementKey: mockEntitlementKey,
        paymentProvider: mockPaymentProvider,
      })

      await waitFor(() => {
        expect(result.current.error).toBeTruthy()
        expect(result.current.error?.message).toContain('Network error')
      })
    })

    test('resets error state on successful retry', async () => {
      const mockErrorResponse = {
        data: undefined,
        error: { message: 'Network error' },
      }
      const mockSuccessResponse = {
        data: { checkoutUrl: mockCheckoutUrl },
        error: undefined,
      }

      // First call fails
      vi.mocked(createCheckoutSession)
        .mockResolvedValueOnce(mockErrorResponse as any)
        .mockResolvedValueOnce(mockSuccessResponse as any)

      const { result } = renderHook(() => useStripeCheckout(), { wrapper })

      // First attempt - fails
      result.current.mutate({
        realmId: mockRealmId,
        clientAppId: mockClientAppId,
        entitlementKey: mockEntitlementKey,
        paymentProvider: mockPaymentProvider,
      })

      await waitFor(() => {
        expect(result.current.error).toBeTruthy()
      })

      // Second attempt - succeeds
      act(() => {
        result.current.reset()
      })

      result.current.mutate({
        realmId: mockRealmId,
        clientAppId: mockClientAppId,
        entitlementKey: mockEntitlementKey,
        paymentProvider: mockPaymentProvider,
      })

      await waitFor(() => {
        expect(result.current.error).toBeNull()
      })
    })
  })

  describe('callback behavior', () => {
    test('calls onSuccess before redirect', async () => {
      const callOrder: string[] = []

      vi.mocked(createCheckoutSession).mockResolvedValue({
        data: { checkoutUrl: mockCheckoutUrl },
        error: undefined,
      } as any)

      const onSuccess = vi.fn(() => {
        callOrder.push('onSuccess')
      })

      const { result } = renderHook(() => useStripeCheckout({ onSuccess }), { wrapper })

      result.current.mutate({
        realmId: mockRealmId,
        clientAppId: mockClientAppId,
        entitlementKey: mockEntitlementKey,
        paymentProvider: mockPaymentProvider,
      })

      await waitFor(() => {
        expect(callOrder).toContain('onSuccess')
      })
    })

    test('works without callbacks', async () => {
      vi.mocked(createCheckoutSession).mockResolvedValue({
        data: { checkoutUrl: mockCheckoutUrl },
        error: undefined,
      } as any)

      const { result } = renderHook(() => useStripeCheckout(), { wrapper })

      result.current.mutate({
        realmId: mockRealmId,
        clientAppId: mockClientAppId,
        entitlementKey: mockEntitlementKey,
        paymentProvider: mockPaymentProvider,
      })

      await waitFor(() => {
        expect(mockLocation.href).toBe(mockCheckoutUrl)
      })
    })
  })

  describe('concurrent mutations', () => {
    test('handles multiple sequential mutations', async () => {
      vi.mocked(createCheckoutSession).mockResolvedValue({
        data: { checkoutUrl: mockCheckoutUrl },
        error: undefined,
      } as any)

      const { result } = renderHook(() => useStripeCheckout(), { wrapper })

      // First mutation
      result.current.mutate({
        realmId: mockRealmId,
        clientAppId: mockClientAppId,
        entitlementKey: 'basic',
        paymentProvider: 'stripe',
      })

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true)
      })

      // Second mutation
      act(() => {
        result.current.reset()
      })

      result.current.mutate({
        realmId: mockRealmId,
        clientAppId: mockClientAppId,
        entitlementKey: 'pro',
        paymentProvider: 'stripe',
      })

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true)
      })
    })
  })
})
