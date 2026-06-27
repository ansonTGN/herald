import { useCallback } from 'react'
import { Button } from '@/components/ui/button'
import { CreditCard, Loader2 } from 'lucide-react'
import { useStripeCheckout } from '@/hooks/use-stripe-checkout'
import { toast } from 'sonner'
import { m } from '@/paraglide/messages'

interface StripeCheckoutButtonProps {
  realmId: string
  clientAppId: string
  mappingId: string
  variant?: 'default' | 'outline' | 'ghost' | 'destructive'
  size?: 'default' | 'sm' | 'lg' | 'icon'
  className?: string
}

/**
 * Button component for initiating Stripe Checkout.
 * Displays loading state and handles errors.
 *
 * @precondition Callers MUST verify that Stripe is available as a payment provider
 * before rendering this component. The component does not perform this check internally.
 */
export function StripeCheckoutButton({
  realmId,
  clientAppId,
  mappingId,
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
        mappingId,
        paymentProvider: 'stripe',
      })
    } catch (error) {
      // Error is already handled by the mutation callbacks
      console.error('Checkout error:', error)
    }
  }, [checkoutMutation, realmId, clientAppId, mappingId])

  return (
    <Button
      onClick={handleCheckout}
      disabled={checkoutMutation.isPending}
      variant={variant}
      size={size}
      className={className}
      data-testid={`stripe-checkout-button-${mappingId}`}
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
