import { renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { usePermission } from '../use-permission'

vi.mock('@/hooks/use-auth', () => ({
  useAuth: () => ({
    permissions: ['billing:manage', 'points:view'],
    roles: ['realm-admin'],
    isLoading: false,
  }),
}))

describe('usePermission', () => {
  it('matches backend colon-formatted permissions with dot-form API checks', () => {
    const { result } = renderHook(() => usePermission())

    expect(result.current.hasPermission('billing.manage')).toBe(true)
    expect(result.current.hasAnyPermission(['settings.manage', 'billing.manage'])).toBe(true)
    expect(result.current.hasAllPermissions(['billing.manage', 'points.view'])).toBe(true)
  })
})
