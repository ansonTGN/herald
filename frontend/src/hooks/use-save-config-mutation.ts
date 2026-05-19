import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'

interface UseSaveConfigMutationProps<T> {
  realmId: string
  mutationFn: (data: T) => Promise<void>
  providerName: string
  invalidateKeys?: string[][]
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
  ]

  const keysToInvalidate = invalidateKeys ?? defaultInvalidateKeys

  return useMutation({
    mutationFn,
    onSuccess: async () => {
      toast.success(
        isEditing
          ? `${providerName} configuration updated successfully`
          : `${providerName} configuration created successfully`
      )
      await Promise.all(
        keysToInvalidate.map((key) => queryClient.invalidateQueries({ queryKey: key }))
      )
      navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
    },
    onError: (error: { status?: number; message?: string }) => {
      toast.error(`Failed to save configuration: ${error?.message || 'Unknown error'}`)
    },
  })
}
