import { useState } from 'react'
import { RefreshCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useSyncProviderProducts } from '@/data/entitlement-mapping-mutations'
import { m } from '@/paraglide/messages'

interface ProviderSyncButtonProps {
  realmId: string
  onSyncComplete?: () => void
}

const SYNC_PROVIDERS = [
  { value: 'stripe', label: 'Stripe' },
  { value: 'creem', label: 'Creem' },
] as const

export function ProviderSyncButton({ realmId, onSyncComplete }: ProviderSyncButtonProps) {
  const [selectedProvider, setSelectedProvider] = useState<string>('')
  const syncMutation = useSyncProviderProducts(realmId)

  const handleSync = () => {
    if (!selectedProvider) return
    syncMutation.mutate(
      { paymentProvider: selectedProvider },
      {
        onSuccess: () => {
          onSyncComplete?.()
        },
      }
    )
  }

  // Show counts inline only for a completed sync (the mutation also toasts).
  // `partial` still surfaces what synced so the admin can see price-level progress.
  const syncData = syncMutation.data
  const showCounts =
    syncData != null && (syncData.syncStatus === 'completed' || syncData.syncStatus === 'partial')

  return (
    <div className="flex items-center gap-2" data-testid="provider-sync-button">
      <Select value={selectedProvider} onValueChange={setSelectedProvider}>
        <SelectTrigger className="w-[160px]" data-testid="sync-provider-select">
          <SelectValue placeholder={m['billing.sync_provider']()} />
        </SelectTrigger>
        <SelectContent>
          {SYNC_PROVIDERS.map((provider) => (
            <SelectItem key={provider.value} value={provider.value}>
              {provider.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <Button
        onClick={handleSync}
        disabled={!selectedProvider || syncMutation.isPending}
        data-testid="sync-button"
      >
        <RefreshCw
          className={syncMutation.isPending ? 'mr-2 h-4 w-4 animate-spin' : 'mr-2 h-4 w-4'}
        />
        {m['billing.sync_provider']()}
      </Button>
      {showCounts && (
        <span className="text-sm text-muted-foreground" data-testid="sync-result">
          <span data-testid="sync-result-products">
            {m['billing.sync_result_products']({ count: syncData.productsSynced })}
          </span>
          {' · '}
          <span data-testid="sync-result-prices">
            {m['billing.sync_result_prices']({ count: syncData.pricesSynced })}
          </span>
        </span>
      )}
    </div>
  )
}
