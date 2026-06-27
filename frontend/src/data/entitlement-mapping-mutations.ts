import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  updateEntitlementMapping,
  syncProviderProducts,
  batchUpdateEntitlementMappings,
} from '@/lib/api-generated'
import type {
  UpdateEntitlementMappingRequest,
  BatchUpdateEntitlementMappingsRequest,
  BatchUpdateEntitlementMappingsResponse,
} from '@/lib/api-generated'
import { getErrorMessage } from '@/lib/error-utils'
import { queryKeys } from '@/data/query-options'

/**
 * Input shape for the legacy single-row PATCH hook below. The shared schema
 * was removed when the admin editor moved to a batch save; this local type
 * keeps the hook compilable for any remaining non-admin consumer.
 */
interface EntitlementMappingUpdateFormData {
  entitlementKey: string
  enabled: boolean
  pointsPerPeriod: number | null
  grantPeriodType: 'once' | 'daily' | 'weekly' | 'monthly' | null
  validityDays: number | null
  grantOnSubscribe: boolean
  maxPeriods: number | null
}

// ==================== Protected-price 409 detection ====================
//
// The batch endpoint rolls back the whole transaction and answers 409 with
// `{ code: "mapping_in_use", activeSubscriptions }` (typed as
// `MappingActiveSubscriptionLockErrorBody`) when a row transitions
// enabled true→false while protected by an active subscription. The mutation
// below follows the repo convention
// `if (response.error) throw response.error`, so these helpers receive the
// thrown `response.error` value — which for a 409 IS the typed lock body.
//
// NOTE: `MappingActiveSubscriptionLockErrorBody.code` is typed `string`
// (not the literal `'mapping_in_use'`), so `isProtectedPriceError` narrows
// with the literal check. If the backend ever renames the code, this check
// silently breaks — the 409 would then surface as a generic error toast.

/**
 * Error code the batch endpoint answers with on the active-subscription lock
 * (409). Extracted as a named constant so a backend rename surfaces here
 * rather than only as a silent magic-string match.
 */
export const PROTECTED_PRICE_ERROR_CODE = 'mapping_in_use' as const

/**
 * Returns true when the thrown error is the batch-save 409 lock body
 * (`{ code: 'mapping_in_use', activeSubscriptions }`). The caller should
 * then open the protected-price confirmation dialog instead of toasting.
 */
export function isProtectedPriceError(error: unknown): boolean {
  if (!error || typeof error !== 'object') return false
  const e = error as { code?: unknown; activeSubscriptions?: unknown }
  return e.code === PROTECTED_PRICE_ERROR_CODE && typeof e.activeSubscriptions === 'number'
}

/**
 * Extracts the active-subscription count from a protected-price 409 error.
 * Returns `null` for any other shape (caller should fall back to a generic
 * message).
 */
export function extractActiveSubscriptions(error: unknown): number | null {
  if (!isProtectedPriceError(error)) return null
  return (error as { activeSubscriptions: number }).activeSubscriptions
}

export function useUpdateEntitlementMapping(realmId: string, mappingId: string) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (values: EntitlementMappingUpdateFormData) => {
      const body: UpdateEntitlementMappingRequest = {
        entitlementKey: values.entitlementKey,
        enabled: values.enabled,
        pointsPerPeriod: values.pointsPerPeriod ?? undefined,
        grantPeriodType: values.grantPeriodType ?? undefined,
        validityDays: values.validityDays ?? undefined,
        grantOnSubscribe: values.grantOnSubscribe,
        maxPeriods: values.maxPeriods ?? undefined,
        // bucketId is intentionally NOT sent: the PATCH handler preserves the
        // existing attribution (assignment is owned by the Credit Bucket
        // directory page). See entitlement-mapping-detail-dialog.tsx.
      }
      const response = await updateEntitlementMapping({
        path: { realmId, mappingId },
        body,
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      toast.success('Mapping updated')
      queryClient.invalidateQueries({
        queryKey: queryKeys.entitlementMappings(realmId, {}),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.entitlementMapping(realmId, mappingId),
      })
    },
    onError: (error) => {
      const errorMessage = getErrorMessage(error)
      toast.error(`Failed to update mapping: ${errorMessage}`)
    },
  })
}

export function useSyncProviderProducts(realmId: string) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ paymentProvider }: { paymentProvider: string }) => {
      const response = await syncProviderProducts({
        path: { realmId },
        body: { paymentProvider },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: (data) => {
      if (data.syncStatus === 'completed') {
        toast.success(`Synced ${data.productsSynced} products and ${data.pricesSynced} prices`)
      } else if (data.syncStatus === 'partial') {
        toast.warning(`Partial sync: ${data.productsSynced} products synced, some prices failed`)
      }
      queryClient.invalidateQueries({
        queryKey: queryKeys.entitlementMappings(realmId, {}),
      })
    },
    onError: (error) => {
      const errorMessage = getErrorMessage(error)
      toast.error(`Failed to sync provider products: ${errorMessage}`)
    },
  })
}

// ==================== Batch Update (price-granularity) ====================

/**
 * Batch upsert of a product's price rows. The whole batch
 * is one server-side transaction; on 409 (`mapping_in_use`) the caller is
 * expected to catch the thrown `response.error` and surface the
 * protected-price confirmation dialog via `isProtectedPriceError` /
 * `extractActiveSubscriptions` — this hook intentionally does NOT toast on
 * a 409 so the page can handle the lock interactively.
 */
export function useBatchUpdateEntitlementMappings(realmId: string) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (body: BatchUpdateEntitlementMappingsRequest) => {
      const response = await batchUpdateEntitlementMappings({
        path: { realmId },
        body,
      })
      if (response.error) throw response.error
      return response.data as BatchUpdateEntitlementMappingsResponse
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.entitlementMappings(realmId, {}),
      })
    },
    onError: (error) => {
      // A protected-price 409 is handled by the caller (confirmation dialog);
      // only toast for non-lock errors here.
      if (isProtectedPriceError(error)) return
      const errorMessage = getErrorMessage(error)
      toast.error(`Failed to save mappings: ${errorMessage}`)
    },
  })
}
