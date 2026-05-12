import { AlertCircle } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'

interface UnclaimedSubscriptionBannerProps {
  count: number
  onClaimClick: () => void
  onClose: () => void
}

export function UnclaimedSubscriptionBanner({
  count,
  onClaimClick,
  onClose,
}: UnclaimedSubscriptionBannerProps) {
  if (count === 0) {
    return null
  }

  return (
    <Alert
      className="border-yellow-200 bg-yellow-50 relative"
      data-testid="unclaimed-subscription-banner"
    >
      <button
        onClick={onClose}
        className="absolute right-2 top-2 text-yellow-600 hover:text-yellow-800"
        data-testid="unclaimed-banner-close-button"
        aria-label="Close"
      >
        ×
      </button>
      <AlertCircle className="h-4 w-4 text-yellow-600" />
      <AlertTitle className="text-yellow-800 pr-6">
        Unclaimed Shopify Subscription{count > 1 ? 's' : ''} Found
      </AlertTitle>
      <AlertDescription className="flex items-center justify-between gap-4 text-yellow-900">
        <span data-testid="unclaimed-count-display">
          Found {count} Shopify subscription{count > 1 ? 's' : ''} waiting to be claimed. Claim{' '}
          {count > 1 ? 'them' : 'it'} to start receiving your subscription benefits.
        </span>
        <Button
          size="sm"
          onClick={onClaimClick}
          className="shrink-0"
          data-testid="claim-subscription-button"
        >
          Claim Subscription{count > 1 ? 's' : ''}
        </Button>
      </AlertDescription>
    </Alert>
  )
}
