/**
 * Authentication Service
 *
 * Direct API calls for authentication and authorization.
 * This service bypasses React Query to provide direct, synchronous
 * access to auth data for Zustand store updates.
 */

import { status, getUserRoles, getProfile, login, logout } from '@/lib/api-generated'
import type {
  StatusResponse,
  LoginRequestPayload,
  LoginResponse,
  UserProfile,
} from '@/lib/api-generated'

/**
 * Extended status response with permissions
 */
export interface ExtendedStatusResponse extends StatusResponse {
  permissions?: string[]
}

/**
 * Fetch authentication status from the API
 *
 * @param realmId - The realm ID to check auth status for
 * @returns The authentication status
 */
export async function fetchAuthStatus(realmId: string): Promise<StatusResponse> {
  console.log('[fetchAuthStatus] Checking auth status for realm:', realmId)
  const { data, error } = await status({ path: { realmId } })
  console.log('[fetchAuthStatus] Response:', { data, error })
  if (error) throw error
  return data
}

/**
 * Fetch user roles and permissions from the API
 *
 * @returns Object containing permissions and roles arrays
 */
export async function fetchUserPermissions(): Promise<{ permissions: string[]; roles: string[] }> {
  console.log('[fetchUserPermissions] Fetching user roles and permissions')
  const { data, error } = await getUserRoles()
  console.log('[fetchUserPermissions] Response:', { data, error })
  if (error || !data) {
    return { permissions: [], roles: [] }
  }
  return {
    permissions: data.permissions || [],
    roles: data.roles || [],
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
 * Perform logout
 *
 * @param realmId - The realm ID to logout from
 */
export async function performLogout(realmId: string): Promise<void> {
  const { error } = await logout({ path: { realmId } })
  if (error) throw error
}

/**
 * Fetch auth data sequentially based on authentication status
 * First checks auth status, then conditionally fetches user data
 *
 * @param realmId - The realm ID to fetch auth data for
 * @returns Object containing auth status, user permissions, and profile
 */
export async function fetchAuthData(realmId: string): Promise<{
  authStatus: StatusResponse
  userPermissions: { permissions: string[]; roles: string[] }
  userProfile: UserProfile | null
}> {
  console.log('[fetchAuthData] Starting to fetch auth data for realm:', realmId)

  // First, check authentication status
  const authStatus = await fetchAuthStatus(realmId)

  console.log('[fetchAuthData] Auth status:', {
    authenticated: authStatus.authenticated,
    realmId: authStatus.realmId,
  })

  // Only fetch user data if authenticated
  const userPermissions = authStatus.authenticated
    ? await fetchUserPermissions()
    : { permissions: [], roles: [] }

  const userProfile = authStatus.authenticated ? await fetchUserProfile().catch(() => null) : null

  console.log('[fetchAuthData] Final result:', {
    authenticated: authStatus.authenticated,
    permissions: userPermissions.permissions,
    hasProfile: !!userProfile,
  })

  return {
    authStatus,
    userPermissions,
    userProfile,
  }
}
