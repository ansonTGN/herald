import { useAuth } from '@/hooks/use-auth'
import { hasAdminPermission } from '@/lib/constants/auth-constants'

/**
 * Unified permission checking hook
 *
 * Provides comprehensive permission and role checking functionality.
 * Merges capabilities from use-permission and use-auth-check for consistency.
 */
export function usePermission() {
  const { permissions, roles, isLoading } = useAuth()

  // Handle null/undefined permissions safely
  const safePermissions = permissions || []
  const safeRoles = roles || []

  return {
    // Basic permission checks
    hasPermission: (permission: string) => safePermissions.includes(permission),
    hasAnyPermission: (permissionList: string[]) =>
      permissionList.some((p) => safePermissions.includes(p)),
    hasAllPermissions: (permissionList: string[]) =>
      permissionList.every((p) => safePermissions.includes(p)),

    // Role checks
    hasRole: (role: string) => safeRoles.includes(role),
    hasAnyRole: (roleList: string[]) => roleList.some((r) => safeRoles.includes(r)),

    // Predefined permission checks
    hasAdminPermission: hasAdminPermission(safePermissions),
    isSuperAdmin: safePermissions.includes('*') || false,

    // Raw data
    permissions: safePermissions,
    roles: safeRoles,
    isLoading,
  }
}
