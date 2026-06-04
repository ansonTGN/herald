import { useCallback, useMemo } from 'react'
import { Button } from '@/components/ui/button'
import { CreditCard, Loader2 } from 'lucide-react'
import { useStripeCheckout } from '@/hooks/use-stripe-checkout'
import { toast } from 'sonner'
import type { SubscriptionPlanResponse } from '@/lib/api-generated'
import { m } from '@/paraglide/messages'

interface StripeCheckoutButtonProps {
  realmId: string
  clientAppId: string
  plan: SubscriptionPlanResponse
  billingPeriod?: 'monthly' | 'yearly'
  variant?: 'default' | 'outline' | 'ghost' | 'destructive'
  size?: 'default' | 'sm' | 'lg' | 'icon'
  className?: string
}

/**
 * Button component for initiating Stripe Checkout
 * Displays loading state and handles errors
 */
export function StripeCheckoutButton({
  realmId,
  clientAppId,
  plan,
  billingPeriod = 'monthly',
  variant = 'default',
  size = 'default',
  className,
}: StripeCheckoutButtonProps) {
  const checkoutMutation = useStripeCheckout({
    onSuccess: (_checkoutUrl) => {
      toast.success(m['billing.stripe_redirecting']())
    },
    onError: (error) => {
      toast.error(m['billing.stripe_checkout_failed']({ message: error.message }))
    },
  })

  const handleCheckout = useCallback(async () => {
    try {
      await checkoutMutation.mutateAsync({
        realmId,
        clientAppId,
        planId: plan.id,
        paymentProvider: 'stripe',
        billingPeriod,
      })
    } catch (error) {
      // Error is already handled by the mutation callbacks
      console.error('Checkout error:', error)
    }
  }, [checkoutMutation, realmId, clientAppId, plan.id, billingPeriod])

  // Memoize provider check to avoid recomputing on every render
  const stripeMapping = useMemo(
    () => plan.paymentProviders?.find((p) => p.paymentProvider === 'stripe' && p.enabled),
    [plan.paymentProviders]
  )

  if (!stripeMapping) {
    return null
  }

  return (
    <Button
      onClick={handleCheckout}
      disabled={checkoutMutation.isPending}
      variant={variant}
      size={size}
      className={className}
      data-testid={`plan-stripe-checkout-button-${plan.id}`}
    >
      {checkoutMutation.isPending ? (
        <>
          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          {m['billing.stripe_creating']()}
        </>
      ) : (
        <>
          <CreditCard className="mr-2 h-4 w-4" />
          {m['billing.stripe_subscribe']()}
        </>
      )}
    </Button>
  )
}
