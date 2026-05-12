import { useState, useMemo, useCallback } from 'react'
import { Link } from '@tanstack/react-router'
import {
  LayoutDashboard,
  Users,
  Shield,
  Settings,
  Globe,
  Key,
  ChevronDown,
  Briefcase,
  CreditCard,
  History,
  Coins,
  FileText,
} from 'lucide-react'
import { useAuth } from '@/hooks/use-auth'
import { useRealmId } from '@/stores/auth-store'
import type { LucideIcon } from 'lucide-react'

interface MenuItem {
  name: string
  path?: string
  icon: LucideIcon
  permission: string | null
  children?: MenuItem[]
}

export function Sidebar() {
  const realmId = useRealmId()
  const { permissions } = useAuth()
  const [openMenus, setOpenMenus] = useState<Set<string>>(new Set(['Authorization']))

  // Memoize toggle function to prevent unnecessary re-renders
  const toggleMenu = useCallback((name: string) => {
    setOpenMenus((prev) => {
      const next = new Set(prev)
      if (next.has(name)) {
        next.delete(name)
      } else {
        next.add(name)
      }
      return next
    })
  }, [])

  // Memoize menu items to prevent infinite re-renders
  const menuItems: MenuItem[] = useMemo(
    () => [
      { name: 'Dashboard', path: `/${realmId}/manage`, icon: LayoutDashboard, permission: null },
      {
        name: 'Realms',
        path: `/${realmId}/manage/realms`,
        icon: Globe,
        permission: 'realm.create',
      },
      {
        name: 'Clients',
        path: `/${realmId}/manage/client-apps`,
        icon: Briefcase,
        permission: 'clients.view',
      },
      { name: 'Users', path: `/${realmId}/manage/users`, icon: Users, permission: 'users.view' },
      {
        name: 'Authorization',
        icon: Shield,
        permission: null,
        children: [
          {
            name: 'Permissions',
            path: `/${realmId}/manage/permissions`,
            icon: Key,
            permission: 'permissions.view',
          },
          {
            name: 'Roles',
            path: `/${realmId}/manage/roles`,
            icon: Shield,
            permission: 'roles.view',
          },
        ],
      },
      {
        name: 'Points',
        icon: Coins,
        permission: 'points.view',
        children: [
          {
            name: 'User Accounts',
            path: `/${realmId}/manage/points/accounts`,
            icon: Users,
            permission: 'points.view',
          },
          {
            name: 'Plan Configurations',
            path: `/${realmId}/manage/points/configs`,
            icon: Settings,
            permission: 'points.view',
          },
        ],
      },
      {
        name: 'Billing',
        icon: CreditCard,
        permission: 'billing.view',
        children: [
          {
            name: 'Billing Plans',
            path: `/${realmId}/manage/billing`,
            icon: CreditCard,
            permission: 'billing.view',
          },
          {
            name: 'Invoices',
            path: `/${realmId}/manage/billing/invoices`,
            icon: FileText,
            permission: 'billing.view',
          },
          {
            name: 'Products',
            path: `/${realmId}/manage/products`,
            icon: Briefcase,
            permission: 'billing.view',
          },
          {
            name: 'Points Packages',
            path: `/${realmId}/manage/points-packages`,
            icon: Coins,
            permission: 'billing.view',
          },
          {
            name: 'Payment Providers',
            path: `/${realmId}/manage/billing/payment-providers`,
            icon: CreditCard,
            permission: 'billing.view',
          },
          {
            name: 'Subscription History',
            path: `/${realmId}/manage/subscription-history`,
            icon: History,
            permission: 'billing.view',
          },
        ],
      },
      { name: 'Settings', path: `/${realmId}/manage/settings`, icon: Settings, permission: null },
    ],
    [realmId]
  )

  // Filter menu items: Realms menu only shows in admin realm
  const filteredMenuItems = menuItems.filter((item) => {
    // Realms menu only shows in admin realm
    if (item.name === 'Realms' && realmId !== 'admin') {
      return false
    }
    return true
  })

  const renderMenuItem = (item: MenuItem, level: number = 0) => {
    const hasChildren = item.children && item.children.length > 0
    const isOpen = openMenus.has(item.name)
    const Icon = item.icon

    // Skip if permission check fails
    if (item.permission && !permissions.includes(item.permission)) {
      return null
    }

    // For parent menus, verify at least one child is visible
    const visibleChildren = hasChildren
      ? item.children!.filter(
          (child) => !child.permission || permissions.includes(child.permission)
        )
      : []

    if (hasChildren && visibleChildren.length === 0) {
      return null
    }

    const paddingLeft = level > 0 ? 'px-12' : 'px-6'

    return (
      <div key={item.name}>
        {!hasChildren && item.path ? (
          <Link
            to={item.path}
            className={`flex items-center text-gray-700 hover:bg-gray-100 ${paddingLeft} py-3`}
            activeProps={{ className: 'bg-gray-100 font-semibold' }}
            activeOptions={{ exact: true }}
            data-testid={`sidebar-menu-${item.name.toLowerCase().replace(/\s+/g, '-')}`}
          >
            <Icon className="w-5 h-5 mr-3" />
            {item.name}
          </Link>
        ) : (
          <div
            onClick={() => hasChildren && toggleMenu(item.name)}
            className={`flex items-center text-gray-700 hover:bg-gray-100 cursor-pointer ${paddingLeft} py-3`}
            data-testid={`sidebar-menu-${item.name.toLowerCase().replace(/\s+/g, '-')}`}
          >
            <Icon className="w-5 h-5 mr-3" />
            <span className="flex-1">{item.name}</span>
            {hasChildren && (
              <ChevronDown
                className={`w-4 h-4 transition-transform ${isOpen ? 'rotate-0' : '-rotate-90'}`}
              />
            )}
          </div>
        )}

        {hasChildren && isOpen && (
          <div>{visibleChildren.map((child) => renderMenuItem(child, level + 1))}</div>
        )}
      </div>
    )
  }

  return (
    <div
      className="flex h-full min-h-0 w-64 flex-col bg-white border-r"
      data-testid="admin-sidebar"
    >
      <div className="shrink-0 p-6">
        <h1 className="text-xl font-bold">CAS Admin</h1>
        <p className="text-sm text-gray-500">{realmId}</p>
      </div>

      <nav className="mt-6 min-h-0 flex-1 overflow-y-auto" data-testid="sidebar-nav">
        {filteredMenuItems.map((item) => renderMenuItem(item))}
      </nav>
    </div>
  )
}
