/**
 * Authentication Service
 *
 * Direct API calls for authentication and authorization.
 * This service bypasses React Query to provide direct, synchronous
 * access to auth data for Zustand store updates.
 *
 * All calls go through the generated `@hey-api` client, which (after
 * `initBearerClient()` in `main.tsx`) injects `Authorization: Bearer` from the
 * in-memory access-token holder and silently refreshes on a single 401.
 */

import {
  status,
  getCurrentUserPermissions,
  getUserRoles,
  getProfile,
  login,
  logout,
  refresh,
  oauthToken,
} from '@/lib/api-generated'
import type {
  StatusResponse,
  LoginRequestPayload,
  LoginResponse,
  UserProfile,
  BrowserTokenResponse,
} from '@/lib/api-generated'
import { FIRST_PARTY_CLIENT_ID } from '@/lib/constants/auth-constants'

/**
 * Extended status response with permissions
 */
export interface ExtendedStatusResponse extends StatusResponse {
  permissions?: string[]
}

/**
 * Fetch authentication status from the API
 *
 * @returns The authentication status
 */
export async function fetchAuthStatus(): Promise<StatusResponse> {
  const { data, error } = await status()
  if (error) throw error
  if (!data) {
    throw new Error('Auth status failed: no response data')
  }
  return data
}

/**
 * Fetch user roles and permissions from the API
 *
 * @returns Object containing permissions and roles arrays
 */
export async function fetchUserPermissions(): Promise<{ permissions: string[]; roles: string[] }> {
  const [permissionsResult, rolesResult] = await Promise.all([
    getCurrentUserPermissions(),
    getUserRoles(),
  ])

  if (permissionsResult.error || rolesResult.error) {
    return { permissions: [], roles: [] }
  }

  return {
    permissions: permissionsResult.data?.permissions || [],
    roles: rolesResult.data?.roles || [],
  }
}

/**
 * Fetch user profile from the API
 *
 * @returns The user profile or null if not authenticated
 */
export async function fetchUserProfile(): Promise<UserProfile | null> {
  const { data, error } = await getProfile()
  if (error || !data) {
    return null
  }
  return data
}

/**
 * Perform login with credentials
 *
 * @param realmId - The realm ID to login to
 * @param credentials - Login credentials
 * @returns Login response data
 */
export async function performLogin(
  realmId: string,
  credentials: LoginRequestPayload
): Promise<LoginResponse> {
  const { data, error } = await login({
    path: { realmId },
    body: credentials,
  })
  if (error) {
    throw error
  }
  if (!data) {
    throw new Error('Login failed: no response data')
  }
  return data
}

/**
 * Perform logout — revokes the Bearer access/refresh token family.
 *
 * Runs against the Bearer-injecting generated client (the `logout` SDK function
 * carries the `bearer` security scheme). Safe to call even if the access token
 * has already expired: the backend revokes by family.
 */
export async function performLogout(): Promise<void> {
  const { error } = await logout()
  if (error) throw error
}

/**
 * Refresh the browser token set.
 *
 * Wraps the generated `refresh` SDK function (`POST /api/auth/browser-token/
 * refresh`). The refresh token rotates: the returned set contains a NEW
 * access token AND a NEW refresh token which must replace the stored one.
 *
 * @param refreshToken - The current (about-to-be-rotated) refresh token.
 * @param clientId     - The Client App `client_id` the token was issued for.
 * @returns The new access + refresh token set.
 */
export async function refreshBrowserToken(
  refreshToken: string,
  clientId: string
): Promise<BrowserTokenResponse> {
  const { data, error } = await refresh({
    body: { refreshToken, clientId },
  })
  if (error) {
    throw error
  }
  if (!data) {
    throw new Error('Token refresh failed: no response data')
  }
  return data
}

/**
 * Input for the FirstParty PKCE token exchange.
 */
export interface PkceTokenExchangeInput {
  /** The authorization `code` returned in the login `redirectTo` URL. */
  code: string
  /** The PKCE `code_verifier` paired with the challenge sent to authorize. */
  codeVerifier: string
  /** The pre-registered `redirect_uri` the code was issued for. */
  redirectUri: string
}

/**
 * Exchange an OAuth authorization code for a FirstParty Bearer token set.
 *
 * Wraps the generated `oauthToken` SDK function (`POST /api/oauth/{realmId}/
 * token`). The token endpoint verifies the PKCE `code_verifier` against the
 * stored S256 challenge, then issues a `FirstParty` token set (full RBAC) for
 * the built-in `admin-web-console` Client App. The response uses OAuth
 * snake_case field names (`access_token`, `refresh_token`, ...) per RFC 6749.
 *
 * @param realmId - The realm ID the code was issued in.
 * @param input   - The code + PKCE verifier + redirect URI.
 * @returns The new access + refresh token set (normalized to camelCase).
 */
export async function performPkceTokenExchange(
  realmId: string,
  input: PkceTokenExchangeInput
): Promise<BrowserTokenResponse> {
  const { data, error } = await oauthToken({
    path: { realmId },
    body: {
      grant_type: 'authorization_code',
      code: input.code,
      code_verifier: input.codeVerifier,
      redirect_uri: input.redirectUri,
      // `client_id` is the OAuth Client App the code was bound to — the built-in
      // Herald console Client App.
      client_id: FIRST_PARTY_CLIENT_ID,
    },
  })
  if (error) {
    throw error
  }
  if (!data) {
    throw new Error('PKCE token exchange failed: no response data')
  }
  // Normalize OAuth snake_case → the shared BrowserTokenResponse shape so the
  // store and API client deal with one token-set contract everywhere.
  return {
    accessToken: data.access_token,
    refreshToken: data.refresh_token,
    tokenType: data.token_type,
    expiresIn: data.expires_in,
    refreshExpiresIn: data.refresh_expires_in,
  }
}

/**
 * Fetch auth data based on authentication status.
 * First checks auth status, then conditionally fetches user data in parallel.
 *
 * @returns Object containing auth status, user permissions, and profile
 */
export async function fetchAuthData(): Promise<{
  authStatus: StatusResponse
  userPermissions: { permissions: string[]; roles: string[] }
  userProfile: UserProfile | null
}> {
  // First, check authentication status
  const authStatus = await fetchAuthStatus()

  // Fetch user data in parallel since they have no dependency on each other
  const [userPermissions, userProfile] = authStatus.authenticated
    ? await Promise.all([fetchUserPermissions(), fetchUserProfile().catch(() => null)])
    : [{ permissions: [], roles: [] }, null]

  return {
    authStatus,
    userPermissions,
    userProfile,
  }
}
