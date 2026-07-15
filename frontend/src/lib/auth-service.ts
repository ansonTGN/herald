/**
 * Authentication Service
 *
 * Direct API calls for authentication and authorization.
 * This service bypasses React Query to provide direct, synchronous
 * access to auth data for Zustand store updates.
 */

import {
  status,
  getCurrentUserPermissions,
  getUserRoles,
  getProfile,
  login,
  logout,
} from '@/lib/api-generated'
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
 * @returns The authentication status
 */
export async function fetchAuthStatus(): Promise<StatusResponse> {
  const { data, error } = await status()
  if (error) throw error
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
 * Perform logout
 *
 */
export async function performLogout(): Promise<void> {
  const { error } = await logout()
  if (error) throw error
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
