import { z } from 'zod'

/**
 * Type guard for Zod formatted errors and custom error objects
 */
type FormFieldError = string | { _errors: string[] } | { message?: string }

/**
 * Type guard to check if value is a valid form error
 */
export function isFormError(error: unknown): error is FormFieldError {
  if (typeof error === 'string') return true
  if (typeof error === 'object' && error !== null) {
    if ('message' in error && typeof error.message === 'string') return true
    if ('_errors' in error) {
      const err = error as Record<string, unknown>
      return Array.isArray(err._errors)
    }
  }
  return false
}

/**
 * Extract error message from form field meta with type safety
 * Handles string errors, Zod formatted errors, and custom error objects
 */
export function getFieldErrorMessage(meta: { errors?: unknown[] }): string | undefined {
  if (!meta.errors || meta.errors.length === 0) {
    return undefined
  }

  const error = meta.errors[0]

  // Type guard narrows the type safely
  if (!isFormError(error)) {
    return 'Validation error'
  }

  // Handle different error formats
  if (typeof error === 'string') {
    return error
  }

  if ('message' in error && error.message) {
    return error.message
  }

  if ('_errors' in error && error._errors.length > 0) {
    return error._errors[0]
  }

  return 'Validation error'
}

/**
 * Helper function to handle empty string to undefined transformation for optional select fields
 */
export function optionalStringEnum<T extends z.ZodType>(enumSchema: T) {
  return z.preprocess((val) => (val === '' ? undefined : val), enumSchema.optional())
}

/**
 * Type-safe field value extraction utilities
 */

export function getFieldValueAsString(value: unknown): string {
  if (value === null || value === undefined) return ''
  if (typeof value === 'string') return value
  return String(value)
}

export function getFieldValueAsNumber(value: unknown, defaultValue: number = 0): number {
  if (value === null || value === undefined) return defaultValue
  if (typeof value === 'number') return value
  const num = Number(value)
  return isNaN(num) ? defaultValue : num
}

export function getFieldValueAsStringArray(value: unknown): string[] {
  if (Array.isArray(value)) return value as string[]
  return []
}

/**
 * Validate that a field is non-empty when creating (not editing).
 * Sets an onSubmit error on the form field if the value is empty.
 */
export function requireFieldOnCreate<T extends string>(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  form: { setFieldMeta: (name: T, updater: (meta: any) => any) => void },
  isEditing: boolean,
  fieldName: T,
  value: string | undefined | null,
  errorMessage: string
): boolean {
  if (!isEditing && !value) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    form.setFieldMeta(fieldName, (meta: any) => ({
      ...meta,
      errorMap: { onSubmit: errorMessage },
    }))
    return false
  }
  return true
}
