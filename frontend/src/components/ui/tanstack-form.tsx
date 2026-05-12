'use client'

import { useForm } from '@tanstack/react-form'
import type { z } from 'zod'

/**
 * Type-safe form hook with Zod schema integration.
 * Types are automatically inferred from defaultValues - no generics needed!
 *
 * @example
 * ```ts
 * const form = useAppForm({
 *   schema: createUserSchema,
 *   defaultValues: {
 *     email: '',
 *     password: '',
 *   },
 *   onSubmit: async ({ value }) => {
 *     await mutate(value)
 *   },
 * })
 * // form.state.values is automatically typed as CreateUserFormData
 * ```
 */
/* eslint-disable react-refresh/only-export-components */
export function useAppForm<TSchema extends z.ZodTypeAny>(options: {
  schema: TSchema
  defaultValues: z.infer<TSchema>
  onSubmit?: (props: { value: z.infer<TSchema> }) => void | Promise<void>
}) {
  const { schema, defaultValues, onSubmit } = options

  return useForm({
    validators: {
      onChange: schema as any, // eslint-disable-line @typescript-eslint/no-explicit-any -- Zod schema type is complex and auto-inferred
    },
    defaultValues,
    onSubmit,
  })
}

export function AppForm({ children }: { children: React.ReactNode }) {
  return <>{children}</>
}
/* eslint-enable react-refresh/only-export-components */
