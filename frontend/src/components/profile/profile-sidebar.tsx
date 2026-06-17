import { Link, useLocation } from '@tanstack/react-router'
import { User, Shield, Coins, CreditCard, FileText, LogOut, type LucideIcon } from 'lucide-react'
import { useCallback, useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useRealmId } from '@/stores/auth-store'
import { logoutFlow } from '@/lib/auth-utils'
import { featureAvailabilityQueryOptions } from '@/data/query-options'
import { m } from '@/paraglide/messages'
import { LanguageSwitcher } from '@/components/shared/language-switcher'

interface MenuItem {
  name: string
  path: string
  icon: LucideIcon
  visible?: boolean
}

export function ProfileSidebar() {
  const location = useLocation()
  const realmId = useRealmId()
  const { data: features } = useQuery(featureAvailabilityQueryOptions(realmId))
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
      { name: 'Profile', path: `/${realmId}/user/profile`, icon: User },
      { name: 'Security', path: `/${realmId}/user/security`, icon: Shield },
      {
        name: 'Points',
        path: `/${realmId}/user/points`,
        icon: Coins,
        visible: userFeatures?.pointsVisible === true,
      },
      {
        name: 'PurchaseRecords',
        path: `/${realmId}/user/subscription-history`,
        icon: CreditCard,
        visible: userFeatures?.pointsPurchaseVisible === true,
      },
      {
        name: 'Invoices',
        path: `/${realmId}/user/invoices`,
        icon: FileText,
        visible: userFeatures?.invoicesVisible === true,
      },
    ],
    [realmId, userFeatures]
  )

  const isActive = (path: string) => location.pathname === path

  const handleLogout = useCallback(async () => {
    await logoutFlow(realmId)
  }, [realmId])

  return (
    <aside
      data-testid="profile-sidebar"
      className="w-64 bg-sidebar border-r border-sidebar-border flex flex-col"
    >
      <div className="p-6">
        <h1 className="text-xl font-bold text-sidebar-foreground">{m['nav_profile.profile']()}</h1>
      </div>

      <nav className="flex-1 px-3 space-y-1">
        {menuItems
          .filter((item) => item.visible !== false)
          .map((item) => (
            <Link
              key={item.name}
              to={item.path}
              data-testid={`profile-menu-${item.name.toLowerCase()}`}
              className={`flex items-center px-3 py-2 text-sm font-medium rounded-md transition-colors ${
                isActive(item.path)
                  ? 'bg-sidebar-accent text-sidebar-foreground'
                  : 'text-sidebar-foreground/70 hover:bg-sidebar-accent hover:text-sidebar-foreground'
              }`}
            >
              <item.icon className="w-5 h-5 mr-3" />
              {getProfileNavLabel(item.name)}
            </Link>
          ))}
      </nav>

      <div className="px-3 pb-3">
        <LanguageSwitcher />
      </div>

      <div className="p-3 border-t border-sidebar-border">
        <button
          data-testid="profile-logout-button"
          onClick={handleLogout}
          className="flex items-center w-full px-3 py-2 text-sm font-medium text-sidebar-foreground/70 rounded-md hover:bg-sidebar-accent hover:text-sidebar-foreground transition-colors"
        >
          <LogOut className="w-5 h-5 mr-3" />
          {m['user_menu.logout']()}
        </button>
      </div>
    </aside>
  )
}
