import { type SubscriptionPlanResponse, type PaymentProviderSummary } from '@/lib/api-generated'
import { Button } from '@/components/ui/button'
import { CreditCard, Loader2 } from 'lucide-react'
import { getEnabledProviders } from '@/lib/billing-utils'
import { formatProviderName } from './format-provider-name'
import { m } from '@/paraglide/messages'

interface CheckoutProviderSelectorProps {
  plan: SubscriptionPlanResponse
  onSelectProvider: (provider: string) => void
  isPending?: boolean
}

export function CheckoutProviderSelector({
  plan,
  onSelectProvider,
  isPending = false,
}: CheckoutProviderSelectorProps) {
  const enabledProviders = getEnabledProviders(plan.paymentProviders)

  if (enabledProviders.length === 0) {
    return (
      <div className="text-center space-y-2" data-testid="checkout-no-providers">
        <p className="text-sm text-muted-foreground">{m['billing.checkout_no_providers']()}</p>
      </div>
    )
  }

  if (enabledProviders.length === 1) {
    // Single provider - render a direct subscribe button
    const provider = enabledProviders[0]
    return (
      <Button
        onClick={() => onSelectProvider(provider.paymentProvider)}
        disabled={isPending}
        data-testid={`checkout-provider-button-${provider.paymentProvider}`}
      >
        {isPending ? (
          <>
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            {m['billing.checkout_processing']()}
          </>
        ) : (
          <>
            <CreditCard className="mr-2 h-4 w-4" />
            {m['billing.checkout_subscribe_with']({
              name: formatProviderName(provider.paymentProvider),
            })}
          </>
        )}
      </Button>
    )
  }

  // Multiple providers - render selection buttons
  return (
    <div className="space-y-3" data-testid="checkout-provider-selector">
      <p className="text-sm font-medium">{m['billing.checkout_select_method']()}</p>
      <div className="flex flex-col gap-2">
        {enabledProviders.map((provider: PaymentProviderSummary) => (
          <Button
            key={provider.id}
            variant="outline"
            onClick={() => onSelectProvider(provider.paymentProvider)}
            disabled={isPending}
            data-testid={`checkout-provider-button-${provider.paymentProvider}`}
            className="justify-start"
          >
            <CreditCard className="mr-2 h-4 w-4" />
            {formatProviderName(provider.paymentProvider)}
          </Button>
        ))}
      </div>
    </div>
  )
}
