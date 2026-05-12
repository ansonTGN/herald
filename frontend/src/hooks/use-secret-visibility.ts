import { useState, useEffect, useCallback } from 'react'

/**
 * Default auto-hide duration for secrets visibility (5 seconds)
 * Balances usability (time to read/cop) with security (limits exposure)
 */
const DEFAULT_AUTO_HIDE_MS = 5000

/**
 * Hook for managing secret visibility with auto-hide functionality
 * @param autoHideMs - Auto-hide duration in milliseconds (default: 5000)
 * @returns Object with showSecrets state and handler functions
 */
export function useSecretVisibility(autoHideMs = DEFAULT_AUTO_HIDE_MS) {
  const [showSecrets, setShowSecrets] = useState(false)
  const [autoHideTimer, setAutoHideTimer] = useState<ReturnType<typeof setTimeout> | null>(null)

  const handleShowSecrets = useCallback(() => {
    setShowSecrets(true)

    // Clear any existing timer
    if (autoHideTimer) {
      clearTimeout(autoHideTimer)
    }

    // Auto-hide after specified duration
    const timer = setTimeout(() => {
      setShowSecrets(false)
    }, autoHideMs)

    setAutoHideTimer(timer)
  }, [autoHideTimer, autoHideMs])

  const handleHideSecrets = useCallback(() => {
    setShowSecrets(false)
    if (autoHideTimer) {
      clearTimeout(autoHideTimer)
      setAutoHideTimer(null)
    }
  }, [autoHideTimer])

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (autoHideTimer) {
        clearTimeout(autoHideTimer)
      }
    }
  }, [autoHideTimer])

  return {
    showSecrets,
    handleShowSecrets,
    handleHideSecrets,
  }
}
