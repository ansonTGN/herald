/**
 * Permission Check Utilities
 *
 * Provides permission-based access control functions.
 * All checks are based on permissions, not roles.
 *
 * @deprecated Use `hasAdminPermission` from `@/lib/constants/auth-constants` instead.
 * This file is kept for backward compatibility but will be removed in a future release.
 */

/**
 * Checks if a user has any admin permission.
 * Admin permissions are those that allow managing system resources.
 *
 * @param permissions - List of permission strings (e.g., ['users.view', 'roles.manage'])
 * @returns true if user has any admin permission
 * @deprecated Use `hasAdminPermission` from `@/lib/constants/auth-constants` instead
 */
export function hasAdminPermission(permissions: string[]): boolean {
  if (!permissions || permissions.length === 0) {
    return false
  }

  // Super admin has all permissions
  if (permissions.includes('*')) {
    return true
  }

  // Check for admin resource permissions
  const adminPrefixes = ['users.', 'roles.', 'permissions.', 'clients.', 'realms.', 'billing.']
  return permissions.some((p) => adminPrefixes.some((prefix) => p.startsWith(prefix)))
}
