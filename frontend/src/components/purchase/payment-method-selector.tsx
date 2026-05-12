import { type PaymentProviderInfo } from '@/lib/api-generated'
import { Card, CardContent } from '@/components/ui/card'
import { Check } from 'lucide-react'
import { formatProviderName } from '@/components/billing/format-provider-name'

interface PaymentMethodSelectorProps {
  availableProviders: PaymentProviderInfo[]
  selectedProvider: string | null
  onSelect: (provider: string) => void
  disabled?: boolean
}

export function PaymentMethodSelector({
  availableProviders,
  selectedProvider,
  onSelect,
  disabled = false,
}: PaymentMethodSelectorProps) {
  if (availableProviders.length === 0) {
    return (
      <div className="rounded-lg border border-dashed p-8 text-center text-muted-foreground">
        No payment providers available
      </div>
    )
  }

  return (
    <div data-testid="payment-method-selector" className="grid grid-cols-1 gap-4 md:grid-cols-3">
      {availableProviders.map((provider) => {
        const isSelected = selectedProvider === provider.platform
        const isAvailable = provider.enabled // Use 'enabled' field instead of 'available'

        return (
          <Card
            key={provider.platform}
            className={`cursor-pointer transition-all ${
              isSelected
                ? 'border-primary ring-2 ring-primary'
                : 'border-muted-foreground/25 hover:border-muted-foreground/50'
            } ${!isAvailable ? 'opacity-50' : ''}`}
            data-testid={`payment-method-button-${provider.platform}`}
            data-selected={isSelected ? true : undefined}
          >
            <CardContent className="p-4">
              <button
                type="button"
                className="flex w-full items-center justify-between"
                onClick={() => isAvailable && !disabled && onSelect(provider.platform)}
                disabled={!isAvailable || disabled}
                data-testid={`payment-method-select-${provider.platform}`}
              >
                <div className="flex items-center gap-3">
                  <div className="flex h-10 w-10 items-center justify-center rounded-full bg-muted">
                    <span className="text-lg font-bold">
                      {formatProviderName(provider.platform).charAt(0)}
                    </span>
                  </div>
                  <div className="text-left">
                    <div className="font-medium">{formatProviderName(provider.platform)}</div>
                    {!isAvailable && (
                      <div className="text-xs text-muted-foreground">Unavailable</div>
                    )}
                  </div>
                </div>
                {isSelected && (
                  <div className="flex h-6 w-6 items-center justify-center rounded-full bg-primary">
                    <Check
                      className="h-4 w-4 text-primary-foreground"
                      data-testid={`payment-method-selected-${provider.platform}`}
                    />
                  </div>
                )}
              </button>
            </CardContent>
          </Card>
        )
      })}
    </div>
  )
}
