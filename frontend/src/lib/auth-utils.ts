/**
 * Authentication Utilities
 *
 * Helper functions for authentication flows.
 * These functions coordinate between the auth service and Zustand store
 * to provide convenient APIs for authentication operations.
 */

import type {
  LoginRequestPayload,
  LoginResponse,
  VerifyTotpResponse,
  PasskeyVerifyResponse,
} from '@/lib/api-generated'
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
 * Check if a login / verify response requires (re-)consent.
 *
 * The backend may set either the camelCase `consentRequired` or the legacy
 * snake_case `consent_required` flag; this checks both. Shared so that every
 * login path (password, TOTP, Passkey first/second factor) applies the same
 * consent interlock consistently.
 */
export function isConsentRequired(response: {
  consentRequired?: boolean | null
  consent_required?: boolean | null
}): boolean {
  return (
    !!response.consentRequired ||
    !!(response as { consent_required?: boolean | null }).consent_required
  )
}

/**
 * Tracks the realm for which auth data has already been initialized this
 * session. Module-scoped: survives in-app navigation (so the root loader does
 * not re-fetch on every page switch) but resets on a full page reload (F5),
 * which re-runs the module and re-validates the session cookie once.
 */
let initializedRealm: string | null = null

/**
 * Initialize authentication state
 *
 * Fetches auth data from the API and populates the Zustand store on first call
 * per realm. Subsequent calls for the same realm reuse the store state without
 * hitting the API — auth data is session-scoped, not per-navigation.
 *
 * @param realmId - The realm ID to initialize auth for
 * @param forceRefresh - Force a fresh fetch even if already initialized (default: false)
 * @returns Object containing auth status and redirect path
 */
export async function initializeAuth(
  realmId: string,
  forceRefresh: boolean = false
): Promise<{
  authenticated: boolean
  redirectPath: string
}> {
  const store = useAuthStore.getState()

  // Reuse already-initialized store state for this realm (skip on full reload)
  if (initializedRealm === realmId && !forceRefresh) {
    const redirectPath = hasAdminPermission(store.permissions)
      ? DEFAULT_ADMIN_REDIRECT
      : DEFAULT_USER_REDIRECT
    return {
      authenticated: store.isAuthenticated,
      redirectPath,
    }
  }

  store.setIsLoading(true)

  try {
    const { authStatus, userPermissions, userProfile } = await fetchAuthData(realmId)

    // Update store with fetched data
    store.setAuthStatus(authStatus.authenticated, authStatus.realmId || realmId)
    store.setUserPermissions(userPermissions.permissions, userPermissions.roles)
    store.setUserProfile(userProfile)

    initializedRealm = realmId

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
    initializedRealm = null
    return {
      authenticated: false,
      redirectPath: DEFAULT_USER_REDIRECT,
    }
  } finally {
    store.setIsLoading(false)
  }
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

    if (loginResponse.requiresTotp) {
      return { response: loginResponse, redirectPath: DEFAULT_USER_REDIRECT }
    }

    if (
      loginResponse.consentRequired ||
      (loginResponse as { consent_required?: boolean | null }).consent_required
    ) {
      return { response: loginResponse, redirectPath: DEFAULT_USER_REDIRECT }
    }

    // Session is created on the token endpoint, not in the browser
    if (loginResponse.redirectTo) {
      return { response: loginResponse, redirectPath: DEFAULT_USER_REDIRECT }
    }

    const userRealmId = loginResponse.realmId || realmId
    store.login(userRealmId)

    const { authStatus, userPermissions, userProfile } = await fetchAuthData(userRealmId)
    store.setAuthStatus(authStatus.authenticated, authStatus.realmId || userRealmId)
    store.setUserPermissions(userPermissions.permissions, userPermissions.roles)
    store.setUserProfile(userProfile)

    initializedRealm = userRealmId

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
    initializedRealm = null
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
 *
 * @param realmId - The realm ID
 * @param verifyResponse - The TOTP verification response from the API
 */
export async function completeLoginAfterTotp(
  realmId: string,
  verifyResponse: VerifyTotpResponse
): Promise<{ redirectPath?: string; redirectTo?: string | null }> {
  // OAuth flow: when redirectTo is present, return it without updating store
  if (verifyResponse.redirectTo) {
    return { redirectTo: verifyResponse.redirectTo }
  }

  if (
    verifyResponse.consentRequired ||
    (verifyResponse as { consent_required?: boolean | null }).consent_required
  ) {
    return {}
  }

  const store = useAuthStore.getState()

  try {
    const { authStatus, userPermissions, userProfile } = await fetchAuthData(realmId)
    store.setAuthStatus(authStatus.authenticated, authStatus.realmId || realmId)
    store.setUserPermissions(userPermissions.permissions, userPermissions.roles)
    store.setUserProfile(userProfile)

    initializedRealm = realmId

    return { redirectPath: getRedirectPath() }
  } catch (error) {
    store.logout()
    throw error
  }
}

/**
 * Complete login after Passkey verification (first or second factor).
 *
 * Behaviour is identical to `completeLoginAfterTotp`: `PasskeyVerifyResponse`
 * is structurally the same as `VerifyTotpResponse` (camelCase
 * `consentRequired`/`agreements`/`redirectTo`/`token`). Kept as a separate,
 * strongly-typed entry point so call sites don't need an unsafe cast and the
 * consent interlock is applied uniformly across all login paths.
 *
 * @param realmId - The realm ID
 * @param verifyResponse - The Passkey verification response from the API
 */
export async function completeLoginAfterPasskey(
  realmId: string,
  verifyResponse: PasskeyVerifyResponse
): Promise<{ redirectPath?: string; redirectTo?: string | null }> {
  // OAuth flow: when redirectTo is present, return it without updating store
  if (verifyResponse.redirectTo) {
    return { redirectTo: verifyResponse.redirectTo }
  }

  if (isConsentRequired(verifyResponse)) {
    return {}
  }

  const store = useAuthStore.getState()

  try {
    const { authStatus, userPermissions, userProfile } = await fetchAuthData(realmId)
    store.setAuthStatus(authStatus.authenticated, authStatus.realmId || realmId)
    store.setUserPermissions(userPermissions.permissions, userPermissions.roles)
    store.setUserProfile(userProfile)

    initializedRealm = realmId

    return { redirectPath: getRedirectPath() }
  } catch (error) {
    store.logout()
    throw error
  }
}

/**
 * Validate OAuth search params for completeness
 *
 * All 3 params (oauthClientId, redirectUri, state) must be present together.
 * Partial params indicate a misconfigured OAuth flow.
 *
 * @param search - Search params from URL
 * @returns oauthParams if complete, hasPartialOAuth flag for error display
 */
export function validateOAuthParams(search: {
  oauthClientId?: string
  redirectUri?: string
  state?: string
}): {
  oauthParams: { oauthClientId: string; redirectUri: string; state: string } | null
  hasPartialOAuth: boolean
} {
  const oauthParams =
    search.oauthClientId && search.redirectUri && search.state
      ? {
          oauthClientId: search.oauthClientId,
          redirectUri: search.redirectUri,
          state: search.state,
        }
      : null
  const hasPartialOAuth =
    !oauthParams && (!!search.oauthClientId || !!search.redirectUri || !!search.state)
  return { oauthParams, hasPartialOAuth }
}
