import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { updateEntitlementMapping, syncProviderProducts } from '@/lib/api-generated'
import type { UpdateEntitlementMappingRequest } from '@/lib/api-generated'
import type { EntitlementMappingUpdateFormData } from '@/lib/schemas/billing-forms'
import { getErrorMessage } from '@/lib/error-utils'
import { queryKeys } from '@/data/query-options'

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
        // bucketId triple-state: omit when absent (preserve), null clears,
        // string sets. The toggle-enabled path omits it; the detail dialog
        // always supplies the intended value (idempotent re-assert on no-op).
        ...(values.bucketId !== undefined ? { bucketId: values.bucketId } : {}),
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
        queryKey: [queryKeys.entitlementMappings(realmId, {})],
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.entitlementMapping(realmId, mappingId),
      })
      // Mapping <-> bucket attribution is two-sided: changing a mapping's
      // bucketId moves it between bucket holders counts, so the bucket
      // directory (FE-D03) and overview (FE-D04) must re-fetch. Both write
      // the same FK consistently; no conflict.
      queryClient.invalidateQueries({
        queryKey: queryKeys.creditBucketsList(realmId),
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
        queryKey: [queryKeys.entitlementMappings(realmId, {})],
      })
    },
    onError: (error) => {
      const errorMessage = getErrorMessage(error)
      toast.error(`Failed to sync provider products: ${errorMessage}`)
    },
  })
}
