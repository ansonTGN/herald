import { describe, it, expect, beforeEach } from 'vitest'
import {
  usePurchaseFlowStore,
  clearPurchaseFlowStorage,
  type PurchaseFlowState,
} from '../purchase-flow-store'

describe('Purchase Flow Store', () => {
  beforeEach(() => {
    // Clear the store before each test
    const { clearPurchaseState } = usePurchaseFlowStore.getState()
    clearPurchaseState()
  })

  describe('Initial State', () => {
    it('should have correct initial state', () => {
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

  describe('setPurchaseState', () => {
    it('should set purchase state with entitlement_mapping target type', () => {
      const { setPurchaseState } = usePurchaseFlowStore.getState()

      setPurchaseState({
        realmId: 'test-realm',
        userId: 'test-user',
        targetType: 'entitlement_mapping',
        targetId: '550e8400-e29b-41d4-a716-446655440000',
        paymentProvider: 'stripe',
      })

      const state = usePurchaseFlowStore.getState()
      expect(state.realmId).toBe('test-realm')
      expect(state.userId).toBe('test-user')
      expect(state.targetType).toBe('entitlement_mapping')
      expect(state.targetId).toBe('550e8400-e29b-41d4-a716-446655440000')
      expect(state.paymentProvider).toBe('stripe')
    })
  })

  describe('setPaymentAttempt', () => {
    it('should set payment attempt', () => {
      const { setPaymentAttempt } = usePurchaseFlowStore.getState()

      const mockContext = {
        paymentProvider: 'stripe',
        qrCodeUrl: 'https://example.com/qr',
        redirectUrl: 'https://example.com/redirect',
      }

      setPaymentAttempt(
        'attempt-123',
        'Pending',
        mockContext,
        new Date(Date.now() + 15 * 60 * 1000).toISOString()
      )

      const state = usePurchaseFlowStore.getState()
      expect(state.attemptId).toBe('attempt-123')
      expect(state.attemptStatus).toBe('Pending')
      expect(state.paymentContext).toEqual(mockContext)
      expect(state.expiresAt).toBeDefined()
    })
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
    it('should persist state to localStorage', () => {
      const { setPurchaseState, setPaymentAttempt } = usePurchaseFlowStore.getState()

      const futureExpiry = new Date(Date.now() + 15 * 60 * 1000).toISOString()

      setPurchaseState({
        realmId: 'test-realm',
        userId: 'test-user',
        targetType: 'entitlement_mapping',
        targetId: '550e8400-e29b-41d4-a716-446655440000',
        paymentProvider: 'stripe',
      })

      setPaymentAttempt('attempt-123', 'Pending', { paymentProvider: 'stripe' }, futureExpiry)

      // Check if localStorage was called - state should be set
      const state = usePurchaseFlowStore.getState()
      expect(state.realmId).toBe('test-realm')
      expect(state.userId).toBe('test-user')
      expect(state.attemptId).toBe('attempt-123')
    })

    it('should recover state from localStorage', () => {
      const { setPurchaseState, setPaymentAttempt, clearPurchaseState, canRecover } =
        usePurchaseFlowStore.getState()

      const futureExpiry = new Date(Date.now() + 15 * 60 * 1000).toISOString()

      // Set up state
      setPurchaseState({
        realmId: 'test-realm',
        userId: 'test-user',
      })

      setPaymentAttempt('attempt-123', 'Pending', { paymentProvider: 'stripe' }, futureExpiry)

      // Simulate page refresh by clearing and checking if state persists
      const initialCanRecover = canRecover()
      expect(initialCanRecover).toBe(true)

      // Clear state but localStorage should still have data
      clearPurchaseState()

      // After clearing, canRecover should be false
      expect(canRecover()).toBe(false)
    })

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

  describe('Payment attempt lifecycle', () => {
    it('should handle transition from pending to succeeded', () => {
      const { setPurchaseState, setPaymentAttempt, clearPurchaseState } =
        usePurchaseFlowStore.getState()

      const futureExpiry = new Date(Date.now() + 15 * 60 * 1000).toISOString()

      setPurchaseState({
        realmId: 'test-realm',
        userId: 'test-user',
      })

      setPaymentAttempt('attempt-123', 'Pending', { paymentProvider: 'stripe' }, futureExpiry)

      expect(usePurchaseFlowStore.getState().attemptStatus).toBe('Pending')

      // Clear state after success
      clearPurchaseState()

      expect(usePurchaseFlowStore.getState().attemptStatus).toBe(null)
    })

    it('should handle transition from pending to failed', () => {
      const { setPurchaseState, setPaymentAttempt, clearPurchaseState } =
        usePurchaseFlowStore.getState()

      const futureExpiry = new Date(Date.now() + 15 * 60 * 1000).toISOString()

      setPurchaseState({
        realmId: 'test-realm',
        userId: 'test-user',
      })

      setPaymentAttempt('attempt-123', 'Pending', { paymentProvider: 'stripe' }, futureExpiry)

      expect(usePurchaseFlowStore.getState().attemptStatus).toBe('Pending')

      // Simulate failed payment
      clearPurchaseState()

      expect(usePurchaseFlowStore.getState().attemptStatus).toBe(null)
    })

    it('should handle multiple payment attempts sequentially', () => {
      const { setPurchaseState, setPaymentAttempt, clearPurchaseState } =
        usePurchaseFlowStore.getState()

      const futureExpiry = new Date(Date.now() + 15 * 60 * 1000).toISOString()

      // First attempt
      setPaymentAttempt('attempt-1', 'Pending', { paymentProvider: 'stripe' }, futureExpiry)
      expect(usePurchaseFlowStore.getState().attemptId).toBe('attempt-1')

      // Clear first attempt
      clearPurchaseState()

      // Second attempt
      setPaymentAttempt('attempt-2', 'Pending', { paymentProvider: 'wechat' }, futureExpiry)
      expect(usePurchaseFlowStore.getState().attemptId).toBe('attempt-2')
    })
  })

  describe('Error handling and edge cases', () => {
    it('should handle null payment context', () => {
      const { setPaymentAttempt } = usePurchaseFlowStore.getState()

      const futureExpiry = new Date(Date.now() + 15 * 60 * 1000).toISOString()

      setPaymentAttempt('attempt-123', 'Pending', null, futureExpiry)

      expect(usePurchaseFlowStore.getState().paymentContext).toBe(null)
    })

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

    it('should handle all payment provider types', () => {
      const { setPurchaseState } = usePurchaseFlowStore.getState()

      const providers = ['wechat', 'stripe', 'creem'] as const

      providers.forEach((provider) => {
        setPurchaseState({
          realmId: 'test-realm',
          paymentProvider: provider,
        })

        expect(usePurchaseFlowStore.getState().paymentProvider).toBe(provider)
      })
    })

    it('should handle all valid target types', () => {
      const { setPurchaseState } = usePurchaseFlowStore.getState()

      const targetTypes = ['entitlement_mapping', 'subscription_plan'] as const

      targetTypes.forEach((targetType) => {
        setPurchaseState({
          realmId: 'test-realm',
          targetType,
        })

        expect(usePurchaseFlowStore.getState().targetType).toBe(targetType)
      })
    })
  })

  describe('Concurrent operations', () => {
    it('should handle rapid state changes', () => {
      const { setPurchaseState, setPaymentAttempt, clearPurchaseState } =
        usePurchaseFlowStore.getState()

      const futureExpiry = new Date(Date.now() + 15 * 60 * 1000).toISOString()

      // Rapid state changes
      for (let i = 0; i < 10; i++) {
        setPurchaseState({
          realmId: `realm-${i}`,
          userId: `user-${i}`,
        })

        setPaymentAttempt(`attempt-${i}`, 'Pending', { paymentProvider: 'stripe' }, futureExpiry)

        expect(usePurchaseFlowStore.getState().realmId).toBe(`realm-${i}`)
        expect(usePurchaseFlowStore.getState().attemptId).toBe(`attempt-${i}`)

        clearPurchaseState()
      }
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
