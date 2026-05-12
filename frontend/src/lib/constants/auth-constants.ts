/**
 * Authentication Constants
 *
 * Centralized constants and utility functions for authentication and authorization.
 * This file contains the admin permissions list and related helper functions
 * that were previously duplicated across multiple files.
 */

// Admin permissions list - centralized definition
export const ADMIN_PERMISSIONS = [
  'users.view',
  'users.create',
  'users.update',
  'users.delete',
  'roles.view',
  'roles.create',
  'roles.update',
  'roles.delete',
  'permissions.view',
  'permissions.create',
  'permissions.update',
  'permissions.delete',
  'clients.view',
  'clients.create',
  'clients.update',
  'clients.delete',
  'realms.create',
  'realms.update',
  'billing.view',
  'billing.manage',
  'points.manage',
] as const

/**
 * Check if user has any admin permission
 *
 * @param permissions - List of permission strings
 * @returns true if user has any admin permission
 */
export function hasAdminPermission(permissions?: string[]): boolean {
  return ADMIN_PERMISSIONS.some((perm) => permissions?.includes(perm))
}

/**
 * Allowed redirect paths - whitelist to prevent open redirect attacks
 * These patterns expect relative paths (without realm prefix)
 */
const ALLOWED_REDIRECT_PATTERNS = [
  /^\/$/, // Root path
  /^\/user(\/.*)?$/, // User related paths (relative, without realm prefix)
  /^\/manage(\/.*)?$/, // Management related paths (relative, without realm prefix)
]

/**
 * Validate redirect path to prevent open redirect vulnerabilities
 *
 * @param redirectPath - The redirect path to validate
 * @returns true if the path is safe to redirect to
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
 */
export function getSafeRedirectPath(
  redirectPath: string | undefined,
  fallbackPath: string
): string {
  return validateRedirectPath(redirectPath) ? (redirectPath ?? fallbackPath) : fallbackPath
}

/**
 * Storage key for auth state persistence in localStorage
 */
export const AUTH_STORAGE_KEY = 'auth-storage'

/**
 * Name for Zustand store (used in DevTools)
 */
export const AUTH_STORE_NAME = 'AuthStore'

/**
 * Default redirect path for regular users
 */
export const DEFAULT_USER_REDIRECT = '/user/profile'

/**
 * Default redirect path for admin users
 */
export const DEFAULT_ADMIN_REDIRECT = '/manage'
