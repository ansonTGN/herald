import { useQuery } from '@tanstack/react-query'
import { creditBucketsListQueryOptions } from '@/data/query-options'
import type { BucketResponse } from '@/lib/api-generated'

/**
 * Minimal bucket view-model consumed by Select/option groups across the
 * credit-bucket feature (grant dialog, entitlement mappings,
 * purchase grouping, admin wallets).
 *
 * Source-of-truth type is {@link BucketResponse}; we project only the fields
 * downstream selects need so callers don't couple to the full DTO.
 */
export type BucketOption = Pick<BucketResponse, 'id' | 'name' | 'enabled' | 'bucketKey'>

/**
 * All credit buckets for `realmId` (enabled + disabled).
 *
 * Backed by {@link creditBucketsListQueryOptions}. `buckets` is `[]`
 * while loading so select components can render an empty state without
 * distinguishing undefined; use `isLoading` for skeletons.
 */
export function useBuckets(realmId: string) {
  const query = useQuery(creditBucketsListQueryOptions(realmId))
  const buckets: BucketOption[] = (query.data ?? []).map((b) => ({
    id: b.id,
    name: b.name,
    enabled: b.enabled,
    bucketKey: b.bucketKey,
  }))
  return { buckets, isLoading: query.isLoading }
}

/**
 * Only enabled credit buckets for `realmId`. Use this for the option list in
 * grant/entitlement/purchase flows where disabled buckets are not selectable.
 */
export function useEnabledBuckets(realmId: string) {
  const { buckets, isLoading } = useBuckets(realmId)
  return { buckets: buckets.filter((b) => b.enabled), isLoading }
}
