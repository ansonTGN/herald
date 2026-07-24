import { describe, it, expect, beforeEach } from 'vitest'
import { usePurchaseFlowStore, clearPurchaseFlowStorage } from '../purchase-flow-store'

describe('Purchase Flow Store', () => {
  beforeEach(() => {
    // Clear the store before each test
    const { clearPurchaseState } = usePurchaseFlowStore.getState()
    clearPurchaseState()
  })

  describe('clearPurchaseState', () => {
    it('should clear all state', () => {
      const { setPurchaseState, clearPurchaseState } = usePurchaseFlowStore.getState()

      setPurchaseState({
        realmId: 'test-realm',
        userId: 'test-user',
        targetType: 'entitlement_mapping',
        targetId: '550e8400-e29b-41d4-a716-446655440000',
        paymentProvider: 'stripe',
      })

      clearPurchaseState()

      const state = usePurchaseFlowStore.getState()
      expect(state.realmId).toBe(null)
      expect(state.userId).toBe(null)
      expect(state.targetType).toBe(null)
      expect(state.targetId).toBe(null)
      expect(state.paymentProvider).toBe(null)
      expect(state.attemptId).toBe(null)
      expect(state.attemptStatus).toBe(null)
      expect(state.paymentContext).toBe(null)
      expect(state.expiresAt).toBe(null)
    })
  })

  describe('isExpired', () => {
    it('should return true when expired', () => {
      const { setPaymentAttempt, isExpired } = usePurchaseFlowStore.getState()

      const pastExpiry = new Date(Date.now() - 1000).toISOString()

      setPaymentAttempt('attempt-123', 'Pending', { paymentProvider: 'stripe' }, pastExpiry)

      expect(isExpired()).toBe(true)
    })

    it('should return false when not expired', () => {
      const { setPaymentAttempt, isExpired } = usePurchaseFlowStore.getState()

      const futureExpiry = new Date(Date.now() + 15 * 60 * 1000).toISOString()

      setPaymentAttempt('attempt-123', 'Pending', { paymentProvider: 'stripe' }, futureExpiry)

      expect(isExpired()).toBe(false)
    })

    it('should return true when no expiry set', () => {
      const { isExpired } = usePurchaseFlowStore.getState()

      expect(isExpired()).toBe(true)
    })
  })

  describe('canRecover', () => {
    it('should return true for valid recoverable state', () => {
      const { setPurchaseState, setPaymentAttempt, canRecover } = usePurchaseFlowStore.getState()

      const futureExpiry = new Date(Date.now() + 15 * 60 * 1000).toISOString()

      setPurchaseState({
        realmId: 'test-realm',
        userId: 'test-user',
      })

      setPaymentAttempt('attempt-123', 'Pending', { paymentProvider: 'stripe' }, futureExpiry)

      expect(canRecover()).toBe(true)
    })

    it('should return false when no attempt exists', () => {
      const { canRecover } = usePurchaseFlowStore.getState()

      expect(canRecover()).toBe(false)
    })

    it('should return false for expired attempts', () => {
      const { setPurchaseState, setPaymentAttempt, canRecover } = usePurchaseFlowStore.getState()

      const pastExpiry = new Date(Date.now() - 1000).toISOString()

      setPurchaseState({
        realmId: 'test-realm',
        userId: 'test-user',
      })

      setPaymentAttempt('attempt-123', 'Pending', { paymentProvider: 'stripe' }, pastExpiry)

      expect(canRecover()).toBe(false)
    })

    it('should return false for completed attempts', () => {
      const { setPurchaseState, setPaymentAttempt, canRecover } = usePurchaseFlowStore.getState()

      const futureExpiry = new Date(Date.now() + 15 * 60 * 1000).toISOString()

      setPurchaseState({
        realmId: 'test-realm',
        userId: 'test-user',
      })

      setPaymentAttempt('attempt-123', 'Succeeded', { paymentProvider: 'stripe' }, futureExpiry)

      expect(canRecover()).toBe(false)
    })

    it('should return false when missing critical fields', () => {
      const { setPurchaseState, setPaymentAttempt, canRecover } = usePurchaseFlowStore.getState()

      const futureExpiry = new Date(Date.now() + 15 * 60 * 1000).toISOString()

      setPaymentAttempt('attempt-123', 'Pending', { paymentProvider: 'stripe' }, futureExpiry)

      // No realmId or userId set
      expect(canRecover()).toBe(false)
    })

    it('should recover with entitlement_mapping target type', () => {
      const { setPurchaseState, setPaymentAttempt, canRecover } = usePurchaseFlowStore.getState()

      const futureExpiry = new Date(Date.now() + 15 * 60 * 1000).toISOString()

      setPurchaseState({
        realmId: 'test-realm',
        userId: 'test-user',
        targetType: 'entitlement_mapping',
        targetId: '550e8400-e29b-41d4-a716-446655440000',
      })

      setPaymentAttempt('attempt-123', 'Pending', { paymentProvider: 'creem' }, futureExpiry)

      expect(canRecover()).toBe(true)

      const state = usePurchaseFlowStore.getState()
      expect(state.targetType).toBe('entitlement_mapping')
      expect(state.targetId).toBe('550e8400-e29b-41d4-a716-446655440000')
    })

    it('should recover RequiresAction status with entitlement_mapping', () => {
      const { setPurchaseState, setPaymentAttempt, canRecover } = usePurchaseFlowStore.getState()

      const futureExpiry = new Date(Date.now() + 15 * 60 * 1000).toISOString()

      setPurchaseState({
        realmId: 'test-realm',
        userId: 'test-user',
        targetType: 'entitlement_mapping',
        targetId: '550e8400-e29b-41d4-a716-446655440000',
      })

      setPaymentAttempt(
        'attempt-456',
        'RequiresAction',
        { paymentProvider: 'stripe' },
        futureExpiry
      )

      expect(canRecover()).toBe(true)
    })
  })

  describe('State persistence and recovery', () => {
    it('should handle corrupted localStorage data', () => {
      // Store invalid JSON in localStorage
      localStorage.setItem('purchase-flow-storage', 'invalid-json{')

      // This should not throw an error
      expect(() => {
        const state = usePurchaseFlowStore.getState()
        expect(state.realmId).toBe(null)
      }).not.toThrow()
    })

    it('should handle missing localStorage gracefully', () => {
      // Remove localStorage item
      localStorage.removeItem('purchase-flow-storage')

      // This should not throw an error
      expect(() => {
        const state = usePurchaseFlowStore.getState()
        expect(state.realmId).toBe(null)
      }).not.toThrow()
    })

    it('should clear stale points_package state on rehydration', () => {
      // Simulate stale persisted state from an old deployment where
      // targetType was still 'points_package'. The rehydration guard
      // in onRehydrateStorage must detect this and clear all purchase state.
      const staleState = JSON.stringify({
        state: {
          realmId: 'test-realm',
          userId: 'test-user',
          targetType: 'points_package',
          targetId: 'old-package-id',
          paymentProvider: 'stripe',
          attemptId: null,
          attemptStatus: null,
          paymentContext: null,
          expiresAt: null,
        },
        version: 0,
      })
      localStorage.setItem('cas-purchase-flow', staleState)

      // Trigger rehydration from the stale localStorage data
      usePurchaseFlowStore.persist.rehydrate()

      const state = usePurchaseFlowStore.getState()
      expect(state.targetType).toBeNull()
      expect(state.targetId).toBeNull()
      expect(state.realmId).toBeNull()
      expect(state.userId).toBeNull()
    })
  })

  describe('Error handling and edge cases', () => {
    it('should handle undefined expiry date', () => {
      const { setPaymentAttempt, isExpired } = usePurchaseFlowStore.getState()

      setPaymentAttempt('attempt-123', 'Pending', { paymentProvider: 'stripe' }, undefined)

      expect(isExpired()).toBe(true)
    })

    it('should handle invalid expiry date format', () => {
      const { setPaymentAttempt, isExpired } = usePurchaseFlowStore.getState()

      setPaymentAttempt('attempt-123', 'Pending', { paymentProvider: 'stripe' }, 'invalid-date')

      // Should handle gracefully without throwing
      expect(() => isExpired()).not.toThrow()
    })

    it('should handle partial state updates', () => {
      const { setPurchaseState } = usePurchaseFlowStore.getState()

      // Set partial state
      setPurchaseState({
        realmId: 'test-realm',
      })

      expect(usePurchaseFlowStore.getState().realmId).toBe('test-realm')
      expect(usePurchaseFlowStore.getState().userId).toBe(null)

      // Update with more fields
      setPurchaseState({
        userId: 'test-user',
      })

      expect(usePurchaseFlowStore.getState().realmId).toBe('test-realm')
      expect(usePurchaseFlowStore.getState().userId).toBe('test-user')
    })
  })

  describe('clearPurchaseFlowStorage', () => {
    it('should clear localStorage', () => {
      const { setPurchaseState } = usePurchaseFlowStore.getState()

      setPurchaseState({
        realmId: 'test-realm',
        userId: 'test-user',
      })

      expect(localStorage.getItem('purchase-flow-storage')).toBeDefined()

      clearPurchaseFlowStorage()

      expect(localStorage.getItem('purchase-flow-storage')).toBeNull()
    })

    it('should handle clearing empty localStorage', () => {
      expect(() => {
        clearPurchaseFlowStorage()
        clearPurchaseFlowStorage() // Clear again
      }).not.toThrow()
    })
  })
})
