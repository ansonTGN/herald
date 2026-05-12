import { Link, useLocation } from '@tanstack/react-router'
import { User, Shield, Coins, CreditCard, LogOut, FileText, type LucideIcon } from 'lucide-react'
import { useCallback, useMemo } from 'react'
import { useRealmId } from '@/stores/auth-store'
import { logoutFlow } from '@/lib/auth-utils'

interface MenuItem {
  name: string
  path: string
  icon: LucideIcon
}

export function ProfileSidebar() {
  const location = useLocation()
  const realmId = useRealmId()

  // Memoize menu items to prevent infinite re-renders
  const menuItems: MenuItem[] = useMemo(
    () => [
      { name: 'Profile', path: `/${realmId}/user/profile`, icon: User },
      { name: 'Security', path: `/${realmId}/user/security`, icon: Shield },
      { name: 'Subscription', path: `/${realmId}/user/subscription-history`, icon: CreditCard },
      { name: 'Invoices', path: `/${realmId}/user/invoices`, icon: FileText },
      { name: 'Points', path: `/${realmId}/user/points`, icon: Coins },
    ],
    [realmId]
  )

  const isActive = (path: string) => location.pathname === path

  const handleLogout = useCallback(async () => {
    await logoutFlow(realmId)
  }, [realmId])

  return (
    <aside
      data-testid="profile-sidebar"
      className="w-64 bg-white border-r border-gray-200 flex flex-col"
    >
      <div className="p-6">
        <h1 className="text-xl font-bold text-gray-900">Profile</h1>
      </div>

      <nav className="flex-1 px-3 space-y-1">
        {menuItems.map((item) => (
          <Link
            key={item.name}
            to={item.path}
            data-testid={`profile-menu-${item.name.toLowerCase()}`}
            className={`flex items-center px-3 py-2 text-sm font-medium rounded-md ${
              isActive(item.path) ? 'bg-blue-50 text-blue-600' : 'text-gray-700 hover:bg-gray-50'
            }`}
          >
            <item.icon className="w-5 h-5 mr-3" />
            {item.name}
          </Link>
        ))}
      </nav>

      <div className="p-3 border-t border-gray-200">
        <button
          data-testid="profile-logout-button"
          onClick={handleLogout}
          className="flex items-center w-full px-3 py-2 text-sm font-medium text-gray-700 rounded-md hover:bg-gray-50"
        >
          <LogOut className="w-5 h-5 mr-3" />
          Logout
        </button>
      </div>
    </aside>
  )
}
