import { Link } from '@tanstack/react-router'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Users, Shield, Key, Briefcase, Globe, Settings } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { useAuth } from '@/hooks/use-auth'
import { PERMISSION } from '@/lib/constants/auth-constants'
import { filterByPermission } from '@/lib/utils/filter-by-permission'

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
      title: 'Users',
      description: 'Manage users and their permissions',
      icon: Users,
      path: '/$realmId/manage/users',
      testId: 'dashboard-users-card',
      permission: PERMISSION.USERS_VIEW,
    },
    {
      id: 'roles',
      title: 'Roles',
      description: 'Define roles and assign permissions',
      icon: Shield,
      path: '/$realmId/manage/roles',
      testId: 'dashboard-roles-card',
      permission: PERMISSION.ROLES_VIEW,
    },
    {
      id: 'permissions',
      title: 'Permissions',
      description: 'Configure system permissions',
      icon: Key,
      path: '/$realmId/manage/permissions',
      testId: 'dashboard-permissions-card',
      permission: PERMISSION.PERMISSIONS_VIEW,
    },
    {
      id: 'client-apps',
      title: 'Client Apps',
      description: 'Manage OAuth 2.0 client applications',
      icon: Briefcase,
      path: '/$realmId/manage/client-apps',
      testId: 'dashboard-client-apps-card',
      permission: PERMISSION.CLIENTS_VIEW,
    },
    {
      id: 'realms',
      title: 'Realms',
      description: 'Manage realms in the system',
      icon: Globe,
      path: '/$realmId/manage/realms',
      testId: 'dashboard-realms-card',
      permission: PERMISSION.REALM_VIEW,
    },
    {
      id: 'settings',
      title: 'Settings',
      description: 'Configure realm settings',
      icon: Settings,
      path: '/$realmId/manage/settings',
      testId: 'dashboard-settings-card',
      permission: PERMISSION.SETTINGS_VIEW,
    },
  ]

  const visibleItems = filterByPermission(navItems, permissions, realmId)

  return (
    <div data-testid={testId}>
      <h2 className="mb-4 text-lg font-semibold tracking-tight">Quick Navigation</h2>
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
