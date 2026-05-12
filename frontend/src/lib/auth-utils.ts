/**
 * Authentication Utilities
 *
 * Helper functions for authentication flows.
 * These functions coordinate between the auth service and Zustand store
 * to provide convenient APIs for authentication operations.
 */

import type { LoginRequestPayload, LoginResponse } from '@/lib/api-generated'
import { fetchAuthData, performLogin, performLogout } from '@/lib/auth-service'
import { useAuthStore, clearAuthStorage } from '@/stores/auth-store'
import {
  hasAdminPermission,
  DEFAULT_USER_REDIRECT,
  DEFAULT_ADMIN_REDIRECT,
  getSafeRedirectPath,
} from '@/lib/constants/auth-constants'

/**
 * Result object for login flow
 */
export interface LoginFlowResult {
  response: LoginResponse
  redirectPath: string
}

/**
 * Initialize authentication state
 * Fetches auth data from the API and populates the Zustand store
 *
 * @param realmId - The realm ID to initialize auth for
 * @param forceRefresh - Force a fresh fetch even if already authenticated (default: false)
 * @returns Object containing auth status and redirect path
 */
export async function initializeAuth(
  realmId: string,
  forceRefresh: boolean = false
): Promise<{
  authenticated: boolean
  redirectPath: string
}> {
  console.log(
    '[initializeAuth] Starting auth initialization for realm:',
    realmId,
    'forceRefresh:',
    forceRefresh
  )

  const store = useAuthStore.getState()

  // Always fetch fresh auth data from API
  store.setIsLoading(true)

  try {
    console.log('[initializeAuth] Fetching auth data from API...')
    const { authStatus, userPermissions, userProfile } = await fetchAuthData(realmId)

    console.log('[initializeAuth] Auth data fetched:', {
      authenticated: authStatus.authenticated,
      permissions: userPermissions.permissions,
      roles: userPermissions.roles,
    })

    // Update store with fetched data
    store.setAuthStatus(authStatus.authenticated, authStatus.realmId || realmId)
    store.setUserPermissions(userPermissions.permissions, userPermissions.roles)
    store.setUserProfile(userProfile)

    // Determine redirect path based on permissions
    const redirectPath = hasAdminPermission(userPermissions.permissions)
      ? DEFAULT_ADMIN_REDIRECT
      : DEFAULT_USER_REDIRECT

    console.log('[initializeAuth] Auth initialization completed, redirectPath:', redirectPath)

    return {
      authenticated: authStatus.authenticated,
      redirectPath,
    }
  } catch (error) {
    console.error('[initializeAuth] Error during auth initialization:', error)
    // On error, clear auth state
    store.reset()
    return {
      authenticated: false,
      redirectPath: DEFAULT_USER_REDIRECT,
    }
  } finally {
    store.setIsLoading(false)
  }
}

/**
 * Refresh authentication data
 * Forces a fresh fetch of auth data from the API
 *
 * @param realmId - The realm ID to refresh auth for
 * @returns Object containing auth status and redirect path
 */
export async function refreshAuth(realmId: string): Promise<{
  authenticated: boolean
  redirectPath: string
}> {
  return await initializeAuth(realmId)
}

/**
 * Login flow
 * Handles the complete login process including API call and state update
 *
 * @param realmId - The realm ID to login to
 * @param credentials - Login credentials
 * @returns Login response data
 * @throws Error if login fails or requires TOTP
 */
export async function loginFlow(
  realmId: string,
  credentials: LoginRequestPayload
): Promise<LoginFlowResult> {
  const store = useAuthStore.getState()

  console.log('[loginFlow] Starting login flow for realm:', realmId)
  console.log('[loginFlow] Store state before login:', {
    isAuthenticated: store.isAuthenticated,
    permissions: store.permissions,
  })

  try {
    // Perform login API call
    const loginResponse = await performLogin(realmId, credentials)

    console.log('[loginFlow] Login API response:', {
      requiresTotp: loginResponse.requiresTotp,
      realmId: loginResponse.realmId,
    })

    // Check if TOTP is required
    if (loginResponse.requiresTotp) {
      console.log('[loginFlow] TOTP required, returning early')
      // Return early - caller should handle TOTP verification
      return { response: loginResponse, redirectPath: DEFAULT_USER_REDIRECT }
    }

    // Get the user's actual realm from the response
    const userRealmId = loginResponse.realmId || realmId

    console.log('[loginFlow] Updating store with login state')
    // Update store with login state
    store.login(userRealmId)

    console.log('[loginFlow] Fetching auth data...')
    // Fetch and populate auth data
    const { authStatus, userPermissions, userProfile } = await fetchAuthData(userRealmId)

    console.log('[loginFlow] Auth data fetched:', {
      authenticated: authStatus.authenticated,
      permissions: userPermissions.permissions,
      roles: userPermissions.roles,
    })

    // Update store with fetched data
    console.log('[loginFlow] Updating store with auth status and permissions')
    store.setAuthStatus(authStatus.authenticated, authStatus.realmId || userRealmId)
    store.setUserPermissions(userPermissions.permissions, userPermissions.roles)
    store.setUserProfile(userProfile)

    // Determine redirect path based on permissions (using the freshly fetched data)
    const redirectPath = hasAdminPermission(userPermissions.permissions)
      ? DEFAULT_ADMIN_REDIRECT
      : DEFAULT_USER_REDIRECT

    console.log('[loginFlow] Redirect path determined:', redirectPath)
    console.log('[loginFlow] Login flow completed successfully')

    return { response: loginResponse, redirectPath }
  } catch (error) {
    console.error('[loginFlow] Error during login flow:', error)
    // On error, ensure store is in clean state
    store.logout()
    throw error
  }
}

/**
 * Logout flow
 * Handles the complete logout process including API call, state reset, storage cleanup, and navigation
 *
 * @param realmId - The realm ID to logout from
 */
export async function logoutFlow(realmId: string): Promise<void> {
  const store = useAuthStore.getState()
  store.setIsLoading(true)

  try {
    // Perform logout API call - this will clear the session cookie
    await performLogout(realmId)
  } catch (error) {
    // Log the error but continue with state cleanup
    console.error('Logout API call failed:', error)
  } finally {
    // Always reset the store and clear persisted storage
    store.reset()
    store.setIsLoading(false)

    // Clear localStorage to ensure all auth data is removed
    clearAuthStorage()

    // Navigate to login page - use window.location for simple redirect
    // since we need to reload the page to properly clear auth state
    window.location.href = `/${realmId}/auth/login`
  }
}

/**
 * Check if user has admin permission
 * Uses the current state from the Zustand store
 *
 * @returns true if user has admin permission
 */
export function checkAdminPermission(): boolean {
  const { permissions } = useAuthStore.getState()
  return hasAdminPermission(permissions)
}

/**
 * Get redirect path based on current permissions
 * Uses the current state from the Zustand store
 *
 * @returns The appropriate redirect path
 */
export function getRedirectPath(): string {
  console.log('[getRedirectPath] Calculating redirect path')
  const { permissions } = useAuthStore.getState()
  console.log('[getRedirectPath] Current permissions in store:', permissions)
  const hasAdmin = checkAdminPermission()
  console.log('[getRedirectPath] Has admin permission:', hasAdmin)
  const redirectPath = hasAdmin ? DEFAULT_ADMIN_REDIRECT : DEFAULT_USER_REDIRECT
  console.log('[getRedirectPath] Final redirect path:', redirectPath)
  return redirectPath
}

/**
 * Get safe redirect path
 * Validates the redirect path and returns a safe fallback if invalid
 *
 * @param redirectPath - The requested redirect path
 * @param fallback - The fallback path (defaults to user profile)
 * @returns The safe redirect path
 */
/**
 * Get safe redirect path
 * Validates redirect path and returns a safe fallback if invalid
 *
 * @param redirectPath - The requested redirect path
 * @param fallback - The fallback path (defaults to user profile)
 * @returns The safe redirect path
 */
export function getSafeRedirect(
  redirectPath: string | undefined,
  fallback: string = DEFAULT_USER_REDIRECT
): string {
  return getSafeRedirectPath(redirectPath, fallback)
}

/**
 * Complete login after TOTP verification
 * Should be called after successful TOTP verification
 *
 * @param realmId - The realm ID
 */
export async function completeLoginAfterTotp(realmId: string): Promise<string> {
  const store = useAuthStore.getState()

  try {
    // Fetch and populate auth data
    const { authStatus, userPermissions, userProfile } = await fetchAuthData(realmId)

    // Update store with fetched data
    store.setAuthStatus(authStatus.authenticated, authStatus.realmId || realmId)
    store.setUserPermissions(userPermissions.permissions, userPermissions.roles)
    store.setUserProfile(userProfile)

    // Determine redirect path based on permissions
    return getRedirectPath()
  } catch (error) {
    store.logout()
    throw error
  }
}
