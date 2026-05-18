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
  _forceRefresh: boolean = false
): Promise<{
  authenticated: boolean
  redirectPath: string
}> {
  const store = useAuthStore.getState()

  // Always fetch fresh auth data from API
  store.setIsLoading(true)

  try {
    const { authStatus, userPermissions, userProfile } = await fetchAuthData(realmId)

    // Update store with fetched data
    store.setAuthStatus(authStatus.authenticated, authStatus.realmId || realmId)
    store.setUserPermissions(userPermissions.permissions, userPermissions.roles)
    store.setUserProfile(userProfile)

    // Determine redirect path based on permissions
    const redirectPath = hasAdminPermission(userPermissions.permissions)
      ? DEFAULT_ADMIN_REDIRECT
      : DEFAULT_USER_REDIRECT

    return {
      authenticated: authStatus.authenticated,
      redirectPath,
    }
  } catch {
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

  try {
    // Perform login API call
    const loginResponse = await performLogin(realmId, credentials)

    // Check if TOTP is required
    if (loginResponse.requiresTotp) {
      // Return early - caller should handle TOTP verification
      return { response: loginResponse, redirectPath: DEFAULT_USER_REDIRECT }
    }

    // Get the user's actual realm from the response
    const userRealmId = loginResponse.realmId || realmId

    // Update store with login state
    store.login(userRealmId)

    // Fetch and populate auth data
    const { authStatus, userPermissions, userProfile } = await fetchAuthData(userRealmId)

    // Update store with fetched data
    store.setAuthStatus(authStatus.authenticated, authStatus.realmId || userRealmId)
    store.setUserPermissions(userPermissions.permissions, userPermissions.roles)
    store.setUserProfile(userProfile)

    // Determine redirect path based on permissions (using the freshly fetched data)
    const redirectPath = hasAdminPermission(userPermissions.permissions)
      ? DEFAULT_ADMIN_REDIRECT
      : DEFAULT_USER_REDIRECT

    return { response: loginResponse, redirectPath }
  } catch (error) {
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
  const hasAdmin = checkAdminPermission()
  const redirectPath = hasAdmin ? DEFAULT_ADMIN_REDIRECT : DEFAULT_USER_REDIRECT
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
