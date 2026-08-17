import { RefreshCw } from 'lucide-react'
import { useQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { useSyncProviderProducts } from '@/data/entitlement-mapping-mutations'
import { paymentProvidersQueryOptions } from '@/data/query-options'
import { formatProviderName } from '@/components/billing/format-provider-name'
import { m } from '@/paraglide/messages'

interface ProviderSyncButtonProps {
  realmId: string
  onSyncComplete?: () => void
}

// Payment providers whose hosted product catalogue can be synced into
// entitlement mappings.
const SYNCABLE_PLATFORMS = ['stripe', 'creem'] as const

export function ProviderSyncButton({ realmId, onSyncComplete }: ProviderSyncButtonProps) {
  const syncMutation = useSyncProviderProducts(realmId)
  const { data: providers } = useQuery(paymentProvidersQueryOptions(realmId))

  const configuredSyncable = SYNCABLE_PLATFORMS.filter((platform) =>
    (providers ?? []).some((p) => p.platform === platform)
  )
  const configuredNonSyncable = (providers ?? []).filter(
    (p) => !(SYNCABLE_PLATFORMS as readonly string[]).includes(p.platform)
  )

  // Sync genuinely does not apply to WeChat Pay (order-based, no hosted
  // catalogue — DEC-wechat-support-006) or in-app purchase stores. Rendering
  // a permanently-disabled "Sync provider" button there reads as broken, and
  // the nothing-configured hint ("configure a provider first") would be a lie
  // when providers ARE configured — so hide the affordance entirely. Manual
  // creation via the Create Mapping button is the correct path for these.
  if (configuredSyncable.length === 0 && configuredNonSyncable.length > 0) {
    return null
  }

  const handleSync = (paymentProvider: string) => {
    syncMutation.mutate(
      { paymentProvider },
      {
        onSuccess: () => {
          onSyncComplete?.()
        },
      }
    )
  }

  const renderSyncButton = (platform: string) => {
    const label = m['billing.sync_provider_with_name']({ name: formatProviderName(platform) })
    return (
      <Button
        key={platform}
        onClick={() => handleSync(platform)}
        disabled={syncMutation.isPending}
        data-testid="sync-button"
        data-provider={platform}
      >
        <RefreshCw
          className={syncMutation.isPending ? 'mr-2 h-4 w-4 animate-spin' : 'mr-2 h-4 w-4'}
        />
        {label}
      </Button>
    )
  }

  return (
    <div className="flex items-center gap-2" data-testid="provider-sync-button">
      {configuredSyncable.length > 0 ? (
        configuredSyncable.map(renderSyncButton)
      ) : (
        <TooltipProvider delayDuration={200}>
          <Tooltip>
            <TooltipTrigger asChild>
              {/* span wrapper so the disabled Button still receives hover events for the tooltip */}
              <span tabIndex={0}>
                <Button disabled data-testid="sync-button">
                  <RefreshCw className="h-4 w-4" />
                  {m['billing.sync_provider']()}
                </Button>
              </span>
            </TooltipTrigger>
            <TooltipContent>{m['billing.sync_provider_none_configured_hint']()}</TooltipContent>
          </Tooltip>
        </TooltipProvider>
      )}
    </div>
  )
}
