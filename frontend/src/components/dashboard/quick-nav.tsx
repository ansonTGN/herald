import { Link } from '@tanstack/react-router'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Users, Shield, Key, Briefcase, Globe, Settings } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { useAuth } from '@/hooks/use-auth'
import { PERMISSION } from '@/lib/constants/auth-constants'
import { filterByPermission } from '@/lib/utils/filter-by-permission'
import { m } from '@/paraglide/messages'

interface QuickNavProps {
  realmId: string
  testId?: string
}

interface NavItem {
  id: string
  title: string
  description: string
  icon: LucideIcon
  path: string
  testId: string
  permission?: string
}

export function QuickNav({ realmId, testId }: QuickNavProps) {
  const { permissions } = useAuth()

  const navItems: NavItem[] = [
    {
      id: 'users',
      title: m['dashboard.nav_users_title'](),
      description: m['dashboard.nav_users_desc'](),
      icon: Users,
      path: '/$realmId/manage/users',
      testId: 'dashboard-users-card',
      permission: PERMISSION.USERS_VIEW,
    },
    {
      id: 'roles',
      title: m['dashboard.nav_roles_title'](),
      description: m['dashboard.nav_roles_desc'](),
      icon: Shield,
      path: '/$realmId/manage/roles',
      testId: 'dashboard-roles-card',
      permission: PERMISSION.ROLES_VIEW,
    },
    {
      id: 'permissions',
      title: m['dashboard.nav_permissions_title'](),
      description: m['dashboard.nav_permissions_desc'](),
      icon: Key,
      path: '/$realmId/manage/permissions',
      testId: 'dashboard-permissions-card',
      permission: PERMISSION.PERMISSIONS_VIEW,
    },
    {
      id: 'client-apps',
      title: m['dashboard.nav_client_apps_title'](),
      description: m['dashboard.nav_client_apps_desc'](),
      icon: Briefcase,
      path: '/$realmId/manage/client-apps',
      testId: 'dashboard-client-apps-card',
      permission: PERMISSION.CLIENTS_VIEW,
    },
    {
      id: 'realms',
      title: m['dashboard.nav_realms_title'](),
      description: m['dashboard.nav_realms_desc'](),
      icon: Globe,
      path: '/$realmId/manage/realms',
      testId: 'dashboard-realms-card',
      permission: PERMISSION.REALM_VIEW,
    },
    {
      id: 'settings',
      title: m['dashboard.nav_settings_title'](),
      description: m['dashboard.nav_settings_desc'](),
      icon: Settings,
      path: '/$realmId/manage/settings',
      testId: 'dashboard-settings-card',
      permission: PERMISSION.SETTINGS_VIEW,
    },
  ]

  const visibleItems = filterByPermission(navItems, permissions, realmId)

  return (
    <div data-testid={testId}>
      <h2 className="mb-4 text-lg font-semibold tracking-tight">
        {m['dashboard.quick_navigation']()}
      </h2>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {visibleItems.map((item) => {
          const Icon = item.icon
          return (
            <Link key={item.path} to={item.path} params={{ realmId }}>
              <Card
                className="cursor-pointer transition-all duration-200 hover:shadow-md hover:border-primary/20 group"
                data-testid={item.testId}
              >
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">{item.title}</CardTitle>
                  <div className="flex size-8 items-center justify-center rounded-lg bg-primary/10 transition-colors group-hover:bg-primary/15">
                    <Icon className="size-4 text-primary" />
                  </div>
                </CardHeader>
                <CardContent>
                  <p className="text-xs text-muted-foreground">{item.description}</p>
                </CardContent>
              </Card>
            </Link>
          )
        })}
      </div>
    </div>
  )
}
