import { useMutation, useQueryClient, type QueryKey } from '@tanstack/react-query'
import { toast } from 'sonner'

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
      // Handle multiple error formats: Error instances, objects with error field, strings, or fallback
      let errorMsg: string

      if (error instanceof Error) {
        errorMsg = error.message
      } else if (error && typeof error === 'object' && 'error' in error) {
        const errorObj = error as { error: unknown }
        errorMsg = typeof errorObj.error === 'string' ? errorObj.error : 'Operation failed'
      } else if (typeof error === 'string') {
        errorMsg = error
      } else {
        errorMsg = 'Operation failed'
      }

      toast.error(errorMsg)
    },
  })

  return {
    isSubmitting: mutation.isPending,
    mutate: mutation.mutateAsync,
  }
}
