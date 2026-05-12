import { useCallback, useMemo } from 'react'
import { Button } from '@/components/ui/button'
import { CreditCard, Loader2 } from 'lucide-react'
import { useStripeCheckout } from '@/hooks/use-stripe-checkout'
import { toast } from 'sonner'
import type { PlanResponse } from '@/lib/api-generated'

interface StripeCheckoutButtonProps {
  realmId: string
  clientAppId: string
  plan: PlanResponse
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
      toast.success('Redirecting to Stripe Checkout...')
    },
    onError: (error) => {
      toast.error(`Failed to create checkout session: ${error.message}`)
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
          Creating...
        </>
      ) : (
        <>
          <CreditCard className="mr-2 h-4 w-4" />
          Subscribe with Stripe
        </>
      )}
    </Button>
  )
}
