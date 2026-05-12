/**
 * Authentication Store (Zustand)
 *
 * Centralized state management for authentication and authorization.
 * Uses DevTools integration for debugging and persist middleware for localStorage.
 */

import { create } from 'zustand'
import { persist, devtools } from 'zustand/middleware'
import { useShallow } from 'zustand/react/shallow'
import type { UserProfile } from '@/lib/api-generated'
import { AUTH_STORAGE_KEY, AUTH_STORE_NAME } from '@/lib/constants/auth-constants'

/**
 * Authentication state
 */
export interface AuthState {
  // Authentication status
  isAuthenticated: boolean
  isLoading: boolean
  realmId: string | null

  // User data
  user: UserProfile | null
  permissions: string[]
  roles: string[]
}

/**
 * Authentication actions
 */
export interface AuthActions {
  // Status actions
  setAuthStatus: (authenticated: boolean, realmId?: string) => void
  setIsLoading: (isLoading: boolean) => void

  // User data actions
  setUserPermissions: (permissions: string[], roles: string[]) => void
  setUserProfile: (user: UserProfile | null) => void

  // Auth flow actions
  login: (realmId: string) => void
  logout: () => void

  // Store actions
  reset: () => void
  clearStorage: () => void
}

/**
 * Initial state
 */
const initialState: AuthState = {
  isAuthenticated: false,
  isLoading: false,
  realmId: null,
  user: null,
  permissions: [],
  roles: [],
}

/**
 * Create the authentication store
 */
export const useAuthStore = create<AuthState & AuthActions>()(
  devtools(
    persist(
      (set, get) => ({
        ...initialState,

        // Status actions
        setAuthStatus: (authenticated, realmId) =>
          set({ isAuthenticated: authenticated, realmId: realmId ?? get().realmId }),

        setIsLoading: (isLoading) => set({ isLoading }),

        // User data actions
        setUserPermissions: (permissions, roles) => set({ permissions, roles }),

        setUserProfile: (user) => set({ user }),

        // Auth flow actions
        login: (realmId) =>
          set({
            isAuthenticated: true,
            realmId,
          }),

        logout: () =>
          set({
            isAuthenticated: false,
            isLoading: false,
            user: null,
            permissions: [],
            roles: [],
          }),

        // Store actions
        reset: () => set(initialState),

        clearStorage: () => {
          set(initialState)
        },
      }),
      {
        name: AUTH_STORAGE_KEY,
        partialize: (state) => ({
          // Persist all state except loading state
          isAuthenticated: state.isAuthenticated,
          realmId: state.realmId,
          user: state.user,
          permissions: state.permissions,
          roles: state.roles,
        }),
      }
    ),
    { name: AUTH_STORE_NAME }
  )
)

/**
 * Get the persist storage instance to clear storage
 * This is needed for proper logout that clears both state and storage
 */
const persistStorage = useAuthStore.persist

/**
 * Clear all persisted auth data from storage
 */
export function clearAuthStorage(): void {
  persistStorage.clearStorage()
}

/**
 * Selector hooks for optimized re-renders
 */

/**
 * Get authentication status
 */
export const useIsAuthenticated = () => useAuthStore((state) => state.isAuthenticated)

/**
 * Get loading state
 */
export const useIsLoading = () => useAuthStore((state) => state.isLoading)

/**
 * Get user data
 */
export const useUser = () => useAuthStore((state) => state.user)

/**
 * Get permissions
 */
export const usePermissions = () => useAuthStore((state) => state.permissions)

/**
 * Get roles
 */
export const useRoles = () => useAuthStore((state) => state.roles)

/**
 * Get realm ID
 */
export const useRealmId = () => useAuthStore((state) => state.realmId || 'admin')

/**
 * Get actions
 */
export const useAuthActions = () =>
  useAuthStore(
    useShallow((state) => ({
      setAuthStatus: state.setAuthStatus,
      setIsLoading: state.setIsLoading,
      setUserPermissions: state.setUserPermissions,
      setUserProfile: state.setUserProfile,
      login: state.login,
      logout: state.logout,
      reset: state.reset,
    }))
  )
