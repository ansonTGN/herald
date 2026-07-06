/**
 * Purchase Flow Store (Zustand)
 *
 * Centralized state management for unified purchase flow.
 * Handles state persistence for page refresh recovery and payment polling.
 */

import { create } from 'zustand'
import { persist, devtools } from 'zustand/middleware'
import { useShallow } from 'zustand/react/shallow'
import type { PaymentContextResponse } from '@/lib/api-generated'

/**
 * Valid target types for purchase flow.
 * Used to reject stale persisted values from old versions (e.g. 'points_package').
 */
const VALID_TARGET_TYPES = ['subscription_plan', 'entitlement_mapping'] as const

type TargetType = (typeof VALID_TARGET_TYPES)[number]

export interface PurchaseFlowState {
  // Current purchase state
  realmId: string | null
  userId: string | null
  targetType: TargetType | null
  targetId: string | null
  paymentProvider: string | null

  // Payment attempt state
  attemptId: string | null
  attemptStatus: string | null
  paymentContext: PaymentContextResponse | null
  expiresAt: string | null
}

/**
 * Purchase flow actions
 */
export interface PurchaseFlowActions {
  // State management
  setPurchaseState: (state: Partial<PurchaseFlowState>) => void
  clearPurchaseState: () => void

  // Payment attempt management
  setPaymentAttempt: (
    attemptId: string,
    status: string,
    context: PaymentContextResponse,
    expiresAt: string
  ) => void

  // Validation helpers
  isExpired: () => boolean
  canRecover: () => boolean
}

/**
 * Storage key for persist middleware
 */
const PURCHASE_FLOW_STORAGE_KEY = 'cas-purchase-flow'
const PURCHASE_FLOW_STORE_NAME = 'purchase-flow'

/**
 * Initial state
 */
const initialState: PurchaseFlowState = {
  realmId: null,
  userId: null,
  targetType: null,
  targetId: null,
  paymentProvider: null,
  attemptId: null,
  attemptStatus: null,
  paymentContext: null,
  expiresAt: null,
}

/**
 * Validate state on load to prevent corrupted state
 * Returns null if state is invalid, otherwise returns cleaned state
 */
function validateState(state: PurchaseFlowState): Partial<PurchaseFlowState> | null {
  // Clear state if critical fields are missing
  if (!state.realmId || !state.userId) {
    return null
  }

  // Stale target type from a removed feature — the entire purchase state is invalid
  if (
    state.targetType &&
    !VALID_TARGET_TYPES.includes(state.targetType as (typeof VALID_TARGET_TYPES)[number])
  ) {
    return null
  }

  // Clear state if attempt exists but payment context is missing
  if (state.attemptId && !state.paymentContext) {
    return {
      ...state,
      attemptId: null,
      attemptStatus: null,
      paymentContext: null,
      expiresAt: null,
    }
  }

  return state
}

/**
 * Create the purchase flow store
 */
export const usePurchaseFlowStore = create<PurchaseFlowState & PurchaseFlowActions>()(
  devtools(
    persist(
      (set, get) => ({
        ...initialState,

        // State management
        setPurchaseState: (newState) =>
          set((state) => {
            // Check if any values actually changed
            const hasChanges = Object.keys(newState).some(
              (key) =>
                state[key as keyof PurchaseFlowState] !== newState[key as keyof PurchaseFlowState]
            )

            // Skip update if nothing changed
            if (!hasChanges) return state

            return { ...state, ...newState }
          }),

        clearPurchaseState: () =>
          set({
            ...initialState,
          }),

        // Payment attempt management
        setPaymentAttempt: (attemptId, status, context, expiresAt) =>
          set({
            attemptId,
            attemptStatus: status,
            paymentContext: context,
            expiresAt,
          }),

        // Validation helpers
        isExpired: () => {
          const { expiresAt } = get()
          if (!expiresAt) return true

          try {
            const now = Date.now()
            const expires = new Date(expiresAt).getTime()
            return expires <= now
          } catch {
            return true
          }
        },

        canRecover: () => {
          const { attemptId, attemptStatus, expiresAt, realmId, userId } = get()

          // Only recover if all conditions are met
          const now = Date.now()
          let expires = 0
          try {
            expires = expiresAt ? new Date(expiresAt).getTime() : 0
          } catch {
            return false
          }

          return !!(
            attemptId &&
            (attemptStatus === 'Pending' || attemptStatus === 'RequiresAction') &&
            expires > now &&
            realmId &&
            userId
          )
        },
      }),
      {
        name: PURCHASE_FLOW_STORAGE_KEY,
        partialize: (state) => ({
          // Persist all state except loading/transient state
          realmId: state.realmId,
          userId: state.userId,
          targetType: state.targetType,
          targetId: state.targetId,
          paymentProvider: state.paymentProvider,
          attemptId: state.attemptId,
          attemptStatus: state.attemptStatus,
          paymentContext: state.paymentContext,
          expiresAt: state.expiresAt,
        }),
        onRehydrateStorage: () => (state) => {
          if (!state) return
          const validated = validateState(state)
          if (!validated) {
            state.realmId = null
            state.userId = null
            state.targetType = null
            state.targetId = null
            state.paymentProvider = null
            state.attemptId = null
            state.attemptStatus = null
            state.paymentContext = null
            state.expiresAt = null
          } else if (validated !== state) {
            state.targetType = validated.targetType ?? null
            state.targetId = validated.targetId ?? null
            state.paymentProvider = validated.paymentProvider ?? null
            state.attemptId = validated.attemptId ?? null
            state.attemptStatus = validated.attemptStatus ?? null
            state.paymentContext = validated.paymentContext ?? null
            state.expiresAt = validated.expiresAt ?? null
          }
        },
      }
    ),
    { name: PURCHASE_FLOW_STORE_NAME }
  )
)

/**
 * Get the persist storage instance to clear storage
 */
const persistStorage = usePurchaseFlowStore.persist

/**
 * Clear all persisted purchase flow data from storage
 */
export function clearPurchaseFlowStorage(): void {
  persistStorage.clearStorage()
}

/**
 * Selector hooks for optimized re-renders
 */

/**
 * Get current purchase state
 */
export const usePurchaseState = () =>
  usePurchaseFlowStore(
    useShallow((state) => ({
      realmId: state.realmId,
      userId: state.userId,
      targetType: state.targetType,
      targetId: state.targetId,
      paymentProvider: state.paymentProvider,
    }))
  )

/**
 * Get payment attempt state
 */
export const usePaymentAttempt = () =>
  usePurchaseFlowStore(
    useShallow((state) => ({
      attemptId: state.attemptId,
      attemptStatus: state.attemptStatus,
      paymentContext: state.paymentContext,
      expiresAt: state.expiresAt,
    }))
  )

/**
 * Get actions
 */
export const usePurchaseFlowActions = () =>
  usePurchaseFlowStore(
    useShallow((state) => ({
      setPurchaseState: state.setPurchaseState,
      clearPurchaseState: state.clearPurchaseState,
      setPaymentAttempt: state.setPaymentAttempt,
      isExpired: state.isExpired,
      canRecover: state.canRecover,
    }))
  )
