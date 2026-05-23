/**
 * Authentication Hook
 *
 * Unified hook for accessing authentication state and actions.
 * Provides a convenient API for components to interact with auth state.
 */

import { useShallow } from 'zustand/react/shallow'
import { useAuthStore, useAuthActions, usePermissions } from '@/stores/auth-store'
import {
  hasAdminPermission,
  DEFAULT_USER_REDIRECT,
  DEFAULT_ADMIN_REDIRECT,
} from '@/lib/constants/auth-constants'

/**
 * Authentication hook return type
 */
export interface UseAuthReturn {
  // State
  isAuthenticated: boolean
  isLoading: boolean
  realmId: string | null
  user: import('@/lib/api-generated').UserProfile | null
  permissions: string[]
  roles: string[]

  // Computed properties
  hasAdminPermission: boolean

  // Actions
  login: (realmId: string) => void
  logout: () => void
  setAuthStatus: (authenticated: boolean, realmId?: string) => void
  setIsLoading: (isLoading: boolean) => void
  setUserPermissions: (permissions: string[], roles: string[]) => void
  setUserProfile: (user: import('@/lib/api-generated').UserProfile | null) => void
  reset: () => void
}

/**
 * Hook for accessing authentication state and actions
 *
 * @returns Authentication state and actions
 */
export function useAuth(): UseAuthReturn {
  const [isAuthenticated, isLoading, realmId, user, permissions, roles] = useAuthStore(
    useShallow((state) => [
      state.isAuthenticated,
      state.isLoading,
      state.realmId,
      state.user,
      state.permissions,
      state.roles,
    ])
  )

  const actions = useAuthActions()

  // Compute derived properties directly in the return
  return {
    isAuthenticated,
    isLoading,
    realmId,
    user,
    permissions,
    roles,
    hasAdminPermission: hasAdminPermission(permissions),
    ...actions,
  }
}

/**
 * Hook for accessing authentication actions only
 *
 * @returns Authentication actions
 */
export function useAuthActionsOnly() {
  return useAuthActions()
}

/**
 * Hook for getting redirect path based on permissions
 *
 * @returns The appropriate redirect path ('/manage' for admins, '/user/profile' for regular users)
 */
export function useRedirectPath(): string {
  const permissions = usePermissions()
  return hasAdminPermission(permissions) ? DEFAULT_ADMIN_REDIRECT : DEFAULT_USER_REDIRECT
}
