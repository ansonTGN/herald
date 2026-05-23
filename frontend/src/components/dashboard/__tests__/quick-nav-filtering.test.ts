/**
 * @vitest-environment jsdom
 */

import { describe, expect, it } from 'vitest'
import { filterByPermission } from '@/lib/utils/filter-by-permission'
import { PERMISSION } from '@/lib/constants/auth-constants'

interface NavItem {
  id: string
  title: string
  description: string
  icon: () => null
  path: string
  testId: string
  permission?: string
}

// Dummy icon to satisfy the NavItem interface
const DummyIcon = () => null

function makeItem(overrides: Partial<NavItem> & { title: string }): NavItem {
  const { title } = overrides
  const id = overrides.id ?? title.toLowerCase().replace(/\s+/g, '-')
  return {
    description: 'Test description',
    icon: DummyIcon,
    path: '/test',
    testId: 'test-item',
    ...overrides,
    id,
  }
}

describe('filterByPermission', () => {
  it('shows items when user has the matching permission', () => {
    const items: NavItem[] = [
      makeItem({ title: 'Users', permission: PERMISSION.USERS_VIEW }),
      makeItem({ title: 'Roles', permission: PERMISSION.ROLES_VIEW }),
    ]

    const result = filterByPermission(
      items,
      [PERMISSION.USERS_VIEW, PERMISSION.ROLES_VIEW],
      'test-realm'
    )

    expect(result).toHaveLength(2)
    expect(result.map((i) => i.title)).toEqual(['Users', 'Roles'])
  })

  it('hides items when user lacks the required permission', () => {
    const items: NavItem[] = [
      makeItem({ title: 'Users', permission: PERMISSION.USERS_VIEW }),
      makeItem({ title: 'Roles', permission: PERMISSION.ROLES_VIEW }),
    ]

    const result = filterByPermission(items, [PERMISSION.USERS_VIEW], 'test-realm')

    expect(result).toHaveLength(1)
    expect(result[0].title).toBe('Users')
  })

  it('shows items with no permission field regardless of user permissions', () => {
    const items: NavItem[] = [makeItem({ title: 'Public Page' })]

    const result = filterByPermission(items, [], 'test-realm')

    expect(result).toHaveLength(1)
    expect(result[0].title).toBe('Public Page')
  })

  it('hides Realms when realmId is not admin even if user has REALM_VIEW', () => {
    const items: NavItem[] = [
      makeItem({ title: 'Realms', permission: PERMISSION.REALM_VIEW }),
      makeItem({ title: 'Settings', permission: PERMISSION.SETTINGS_VIEW }),
    ]

    const result = filterByPermission(
      items,
      [PERMISSION.REALM_VIEW, PERMISSION.SETTINGS_VIEW],
      'some-realm'
    )

    expect(result).toHaveLength(1)
    expect(result[0].title).toBe('Settings')
  })

  it('shows Realms when realmId is admin and user has REALM_VIEW', () => {
    const items: NavItem[] = [makeItem({ title: 'Realms', permission: PERMISSION.REALM_VIEW })]

    const result = filterByPermission(items, [PERMISSION.REALM_VIEW], 'admin')

    expect(result).toHaveLength(1)
    expect(result[0].title).toBe('Realms')
  })

  it('hides all permission-gated items when permissions array is empty', () => {
    const items: NavItem[] = [
      makeItem({ title: 'Public', permission: undefined }),
      makeItem({ title: 'Users', permission: PERMISSION.USERS_VIEW }),
      makeItem({ title: 'Roles', permission: PERMISSION.ROLES_VIEW }),
      makeItem({ title: 'Permissions', permission: PERMISSION.PERMISSIONS_VIEW }),
      makeItem({ title: 'Client Apps', permission: PERMISSION.CLIENTS_VIEW }),
      makeItem({ title: 'Settings', permission: PERMISSION.SETTINGS_VIEW }),
    ]

    const result = filterByPermission(items, [], 'test-realm')

    expect(result).toHaveLength(1)
    expect(result[0].title).toBe('Public')
  })
})
