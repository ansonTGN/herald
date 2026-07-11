import { useMutation, useQueryClient, type QueryKey } from '@tanstack/react-query'
import { toast } from 'sonner'
import { getErrorMessage } from '@/lib/error-utils'

interface UseFormMutationOptions<TData, TVariables> {
  mutationFn: (variables: TVariables) => Promise<TData>
  getSuccessMessage?: (data: TData) => string
  invalidateQueries?: QueryKey[]
  onSuccess?: (data: TData) => void
}

export function useFormMutation<TData, TVariables>({
  mutationFn,
  getSuccessMessage,
  invalidateQueries = [],
  onSuccess,
}: UseFormMutationOptions<TData, TVariables>) {
  const queryClient = useQueryClient()

  const mutation = useMutation({
    mutationFn,
    onSuccess: (data) => {
      invalidateQueries.forEach((key) => queryClient.invalidateQueries({ queryKey: key }))
      toast.success(getSuccessMessage?.(data) ?? 'Success')
      onSuccess?.(data)
    },
    onError: (error) => {
      toast.error(getErrorMessage(error))
    },
  })

  return {
    isSubmitting: mutation.isPending,
    mutate: mutation.mutateAsync,
  }
}
