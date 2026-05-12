import { useEffect, useRef, useCallback } from 'react'
import { toast } from 'sonner'

export interface DraftData<T> {
  data: T
  timestamp: number
  version: string
}

export interface UseDraftAutoSaveOptions<T> {
  /** Unique key for storing the draft in localStorage */
  draftKey: string
  /** Form data to auto-save, or a function that returns the data */
  data: T | (() => T)
  /** Enable/disable auto-save */
  enabled?: boolean
  /** Auto-save interval in milliseconds (default: 30000 = 30 seconds) */
  saveInterval?: number
  /** Debounce delay in milliseconds after field changes (default: 2000 = 2 seconds) */
  debounceDelay?: number
  /** Schema version for migration compatibility */
  version?: string
  /** Callback when save fails */
  onSaveError?: (error: Error) => void
}

export interface DraftMetadata {
  draftKey: string
  timestamp: number
  version: string
}

const DEFAULT_SAVE_INTERVAL = 30000 // 30 seconds
const DEFAULT_DEBOUNCE_DELAY = 2000 // 2 seconds
const DEFAULT_VERSION = '1.0'

/**
 * Hook for auto-saving form data to localStorage with debouncing and periodic saves
 *
 * This hook is designed to work directly with TanStack Form's form.state.values,
 * which is stable and doesn't require useRef workarounds for parent state synchronization.
 *
 * @example
 * ```tsx
 * const form = useAppForm({
 *   schema: wizardSchema,
 *   defaultValues: mapInitialData(initialData),
 * })
 *
 * const { saveDraft, clearDraft } = useDraftAutoSave({
 *   draftKey: `client-app-draft-${realmId}-${mode}`,
 *   data: form.state.values,
 *   enabled: mode === 'create',
 * })
 * ```
 */
export function useDraftAutoSave<T extends Record<string, unknown>>({
  draftKey,
  data,
  enabled = true,
  saveInterval = DEFAULT_SAVE_INTERVAL,
  debounceDelay = DEFAULT_DEBOUNCE_DELAY,
  version = DEFAULT_VERSION,
  onSaveError,
}: UseDraftAutoSaveOptions<T>) {
  const saveTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const lastSavedDataRef = useRef<T | null>(null)
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const lastSavedHashRef = useRef<string>('')

  // Helper function to get current data
  const getCurrentData = useCallback((): T => {
    if (typeof data === 'function') {
      return (data as () => T)()
    }
    return data
  }, [data])

  console.log('[useDraftAutoSave] Hook called', {
    draftKey,
    enabled,
    isFunction: typeof data === 'function',
  })

  // Keep dataRef updated
  const dataRef = useRef<T>(getCurrentData())

  useEffect(() => {
    const currentData = getCurrentData()
    const dataChanged = JSON.stringify(currentData) !== JSON.stringify(dataRef.current)

    if (dataChanged) {
      console.log('[useDraftAutoSave] dataRef updated', {
        draftKey,
        dataChanged,
        previousData: dataRef.current,
        newData: currentData,
      })
      dataRef.current = currentData
    }
  }, [getCurrentData, draftKey])

  /**
   * Save draft data to localStorage
   */
  const saveDraft = useCallback(
    (dataToSave?: T) => {
      if (!enabled) {
        console.log('[useDraftAutoSave] saveDraft called but disabled')
        return
      }

      try {
        const dataToStore = dataToSave ?? dataRef.current

        console.log('[useDraftAutoSave] saveDraft called', {
          draftKey,
          hasDataToSave: !!dataToSave,
          dataSize: JSON.stringify(dataToStore).length,
        })

        // Only save if data has changed since last save
        if (
          lastSavedDataRef.current &&
          JSON.stringify(dataToStore) === JSON.stringify(lastSavedDataRef.current)
        ) {
          console.log('[useDraftAutoSave] Data unchanged, skipping save')
          return
        }

        console.log('[useDraftAutoSave] Saving data to localStorage')

        const draftData: DraftData<T> = {
          data: dataToStore,
          timestamp: Date.now(),
          version,
        }

        localStorage.setItem(draftKey, JSON.stringify(draftData))
        lastSavedDataRef.current = dataToStore
        console.log('[useDraftAutoSave] Data saved successfully')
      } catch (error) {
        const err = error instanceof Error ? error : new Error('Failed to save draft')
        console.error('[useDraftAutoSave] Failed to save draft:', err)

        // Handle quota exceeded error gracefully
        if (err.name === 'QuotaExceededError' || err.message.includes('quota')) {
          toast.error('Storage full. Draft could not be saved.', {
            duration: 5000,
            position: 'bottom-right',
          })
        }

        onSaveError?.(err)
      }
    },
    [draftKey, enabled, version, onSaveError]
  )

  /**
   * Clear draft from localStorage
   */
  const clearDraft = useCallback(() => {
    try {
      localStorage.removeItem(draftKey)
      lastSavedDataRef.current = null
    } catch (error) {
      console.error('Failed to clear draft:', error)
      onSaveError?.(error instanceof Error ? error : new Error('Failed to clear draft'))
    }
  }, [draftKey, onSaveError])

  /**
   * Load draft from localStorage
   */
  const loadDraft = useCallback((): DraftData<T> | null => {
    try {
      const stored = localStorage.getItem(draftKey)
      if (!stored) return null

      const draftData: DraftData<T> = JSON.parse(stored)

      // Validate draft structure
      if (!draftData.data || !draftData.timestamp || !draftData.version) {
        console.warn('Invalid draft data structure, clearing...')
        clearDraft()
        return null
      }

      // Check version compatibility (simple check for now)
      if (draftData.version !== version) {
        console.warn(`Draft version mismatch: expected ${version}, got ${draftData.version}`)
        // Could implement migration logic here
      }

      return draftData
    } catch (error) {
      console.error('Failed to load draft:', error)
      // Clear corrupted data
      clearDraft()
      return null
    }
  }, [draftKey, version, clearDraft])

  /**
   * Check if a draft exists
   */
  const hasDraft = useCallback((): boolean => {
    try {
      return localStorage.getItem(draftKey) !== null
    } catch {
      return false
    }
  }, [draftKey])

  /**
   * Get draft metadata without loading full data
   */
  const getDraftMetadata = useCallback((): DraftMetadata | null => {
    try {
      const stored = localStorage.getItem(draftKey)
      if (!stored) return null

      const draftData: DraftData<T> = JSON.parse(stored)
      return {
        draftKey,
        timestamp: draftData.timestamp,
        version: draftData.version,
      }
    } catch {
      return null
    }
  }, [draftKey])

  /**
   * Trigger debounced save on data changes
   */
  useEffect(() => {
    if (!enabled) return

    const currentData = getCurrentData()
    const currentHash = JSON.stringify(currentData)

    console.log('[useDraftAutoSave] Data change detected', {
      draftKey,
      hashLength: currentHash.length,
      previousHashLength: lastSavedHashRef.current.length,
      hashChanged: currentHash !== lastSavedHashRef.current,
    })

    // Skip if data hasn't actually changed
    if (currentHash === lastSavedHashRef.current) {
      console.log('[useDraftAutoSave] Hash unchanged, skipping save')
      return
    }

    lastSavedHashRef.current = currentHash
    console.log('[useDraftAutoSave] Hash changed, scheduling debounced save')

    // Clear previous timeout
    if (saveTimeoutRef.current) {
      clearTimeout(saveTimeoutRef.current)
    }

    // Set new timeout for debounced save
    saveTimeoutRef.current = setTimeout(() => {
      console.log('[useDraftAutoSave] Executing debounced save')
      saveDraft()
    }, debounceDelay)

    return () => {
      if (saveTimeoutRef.current) {
        clearTimeout(saveTimeoutRef.current)
      }
    }
  }, [enabled, debounceDelay, saveDraft, getCurrentData, draftKey])

  /**
   * Periodic save every N seconds
   */
  useEffect(() => {
    if (!enabled) return

    console.log('[useDraftAutoSave] Setting up periodic save interval', {
      draftKey,
      saveInterval,
    })

    intervalRef.current = setInterval(() => {
      const currentData = getCurrentData()
      console.log('[useDraftAutoSave] Executing periodic save', { data: currentData })
      saveDraft(currentData)
    }, saveInterval)

    return () => {
      if (intervalRef.current) {
        console.log('[useDraftAutoSave] Clearing periodic save interval')
        clearInterval(intervalRef.current)
      }
    }
  }, [enabled, saveInterval, saveDraft, getCurrentData, draftKey])

  /**
   * Cleanup on unmount
   */
  useEffect(
    () => {
      return () => {
        if (saveTimeoutRef.current) {
          clearTimeout(saveTimeoutRef.current)
        }
        if (intervalRef.current) {
          clearInterval(intervalRef.current)
        }
      }
    },
    [] // Only run on unmount
  )

  return {
    saveDraft,
    clearDraft,
    loadDraft,
    hasDraft,
    getDraftMetadata,
  }
}

/**
 * Get all drafts for a specific prefix (e.g., all client-app drafts)
 */
export function getAllDrafts(prefix: string): DraftMetadata[] {
  try {
    const drafts: DraftMetadata[] = []

    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i)
      if (key?.startsWith(prefix)) {
        const stored = localStorage.getItem(key)
        if (stored) {
          try {
            const draftData = JSON.parse(stored)
            drafts.push({
              draftKey: key,
              timestamp: draftData.timestamp,
              version: draftData.version,
            })
          } catch {
            // Skip corrupted drafts
            continue
          }
        }
      }
    }

    return drafts.sort((a, b) => b.timestamp - a.timestamp) // Sort by newest first
  } catch {
    return []
  }
}

/**
 * Format draft age for display
 */
export function formatDraftAge(timestamp: number): string {
  const now = Date.now()
  const diff = now - timestamp

  const seconds = Math.floor(diff / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)

  if (days > 0) {
    return `${days} day${days > 1 ? 's' : ''} ago`
  }
  if (hours > 0) {
    return `${hours} hour${hours > 1 ? 's' : ''} ago`
  }
  if (minutes > 0) {
    return `${minutes} minute${minutes > 1 ? 's' : ''} ago`
  }
  return 'Just now'
}
