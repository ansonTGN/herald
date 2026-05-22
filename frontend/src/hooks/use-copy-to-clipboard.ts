import { useState, useRef, useCallback, useEffect } from 'react'

const DEFAULT_FEEDBACK_DURATION_MS = 2000

/**
 * Hook for copying text to clipboard with a temporary "copied" feedback state.
 * Uses a ref-based timer so repeated calls cancel the previous timer,
 * and cleanup happens on unmount.
 */
export function useCopyToClipboard(feedbackDurationMs = DEFAULT_FEEDBACK_DURATION_MS) {
  const [copied, setCopied] = useState(false)
  const timerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current)
    }
  }, [])

  const copyToClipboard = useCallback(
    async (text: string) => {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      if (timerRef.current) clearTimeout(timerRef.current)
      timerRef.current = setTimeout(() => setCopied(false), feedbackDurationMs)
    },
    [feedbackDurationMs]
  )

  return { copied, copyToClipboard } as const
}
