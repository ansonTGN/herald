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
  ScrollText,
} from 'lucide-react'
import { useQuery } from '@tanstack/react-query'
import { useAuth } from '@/hooks/use-auth'
import { useRealmId } from '@/stores/auth-store'
import { PERMISSION } from '@/lib/constants/auth-constants'
import { realmQueryOptions, featureAvailabilityQueryOptions } from '@/data/query-options'
import { filterByPermission } from '@/lib/utils/filter-by-permission'
import type { LucideIcon } from 'lucide-react'

interface MenuItem {
  id: string
  name: string
  path?: string
  icon: LucideIcon
  permission: string | null
  visible?: boolean
  children?: MenuItem[]
}

export function Sidebar() {
  const realmId = useRealmId()
  const { permissions } = useAuth()
  const { data: realm } = useQuery(realmQueryOptions(realmId))
  const { data: features } = useQuery(featureAvailabilityQueryOptions(realmId))
  const [openMenus, setOpenMenus] = useState<Set<string>>(new Set(['Authorization']))
  const adminFeatures = features?.admin

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
      {
        id: 'dashboard',
        name: 'Dashboard',
        path: `/${realmId}/manage`,
        icon: LayoutDashboard,
        permission: PERMISSION.DASHBOARD_VIEW,
      },
      {
        id: 'realms',
        name: 'Realms',
        path: `/${realmId}/manage/realms`,
        icon: Globe,
        permission: PERMISSION.REALM_VIEW,
      },
      {
        id: 'clients',
        name: 'Clients',
        path: `/${realmId}/manage/client-apps`,
        icon: Briefcase,
        permission: PERMISSION.CLIENTS_VIEW,
      },
      {
        id: 'users',
        name: 'Users',
        path: `/${realmId}/manage/users`,
        icon: Users,
        permission: PERMISSION.USERS_VIEW,
      },
      {
        id: 'authorization',
        name: 'Authorization',
        icon: Shield,
        permission: null,
        children: [
          {
            id: 'permissions',
            name: 'Permissions',
            path: `/${realmId}/manage/permissions`,
            icon: Key,
            permission: PERMISSION.PERMISSIONS_VIEW,
          },
          {
            id: 'roles',
            name: 'Roles',
            path: `/${realmId}/manage/roles`,
            icon: Shield,
            permission: PERMISSION.ROLES_VIEW,
          },
          {
            id: 'api-keys',
            name: 'API Keys',
            path: `/${realmId}/manage/api-keys`,
            icon: Key,
            permission: PERMISSION.API_KEYS_VIEW,
          },
        ],
      },
      {
        id: 'products-payments',
        name: 'Products & Payments',
        icon: Briefcase,
        permission: null,
        children: [
          {
            id: 'products',
            name: 'Products',
            path: `/${realmId}/manage/products`,
            icon: Briefcase,
            permission: PERMISSION.BILLING_VIEW,
            visible: adminFeatures?.productsVisible ?? true,
          },
          {
            id: 'payment-providers',
            name: 'Payment Providers',
            path: `/${realmId}/manage/billing/payment-providers`,
            icon: CreditCard,
            permission: PERMISSION.BILLING_VIEW,
            visible: adminFeatures?.billingConfigVisible ?? true,
          },
          {
            id: 'subscription-plans',
            name: 'Subscription Plans',
            path: `/${realmId}/manage/billing`,
            icon: CreditCard,
            permission: PERMISSION.BILLING_VIEW,
            visible: adminFeatures?.plansVisible ?? true,
          },
          {
            id: 'points-packages',
            name: 'Points Packages',
            path: `/${realmId}/manage/points-packages`,
            icon: Coins,
            permission: PERMISSION.POINTS_VIEW,
            visible: adminFeatures?.pointsPackagesVisible ?? true,
          },
          {
            id: 'points-rules',
            name: 'Points Rules',
            path: `/${realmId}/manage/points/configs`,
            icon: Settings,
            permission: PERMISSION.POINTS_VIEW,
            visible: adminFeatures?.pointsVisible ?? true,
          },
        ],
      },
      {
        id: 'transactions',
        name: 'Transactions',
        icon: FileText,
        permission: null,
        children: [
          {
            id: 'invoices',
            name: 'Invoices',
            path: `/${realmId}/manage/billing/invoices`,
            icon: FileText,
            permission: PERMISSION.BILLING_VIEW,
            visible: adminFeatures?.invoicesVisible ?? true,
          },
          {
            id: 'subscription-history',
            name: 'Subscription History',
            path: `/${realmId}/manage/subscription-history`,
            icon: History,
            permission: PERMISSION.BILLING_VIEW,
            visible: adminFeatures?.subscriptionHistoryVisible ?? true,
          },
          {
            id: 'points-wallets',
            name: 'Points Wallets',
            path: `/${realmId}/manage/points/wallets`,
            icon: Users,
            permission: PERMISSION.POINTS_VIEW,
            visible: adminFeatures?.pointsVisible ?? true,
          },
        ],
      },
      {
        id: 'audit-log',
        name: 'Audit Log',
        path: `/${realmId}/manage/audit`,
        icon: ScrollText,
        permission: PERMISSION.AUDIT_VIEW,
      },
      {
        id: 'settings',
        name: 'Settings',
        path: `/${realmId}/manage/settings`,
        icon: Settings,
        permission: PERMISSION.SETTINGS_VIEW,
      },
    ],
    [adminFeatures, realmId]
  )

  const filteredMenuItems = useMemo(
    () => filterByPermission(menuItems, permissions, realmId),
    [menuItems, permissions, realmId]
  )

  const renderMenuItem = (item: MenuItem, level: number = 0) => {
    const hasChildren = item.children && item.children.length > 0
    const isOpen = openMenus.has(item.name)
    const Icon = item.icon

    if (item.visible === false) {
      return null
    }

    // For parent menus, verify at least one child is visible (visible flag only; permissions already filtered)
    const visibleChildren = hasChildren
      ? item.children!.filter((child) => child.visible !== false)
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
        <h1 className="text-xl font-bold">Herald</h1>
        <p className="text-sm text-gray-500">{realm?.name ?? realmId}</p>
      </div>

      <nav className="mt-6 min-h-0 flex-1 overflow-y-auto" data-testid="sidebar-nav">
        {filteredMenuItems.map((item) => renderMenuItem(item))}
      </nav>
    </div>
  )
}
