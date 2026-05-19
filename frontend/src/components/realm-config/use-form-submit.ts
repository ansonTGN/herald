import { useState } from 'react'

/**
 * Custom hook for handling form submission with duplicate prevention.
 * Encapsulates the standard form submission pattern used across config forms.
 *
 * @param onSave - The async save function to call
 * @param disabled - Whether the form is disabled
 * @returns Object containing submit function and submitting state
 */
export function useFormSubmit<T>(onSave: (values: T) => Promise<void>, disabled?: boolean) {
  const [isSubmitting, setIsSubmitting] = useState(false)

  async function handleSubmit(values: T) {
    // Check if form is disabled
    if (disabled) {
      throw new Error('Form is disabled. You do not have permission to modify this configuration.')
    }

    // Prevent duplicate submissions
    if (isSubmitting) {
      console.log('Form is already submitting, ignoring duplicate submission')
      return
    }

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
