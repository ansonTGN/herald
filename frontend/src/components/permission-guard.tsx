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
  realmId: realmIdProp,
  permission,
  fallback = null,
  children,
}: PermissionGuardProps) {
  // realmId is no longer used by usePermission, kept for backward compatibility
  const storeRealmId = useRealmId()
  void (realmIdProp ?? storeRealmId)
  const { hasPermission } = usePermission()

  if (!permission || hasPermission(permission)) {
    return <>{children}</>
  }

  return <>{fallback}</>
}
