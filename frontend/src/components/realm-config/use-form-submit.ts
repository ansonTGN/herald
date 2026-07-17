import { useState } from 'react'

/**
 * Custom hook for handling form submission state. Encapsulates the standard
 * isSubmitting pattern used across config forms: tracks in-flight state and
 * logs (then re-throws) errors so the caller can surface them.
 *
 * Duplicate-submission / disabled-form guards are intentionally NOT here —
 * every config form disables its submit button while `isSubmitting`/`disabled`,
 * so those guards were unreachable.
 *
 * @param onSave - The async save function to call
 * @returns Object containing submit function and submitting state
 */
export function useFormSubmit<T>(onSave: (values: T) => Promise<void>) {
  const [isSubmitting, setIsSubmitting] = useState(false)

  async function handleSubmit(values: T) {
    setIsSubmitting(true)
    try {
      await onSave(values)
    } catch (error) {
      console.error('Failed to save configuration:', error)
      throw error
    } finally {
      setIsSubmitting(false)
    }
  }

  return { handleSubmit, isSubmitting }
}
