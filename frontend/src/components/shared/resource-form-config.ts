import type { QueryKey } from '@tanstack/react-query'
import type { z } from 'zod'

export interface ResourceFormConfig<TData, TResponse = unknown> {
  schema: z.ZodType<any> // eslint-disable-line @typescript-eslint/no-explicit-any -- Zod schema type is complex and auto-inferred
  defaultValues: TData
  mutationFn: (data: TData) => Promise<TResponse>
  getSuccessMessage: (response: TResponse) => string
  queryKeysToInvalidate: QueryKey[]
  nameFieldLabel: string
  nameFieldPlaceholder?: string
  nameFieldHelpText?: string
  descriptionFieldPlaceholder: string
  nameFieldTestId: string
  descriptionFieldTestId: string
  submitButtonTestId: string
  submitButtonText: string
  submittingButtonText: string
  nameInputId?: string
  descriptionInputId?: string
}
