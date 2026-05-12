import { useMutation } from '@tanstack/react-query'
import { createCheckoutSession } from '@/lib/api-generated'

interface UseStripeCheckoutOptions {
  onSuccess?: (checkoutUrl: string) => void
  onError?: (error: Error) => void
}

interface CreateCheckoutSessionParams {
  realmId: string
  clientAppId: string
  planId: string
  paymentProvider: string
  billingPeriod: 'monthly' | 'yearly'
}

/**
 * Hook for creating Stripe Checkout sessions
 * Redirects to Stripe Checkout URL on success
 */
export function useStripeCheckout({ onSuccess, onError }: UseStripeCheckoutOptions = {}) {
  return useMutation({
    mutationFn: async ({
      realmId,
      clientAppId,
      planId,
      paymentProvider,
      billingPeriod,
    }: CreateCheckoutSessionParams) => {
      const response = await createCheckoutSession({
        path: { realmId, clientAppId },
        body: { planId, paymentProvider, billingPeriod },
      })

      if (response.error) {
        throw new Error(response.error.message || 'Failed to create checkout session')
      }

      if (!response.data?.checkoutUrl) {
        throw new Error('No checkout URL returned from server')
      }

      return response.data
    },
    onSuccess: (data) => {
      // Redirect to Stripe Checkout
      window.location.href = data.checkoutUrl
      onSuccess?.(data.checkoutUrl)
    },
    onError: (error: Error) => {
      onError?.(error)
    },
  })
}
