import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { m } from '@/paraglide/messages'
import type { BucketResponse } from '@/lib/api-generated'

interface CreditBucketListItemProps {
  bucket: BucketResponse
  selected: boolean
  onSelect: () => void
}

/**
 * Left-column list row of the Bucket directory Master-Detail.
 *
 * Surfaces: name, disabled badge (when `enabled=false`), registration-pool
 * badge (`receivesRegistrationCredits`), displayOrder, coveredClientAppCount,
 * entitlementMappingCount. NO isDefault badge/control anywhere.
 */
export function CreditBucketListItem({ bucket, selected, onSelect }: CreditBucketListItemProps) {
  return (
    <button
      type="button"
      onClick={onSelect}
      aria-pressed={selected}
      data-testid={`credit-bucket-list-item-${bucket.id}`}
      className={cn(
        'w-full rounded-md border px-3 py-2.5 text-left transition-colors',
        'hover:bg-accent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring',
        selected ? 'border-primary bg-accent' : 'bg-card',
        !bucket.enabled && 'opacity-70'
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <span className="truncate font-medium">{bucket.name}</span>
        <div className="flex shrink-0 items-center gap-1">
          {bucket.receivesRegistrationCredits && (
            <Badge
              variant="default"
              data-testid={`credit-bucket-list-item-${bucket.id}-registration-badge`}
            >
              {m['credit_buckets.registration_pool_badge']()}
            </Badge>
          )}
          {!bucket.enabled && (
            <Badge
              variant="secondary"
              data-testid={`credit-bucket-list-item-${bucket.id}-disabled-badge`}
            >
              {m['credit_buckets.disabled_badge']()}
            </Badge>
          )}
        </div>
      </div>
      <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-0.5 text-xs text-muted-foreground">
        <span className="font-mono">{bucket.bucketKey}</span>
        <span>#{bucket.displayOrder}</span>
        <span>
          {m['credit_buckets.covered_apps_count']({ count: bucket.coveredClientAppCount })}
        </span>
        <span>{m['credit_buckets.mappings_count']({ count: bucket.entitlementMappingCount })}</span>
      </div>
    </button>
  )
}
