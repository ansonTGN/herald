/**
 * @vitest-environment jsdom
 */

import { describe, expect, it } from 'vitest'
import { filterByPermission } from '@/lib/utils/filter-by-permission'
import { PERMISSION } from '@/lib/constants/auth-constants'

interface MenuItem {
  id: string
  name: string
  path?: string
  icon: () => null
  permission: string | null
  visible?: boolean
  children?: MenuItem[]
}

// Dummy icon component — only needed to satisfy the MenuItem interface
const DummyIcon = () => null

function makeItem(overrides: Partial<MenuItem> & { name: string }): MenuItem {
  const { name } = overrides
  const id = overrides.id ?? name.toLowerCase().replace(/\s+/g, '-')
  return { icon: DummyIcon, permission: null, ...overrides, id }
}

function makeParent(name: string, children: MenuItem[], overrides?: Partial<MenuItem>): MenuItem {
  const id = overrides?.id ?? name.toLowerCase().replace(/\s+/g, '-')
  return { icon: DummyIcon, permission: null, ...overrides, id, name, children }
}

describe('filterByPermission', () => {
  it('keeps items with permission: null visible regardless of user permissions', () => {
    const items: MenuItem[] = [makeItem({ name: 'Public Page', path: '/public' })]

    const result = filterByPermission(items, [])

    expect(result).toHaveLength(1)
    expect(result[0].name).toBe('Public Page')
  })

  it('shows Dashboard item when user has dashboard.view', () => {
    const items: MenuItem[] = [
      makeItem({ name: 'Dashboard', path: '/manage', permission: PERMISSION.DASHBOARD_VIEW }),
    ]

    const result = filterByPermission(items, [PERMISSION.DASHBOARD_VIEW])

    expect(result).toHaveLength(1)
    expect(result[0].name).toBe('Dashboard')
  })

  it('hides Dashboard item when user lacks dashboard.view', () => {
    const items: MenuItem[] = [
      makeItem({ name: 'Dashboard', path: '/manage', permission: PERMISSION.DASHBOARD_VIEW }),
    ]

    const result = filterByPermission(items, [])

    expect(result).toHaveLength(0)
  })

  // The permission check itself is generic (filter-by-permission.ts has no
  // per-permission logic), so one shows/hides pair pins the branch for every
  // menu item; only realm-gated 'realms' has its own tests below.

  it('hides parent when ALL children are filtered out by permissions', () => {
    const items: MenuItem[] = [
      makeParent('Authorization', [
        makeItem({ name: 'Roles', path: '/roles', permission: PERMISSION.ROLES_VIEW }),
        makeItem({
          name: 'Permissions',
          path: '/permissions',
          permission: PERMISSION.PERMISSIONS_VIEW,
        }),
      ]),
    ]

    // User has no relevant permissions
    const result = filterByPermission(items, [PERMISSION.DASHBOARD_VIEW])

    expect(result).toHaveLength(0)
  })

  it('shows parent when at least one child passes permission check', () => {
    const items: MenuItem[] = [
      makeParent('Authorization', [
        makeItem({ name: 'Roles', path: '/roles', permission: PERMISSION.ROLES_VIEW }),
        makeItem({
          name: 'Permissions',
          path: '/permissions',
          permission: PERMISSION.PERMISSIONS_VIEW,
        }),
      ]),
    ]

    // User has roles.view but not permissions.view
    const result = filterByPermission(items, [PERMISSION.ROLES_VIEW])

    expect(result).toHaveLength(1)
    expect(result[0].name).toBe('Authorization')
    expect(result[0].children).toHaveLength(1)
    expect(result[0].children![0].name).toBe('Roles')
  })

  it('hides all permission-gated items with empty permissions but keeps null-permission items', () => {
    const items: MenuItem[] = [
      makeItem({ name: 'Public Page', path: '/public' }),
      makeItem({ name: 'Dashboard', path: '/manage', permission: PERMISSION.DASHBOARD_VIEW }),
      makeItem({ name: 'Settings', path: '/settings', permission: PERMISSION.SETTINGS_VIEW }),
      makeParent('Authorization', [
        makeItem({ name: 'Roles', path: '/roles', permission: PERMISSION.ROLES_VIEW }),
      ]),
    ]

    const result = filterByPermission(items, [])

    // Only the null-permission leaf item survives; parent with no visible children is removed
    expect(result).toHaveLength(1)
    expect(result[0].name).toBe('Public Page')
  })

  it('hides Realms when realmId is not admin even if user has REALM_VIEW', () => {
    const items: MenuItem[] = [
      makeItem({ name: 'Realms', path: '/realms', permission: PERMISSION.REALM_VIEW }),
      makeItem({ name: 'Dashboard', path: '/manage', permission: PERMISSION.DASHBOARD_VIEW }),
    ]

    const result = filterByPermission(
      items,
      [PERMISSION.REALM_VIEW, PERMISSION.DASHBOARD_VIEW],
      'some-realm'
    )

    expect(result).toHaveLength(1)
    expect(result[0].name).toBe('Dashboard')
  })

  it('shows Realms when realmId is admin and user has REALM_VIEW', () => {
    const items: MenuItem[] = [
      makeItem({ name: 'Realms', path: '/realms', permission: PERMISSION.REALM_VIEW }),
    ]

    const result = filterByPermission(items, [PERMISSION.REALM_VIEW], 'admin')

    expect(result).toHaveLength(1)
    expect(result[0].name).toBe('Realms')
  })
})
