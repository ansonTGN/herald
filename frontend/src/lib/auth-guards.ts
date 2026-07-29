/**
 * Authentication Guards
 *
 * Route protection functions for authentication and authorization.
 * These functions now use Zustand store for synchronous state access.
 */

import { redirect } from '@tanstack/react-router'
import { checkAdminPermission, getRedirectPath } from '@/lib/auth-utils'
import {
  ADMIN_WEB_CONSOLE_CLIENT_ID,
  firstPartyClientForPath,
} from '@/lib/constants/auth-constants'

// Type for redirect options - using TanStack Router's internal types
type RedirectOptions = {
  to: string
  params?: Record<string, string | undefined>
  search?: Record<string, string | string[] | undefined>
}

// Helper function to create redirect with proper typing
function createRedirect(options: RedirectOptions): never {
  throw redirect(options)
}

/**
 * Require authentication for a route
 * Uses Zustand store for synchronous state access
 *
 * @param realmId - The realm ID
 * @param currentPath - The current path
 */
export async function requireAuthentication(realmId: string, currentPath: string): Promise<void> {
  // Import here to avoid circular dependency
  const { initializeAuth } = await import('@/lib/auth-utils')
  const store = await import('@/stores/auth-store').then((m) => m.useAuthStore)

  const { authenticated } = await initializeAuth(realmId, firstPartyClientForPath(currentPath))

  if (!authenticated) {
    // Extract relative path (without realm prefix)
    const redirectPath = currentPath.startsWith('http')
      ? new URL(currentPath).pathname.replace(new RegExp(`^/${realmId}`), '') || '/'
      : currentPath.replace(new RegExp(`^/${realmId}`), '') || '/'

    createRedirect({
      to: `/${realmId}/auth/login`,
      search: { redirect: redirectPath },
    })
  }

  const userRealmId = store.getState().realmId
  if (userRealmId && userRealmId !== realmId) {
    // Redirect to root with user's realm id
    createRedirect({
      to: `/${userRealmId}`,
    })
  }
}

/**
 * Require admin permissions for a route
 * Uses Zustand store for synchronous state access
 *
 * @param realmId - The realm ID
 */
export async function requireAdminPermission(realmId: string): Promise<void> {
  // Import here to avoid circular dependency
  const { initializeAuth } = await import('@/lib/auth-utils')

  const { authenticated, clientId } = await initializeAuth(realmId, ADMIN_WEB_CONSOLE_CLIENT_ID)

  if (!authenticated) {
    createRedirect({
      to: `/${realmId}/auth/login`,
      search: { redirect: `/${realmId}/manage` },
    })
  }

  if (clientId !== ADMIN_WEB_CONSOLE_CLIENT_ID || !checkAdminPermission()) {
    createRedirect({
      to: `/${realmId}/user/profile`,
    })
  }
}

/**
 * Get appropriate redirect path based on user permissions
 * Uses Zustand store for synchronous state access
 *
 * @returns The redirect path
 * @deprecated Use `getRedirectPath` from `@/lib/auth-utils` instead
 */
export async function getRedirectPathByPermissions(): Promise<string> {
  return getRedirectPath()
}

/**
 * Redirect if already authenticated
 * Uses Zustand store for synchronous state access
 *
 * @param realmId - The realm ID
 * @param redirectPath - Optional redirect path
 */
export async function redirectIfAuthenticated(
  realmId: string,
  redirectPath?: string
): Promise<void> {
  // Import here to avoid circular dependency
  const { initializeAuth } = await import('@/lib/auth-utils')

  const { authenticated } = await initializeAuth(realmId, firstPartyClientForPath(redirectPath))

  if (authenticated) {
    const targetPath = redirectPath || getRedirectPath()
    createRedirect({
      to: `/${realmId}${targetPath}`,
    })
  }
}

/**
 * Allowed redirect paths - whitelist to prevent open redirect attacks
 * These patterns expect relative paths (without realm prefix)
 */
const ALLOWED_REDIRECT_PATTERNS = [
  /^\/$/, // Root path
  /^\/user\/.*$/, // User related paths (relative, without realm prefix)
  /^\/manage\/.*$/, // Management related paths (relative, without realm prefix)
]

/**
 * Validate redirect path to prevent open redirect vulnerabilities
 *
 * @param redirectPath - The redirect path to validate
 * @returns true if the path is safe to redirect to
 * @deprecated Use `validateRedirectPath` from `@/lib/constants/auth-constants` instead
 */
export function validateRedirectPath(redirectPath: string | undefined): boolean {
  if (!redirectPath) return true

  // Reject absolute URLs to prevent external redirects
  if (redirectPath.startsWith('http://') || redirectPath.startsWith('https://')) {
    return false
  }

  // Reject protocol-relative URLs
  if (redirectPath.startsWith('//')) {
    return false
  }

  // Reject paths starting with \ to prevent Windows-style absolute paths
  if (redirectPath.startsWith('\\')) {
    return false
  }

  // Check against whitelist
  return ALLOWED_REDIRECT_PATTERNS.some((pattern) => pattern.test(redirectPath))
}

/**
 * Get safe redirect path or fallback
 *
 * @param redirectPath - The requested redirect path
 * @param fallbackPath - The fallback path if redirectPath is invalid
 * @returns The safe redirect path
 * @deprecated Use `getSafeRedirect` from `@/lib/auth-utils` instead
 */
export function getSafeRedirectPath(
  redirectPath: string | undefined,
  fallbackPath: string
): string {
  return validateRedirectPath(redirectPath) ? (redirectPath ?? fallbackPath) : fallbackPath
}
