import { Link, useLocation } from '@tanstack/react-router'
import { useCallback, useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { usePermissions, useRealmId } from '@/stores/auth-store'
import { logoutFlow } from '@/lib/auth-utils'
import { hasAdminPermission } from '@/lib/constants/auth-constants'
import { userFeatureAvailabilityQueryOptions } from '@/data/query-options'
import { m } from '@/paraglide/messages'
import { LanguageSwitcher } from '@/components/shared/language-switcher'
import { realmPath, resolvedRealmFromPath } from '@/lib/realm-routing'

interface MenuItem {
  name: string
  path: string
  visible?: boolean
}

export function ProfileSidebar() {
  const location = useLocation()
  const storeRealmId = useRealmId()
  const realmContext = resolvedRealmFromPath(location.pathname)
  const realmId = realmContext.realmId || storeRealmId || 'admin'
  const permissions = usePermissions()
  const canAccessAdminConsole = hasAdminPermission(permissions)
  const { data: features } = useQuery(userFeatureAvailabilityQueryOptions)
  const userFeatures = features?.user

  /** Maps profile menu item name to its translated display label. */
  const getProfileNavLabel = useCallback((name: string): string => {
    const map: Record<string, () => string> = {
      Profile: m['nav_profile.profile'],
      Security: m['nav_profile.security'],
      Points: m['nav_profile.points'],
      PurchaseRecords: m['nav_profile.purchase_records'],
      Invoices: m['nav_profile.invoices'],
    }
    return map[name]?.() ?? name
  }, [])

  // Memoize menu items to prevent infinite re-renders
  const menuItems: MenuItem[] = useMemo(
    () => [
      {
        name: 'Profile',
        path: realmPath({ ...realmContext, realmId }, '/user/profile'),
      },
      {
        name: 'Security',
        path: realmPath({ ...realmContext, realmId }, '/user/security'),
      },
      {
        name: 'Points',
        path: realmPath({ ...realmContext, realmId }, '/user/points'),
        visible: userFeatures?.pointsVisible === true,
      },
      {
        name: 'PurchaseRecords',
        path: realmPath({ ...realmContext, realmId }, '/user/subscription-history'),
        visible: userFeatures?.pointsVisible === true,
      },
      {
        name: 'Invoices',
        path: realmPath({ ...realmContext, realmId }, '/user/invoices'),
        visible: userFeatures?.invoicesVisible === true,
      },
    ],
    [realmContext, realmId, userFeatures]
  )

  const isActive = (path: string) => location.pathname === path

  const handleLogout = useCallback(async () => {
    await logoutFlow(realmId)
  }, [realmId])

  return (
    <aside
      data-testid="profile-sidebar"
      className="w-64 border-r border-border flex flex-col px-6 py-8"
    >
      <h1 className="text-lg font-semibold tracking-tight text-foreground">
        {m['nav_profile.profile']()}
      </h1>

      <nav className="mt-8 flex-1 space-y-1">
        {menuItems
          .filter((item) => item.visible !== false)
          .map((item) => (
            <Link
              key={item.name}
              to={item.path}
              data-testid={`profile-menu-${item.name.toLowerCase()}`}
              className={`block py-1.5 text-sm transition-colors ${
                isActive(item.path)
                  ? 'font-medium text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              {getProfileNavLabel(item.name)}
            </Link>
          ))}
      </nav>

      <div className="space-y-2">
        {canAccessAdminConsole && (
          <a
            href={realmPath({ ...realmContext, realmId }, '/manage')}
            data-testid="profile-admin-console-link"
            className="block py-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
          >
            {m['nav.dashboard']()}
          </a>
        )}
        <LanguageSwitcher />
      </div>

      <div className="mt-4 border-t border-border pt-4">
        <button
          data-testid="profile-logout-button"
          onClick={handleLogout}
          className="block py-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
        >
          {m['user_menu.logout']()}
        </button>
      </div>
    </aside>
  )
}
