import { useMutation, useQueryClient, type QueryKey } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import { queryKeys } from '@/data/query-options'
import { m } from '@/paraglide/messages'

interface UseSaveConfigMutationProps<T> {
  realmId: string
  mutationFn: (data: T) => Promise<void>
  providerName: string
  invalidateKeys?: QueryKey[]
  isEditing: boolean
}

/**
 * Shared mutation hook for billing config forms.
 * Handles error toast + success toast + query invalidation + navigation.
 */
export function useSaveConfigMutation<T>({
  realmId,
  mutationFn,
  providerName,
  invalidateKeys,
  isEditing,
}: UseSaveConfigMutationProps<T>) {
  const navigate = useNavigate()
  const queryClient = useQueryClient()

  const defaultInvalidateKeys = [
    ['payment-providers', realmId],
    ['realmConfig', realmId],
    queryKeys.featureAvailability(realmId),
  ]

  const keysToInvalidate = invalidateKeys ?? defaultInvalidateKeys

  return useMutation({
    mutationFn,
    onSuccess: async () => {
      const action = isEditing ? m['billing.updated']() : m['billing.created']()
      toast.success(m['billing.config_saved']({ provider: providerName, action }))
      await Promise.all(
        keysToInvalidate.map((key) => queryClient.invalidateQueries({ queryKey: key }))
      )
      navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
    },
    onError: (error: { status?: number; message?: string }) => {
      toast.error(m['billing.config_save_failed']({ message: error?.message || 'Unknown error' }))
    },
  })
}
