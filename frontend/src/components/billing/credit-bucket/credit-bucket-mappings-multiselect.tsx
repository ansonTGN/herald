import type { MultiselectOption } from './credit-bucket-coverage-multiselect'
import { BucketMultiselect } from './credit-bucket-coverage-multiselect'
import { m } from '@/paraglide/messages'

interface CreditBucketMappingsMultiselectProps {
  options: MultiselectOption[]
  value: string[]
  onChange: (next: string[]) => void
  testIdPrefix?: string
  placeholder?: string
  searchPlaceholder?: string
  emptyText?: string
  disabled?: boolean
  error?: string
}

/**
 * Entitlement-mappings multiselect: packages / point packs attributed to a
 * bucket. Optional (may be empty), so no at-least-one rule here.
 *
 * Reuses the shared {@link BucketMultiselect} Popover + Command pattern.
 */
export function CreditBucketMappingsMultiselect({
  testIdPrefix = 'bucket-mappings-multiselect',
  placeholder,
  searchPlaceholder,
  emptyText,
  ...rest
}: CreditBucketMappingsMultiselectProps) {
  return (
    <BucketMultiselect
      {...rest}
      placeholder={placeholder ?? m['credit_buckets.mappings_placeholder']()}
      searchPlaceholder={searchPlaceholder ?? m['credit_buckets.mappings_search_placeholder']()}
      emptyText={emptyText ?? m['credit_buckets.mappings_empty']()}
      testIdPrefix={testIdPrefix}
    />
  )
}
