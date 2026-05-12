/**
 * @vitest-environment jsdom
 */

import { renderHook, act, waitFor } from '@testing-library/react'
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { useDraftAutoSave, getAllDrafts, formatDraftAge } from '@/hooks/use-draft-autosave'

describe('useDraftAutoSave', () => {
  const mockDraftKey = 'test-draft-key'
  const testData = { name: 'Test App', description: 'Test Description' }

  beforeEach(() => {
    // Clear localStorage before each test
    localStorage.clear()
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  describe('basic functionality', () => {
    it('should save draft to localStorage', () => {
      const { result } = renderHook(() =>
        useDraftAutoSave({
          draftKey: mockDraftKey,
          data: testData,
          enabled: true,
        })
      )

      act(() => {
        result.current.saveDraft()
      })

      const stored = localStorage.getItem(mockDraftKey)
      expect(stored).toBeDefined()

      const draft = JSON.parse(stored!)
      expect(draft.data).toEqual(testData)
      expect(draft.timestamp).toBeDefined()
      expect(draft.version).toBeDefined()
    })

    it('should load draft from localStorage', () => {
      const draftData = {
        data: testData,
        timestamp: Date.now(),
        version: '1.0',
      }
      localStorage.setItem(mockDraftKey, JSON.stringify(draftData))

      const { result } = renderHook(() =>
        useDraftAutoSave({
          draftKey: mockDraftKey,
          data: {},
          enabled: true,
        })
      )

      const loaded = result.current.loadDraft()
      expect(loaded).toEqual(draftData)
    })

    it('should clear draft from localStorage', () => {
      const draftData = {
        data: testData,
        timestamp: Date.now(),
        version: '1.0',
      }
      localStorage.setItem(mockDraftKey, JSON.stringify(draftData))

      const { result } = renderHook(() =>
        useDraftAutoSave({
          draftKey: mockDraftKey,
          data: {},
          enabled: true,
        })
      )

      act(() => {
        result.current.clearDraft()
      })

      const stored = localStorage.getItem(mockDraftKey)
      expect(stored).toBeNull()
    })

    it('should check if draft exists', () => {
      const { result } = renderHook(() =>
        useDraftAutoSave({
          draftKey: mockDraftKey,
          data: testData,
          enabled: true,
        })
      )

      expect(result.current.hasDraft()).toBe(false)

      act(() => {
        result.current.saveDraft()
      })

      expect(result.current.hasDraft()).toBe(true)
    })
  })

  describe('auto-save behavior', () => {
    it('should auto-save on data changes after debounce delay', () => {
      const { result, rerender } = renderHook(
        ({ data }) =>
          useDraftAutoSave({
            draftKey: mockDraftKey,
            data,
            enabled: true,
            debounceDelay: 1000,
          }),
        { initialProps: { data: testData } }
      )

      // Fast data changes should not trigger immediate save
      rerender({ data: { ...testData, name: 'Updated 1' } })
      rerender({ data: { ...testData, name: 'Updated 2' } })

      expect(localStorage.getItem(mockDraftKey)).toBeNull()

      // Fast forward timers
      act(() => {
        vi.advanceTimersByTime(1000)
      })

      // After debounce, the save should have happened
      const stored = localStorage.getItem(mockDraftKey)
      expect(stored).toBeDefined()
      const draft = JSON.parse(stored!)
      expect(draft.data.name).toBe('Updated 2')
    })

    it('should auto-save periodically', () => {
      const saveInterval = 5000 // 5 seconds

      renderHook(() =>
        useDraftAutoSave({
          draftKey: mockDraftKey,
          data: testData,
          enabled: true,
          saveInterval,
        })
      )

      expect(localStorage.getItem(mockDraftKey)).toBeNull()

      // Fast forward past first save interval
      act(() => {
        vi.advanceTimersByTime(saveInterval)
      })

      // After interval, the save should have happened
      const stored = localStorage.getItem(mockDraftKey)
      expect(stored).toBeDefined()
    })

    it('should not save when disabled', () => {
      const { result } = renderHook(() =>
        useDraftAutoSave({
          draftKey: mockDraftKey,
          data: testData,
          enabled: false,
        })
      )

      act(() => {
        result.current.saveDraft()
      })

      expect(localStorage.getItem(mockDraftKey)).toBeNull()
    })
  })

  describe('error handling', () => {
    it('should handle corrupted data gracefully', () => {
      localStorage.setItem(mockDraftKey, 'invalid json')

      const { result } = renderHook(() =>
        useDraftAutoSave({
          draftKey: mockDraftKey,
          data: {},
          enabled: true,
        })
      )

      const loaded = result.current.loadDraft()
      expect(loaded).toBeNull()
      expect(localStorage.getItem(mockDraftKey)).toBeNull() // Should clear corrupted data
    })

    it('should handle invalid draft structure', () => {
      localStorage.setItem(mockDraftKey, JSON.stringify({ invalid: 'data' }))

      const { result } = renderHook(() =>
        useDraftAutoSave({
          draftKey: mockDraftKey,
          data: {},
          enabled: true,
        })
      )

      const loaded = result.current.loadDraft()
      expect(loaded).toBeNull()
    })

    it('should call onSaveError when save fails', () => {
      // Mock localStorage.setItem to throw error
      const setError = new Error('Storage full')
      vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
        throw setError
      })

      const onSaveError = vi.fn()

      const { result } = renderHook(() =>
        useDraftAutoSave({
          draftKey: mockDraftKey,
          data: testData,
          enabled: true,
          onSaveError,
        })
      )

      act(() => {
        result.current.saveDraft()
      })

      expect(onSaveError).toHaveBeenCalledWith(setError)

      vi.restoreAllMocks()
    })
  })

  describe('cleanup', () => {
    it('should clean up timers on unmount', () => {
      const clearTimeoutSpy = vi.spyOn(global, 'clearTimeout')
      const clearIntervalSpy = vi.spyOn(global, 'clearInterval')

      const { unmount } = renderHook(() =>
        useDraftAutoSave({
          draftKey: mockDraftKey,
          data: testData,
          enabled: true,
        })
      )

      unmount()

      expect(clearTimeoutSpy).toHaveBeenCalled()
      expect(clearIntervalSpy).toHaveBeenCalled()

      clearTimeoutSpy.mockRestore()
      clearIntervalSpy.mockRestore()
    })
  })
})

describe('getAllDrafts', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('should return all drafts with matching prefix', () => {
    const draft1 = { data: { name: 'App 1' }, timestamp: Date.now(), version: '1.0' }
    const draft2 = { data: { name: 'App 2' }, timestamp: Date.now() - 1000, version: '1.0' }
    const otherDraft = { data: { name: 'Other' }, timestamp: Date.now(), version: '1.0' }

    localStorage.setItem('client-app-draft-realm1-create-new', JSON.stringify(draft1))
    localStorage.setItem('client-app-draft-realm1-edit-123', JSON.stringify(draft2))
    localStorage.setItem('other-draft', JSON.stringify(otherDraft))

    const drafts = getAllDrafts('client-app-draft-realm1')

    expect(drafts).toHaveLength(2)
    expect(drafts[0].draftKey).toBe('client-app-draft-realm1-create-new')
    expect(drafts[1].draftKey).toBe('client-app-draft-realm1-edit-123')
  })

  it('should return drafts sorted by timestamp (newest first)', () => {
    const now = Date.now()
    const draft1 = { data: { name: 'App 1' }, timestamp: now - 2000, version: '1.0' }
    const draft2 = { data: { name: 'App 2' }, timestamp: now, version: '1.0' }
    const draft3 = { data: { name: 'App 3' }, timestamp: now - 1000, version: '1.0' }

    localStorage.setItem('draft-1', JSON.stringify(draft1))
    localStorage.setItem('draft-2', JSON.stringify(draft2))
    localStorage.setItem('draft-3', JSON.stringify(draft3))

    const drafts = getAllDrafts('draft')

    expect(drafts[0].draftKey).toBe('draft-2') // Newest
    expect(drafts[1].draftKey).toBe('draft-3') // Middle
    expect(drafts[2].draftKey).toBe('draft-1') // Oldest
  })
})

describe('formatDraftAge', () => {
  it('should format age correctly for various time ranges', () => {
    const now = Date.now()

    expect(formatDraftAge(now)).toBe('Just now')
    expect(formatDraftAge(now - 30000)).toBe('Just now') // 30 seconds
    expect(formatDraftAge(now - 60000)).toBe('1 minute ago')
    expect(formatDraftAge(now - 120000)).toBe('2 minutes ago')
    expect(formatDraftAge(now - 3600000)).toBe('1 hour ago')
    expect(formatDraftAge(now - 7200000)).toBe('2 hours ago')
    expect(formatDraftAge(now - 86400000)).toBe('1 day ago')
    expect(formatDraftAge(now - 172800000)).toBe('2 days ago')
  })
})
