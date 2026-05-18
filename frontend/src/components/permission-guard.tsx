import { type ReactNode } from 'react'
import { usePermission } from '@/hooks/use-permission'
import { useRealmId } from '@/stores/auth-store'

interface PermissionGuardProps {
  realmId?: string // Optional for backward compatibility
  permission?: string
  fallback?: ReactNode
  children: ReactNode
}

export function PermissionGuard({
  realmId: _realmId,
  permission,
  fallback = null,
  children,
}: PermissionGuardProps) {
  // realmId prop and useRealmId are kept for backward compatibility but unused
  void _realmId
  void useRealmId()
  const { hasPermission } = usePermission()

  if (!permission || hasPermission(permission)) {
    return <>{children}</>
  }

  return <>{fallback}</>
}
