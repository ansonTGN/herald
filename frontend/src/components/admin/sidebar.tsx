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

    const visibleChildren = hasChildren
      ? item.children!.filter((child) => child.visible !== false)
      : []

    if (hasChildren && visibleChildren.length === 0) {
      return null
    }

    const paddingLeft = level > 0 ? 'pl-12 pr-4' : 'px-4'

    return (
      <div key={item.name}>
        {!hasChildren && item.path ? (
          <Link
            to={item.path}
            className={`group relative flex items-center gap-3 rounded-lg py-2.5 text-sm font-medium text-sidebar-foreground/70 transition-all duration-150 hover:bg-sidebar-accent hover:text-sidebar-foreground ${paddingLeft}`}
            activeProps={{
              className:
                'bg-sidebar-accent text-sidebar-foreground font-semibold before:absolute before:left-0 before:top-1/2 before:-translate-y-1/2 before:h-5 before:w-[3px] before:rounded-r-full before:bg-sidebar-primary',
            }}
            activeOptions={{ exact: true }}
            data-testid={`sidebar-menu-${item.name.toLowerCase().replace(/\s+/g, '-')}`}
          >
            <Icon className="size-[18px] shrink-0 opacity-60 group-hover:opacity-100 group-[.font-semibold]:opacity-100 transition-opacity" />
            <span className="truncate">{item.name}</span>
          </Link>
        ) : (
          <div
            onClick={() => hasChildren && toggleMenu(item.name)}
            className={`group flex items-center gap-3 rounded-lg py-2.5 text-sm font-medium text-sidebar-foreground/60 cursor-pointer transition-all duration-150 hover:bg-sidebar-accent hover:text-sidebar-foreground/90 px-4`}
            data-testid={`sidebar-menu-${item.name.toLowerCase().replace(/\s+/g, '-')}`}
          >
            <Icon className="size-[18px] shrink-0 opacity-60 group-hover:opacity-100 transition-opacity" />
            <span className="flex-1 truncate">{item.name}</span>
            {hasChildren && (
              <ChevronDown
                className={`size-4 shrink-0 text-sidebar-foreground/40 transition-transform duration-200 ${isOpen ? 'rotate-0' : '-rotate-90'}`}
              />
            )}
          </div>
        )}

        {hasChildren && isOpen && (
          <div className="mt-0.5 space-y-px">
            {visibleChildren.map((child) => renderMenuItem(child, level + 1))}
          </div>
        )}
      </div>
    )
  }

  return (
    <div
      className="flex h-full min-h-0 w-64 flex-col bg-sidebar border-r border-sidebar-border"
      data-testid="admin-sidebar"
    >
      <div className="shrink-0 px-5 pt-6 pb-4">
        <h1 className="text-lg font-bold tracking-tight text-sidebar-foreground">Herald</h1>
        <p className="mt-0.5 text-xs font-medium text-sidebar-foreground/40">
          {realm?.name ?? realmId}
        </p>
      </div>

      <div className="mx-4 mb-3 h-px bg-sidebar-border" />

      <nav className="min-h-0 flex-1 overflow-y-auto px-2 pb-4" data-testid="sidebar-nav">
        <div className="space-y-0.5">{filteredMenuItems.map((item) => renderMenuItem(item))}</div>
      </nav>
    </div>
  )
}
