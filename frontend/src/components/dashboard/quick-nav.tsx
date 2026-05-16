import { Link } from '@tanstack/react-router'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Users, Shield, Key, Briefcase, Globe, Settings } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'

interface QuickNavProps {
  realmId: string
  testId?: string
}

interface NavItem {
  title: string
  description: string
  icon: LucideIcon
  path: string
  testId: string
}

export function QuickNav({ realmId, testId }: QuickNavProps) {
  const navItems: NavItem[] = [
    {
      title: 'Users',
      description: 'Manage users and their permissions',
      icon: Users,
      path: '/$realmId/manage/users',
      testId: 'dashboard-users-card',
    },
    {
      title: 'Roles',
      description: 'Define roles and assign permissions',
      icon: Shield,
      path: '/$realmId/manage/roles',
      testId: 'dashboard-roles-card',
    },
    {
      title: 'Permissions',
      description: 'Configure system permissions',
      icon: Key,
      path: '/$realmId/manage/permissions',
      testId: 'dashboard-permissions-card',
    },
    {
      title: 'Client Apps',
      description: 'Manage OAuth 2.0 client applications',
      icon: Briefcase,
      path: '/$realmId/manage/client-apps',
      testId: 'dashboard-client-apps-card',
    },
    {
      title: 'Realms',
      description: 'Manage realms in the system',
      icon: Globe,
      path: '/$realmId/manage/realms',
      testId: 'dashboard-realms-card',
    },
    {
      title: 'Settings',
      description: 'Configure realm settings',
      icon: Settings,
      path: '/$realmId/manage/settings',
      testId: 'dashboard-settings-card',
    },
  ]

  return (
    <div data-testid={testId}>
      <h2 className="mb-4 text-xl font-semibold">Quick Navigation</h2>
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {navItems.map((item) => {
          const Icon = item.icon
          return (
            <Link key={item.path} to={item.path} params={{ realmId }}>
              <Card
                className="cursor-pointer transition-colors hover:bg-accent"
                data-testid={item.testId}
              >
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">{item.title}</CardTitle>
                  <Icon className="h-4 w-4 text-muted-foreground" />
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
