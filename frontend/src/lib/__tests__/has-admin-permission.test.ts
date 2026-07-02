import { describe, it, expect } from 'vitest'
import { hasAdminPermission, ADMIN_PERMISSIONS, PERMISSION } from '@/lib/constants/auth-constants'

describe('hasAdminPermission', () => {
  it('returns false for undefined input', () => {
    expect(hasAdminPermission(undefined)).toBe(false)
  })

  it('returns false for empty array', () => {
    expect(hasAdminPermission([])).toBe(false)
  })

  it('returns false for an array containing only POINTS_VIEW (user-role permission)', () => {
    expect(hasAdminPermission([PERMISSION.POINTS_VIEW])).toBe(false)
  })

  it.each(ADMIN_PERMISSIONS as readonly string[])(
    'returns true for array containing exactly %s (admin permission)',
    (adminPerm) => {
      expect(hasAdminPermission([adminPerm])).toBe(true)
    }
  )

  it('returns true when mixing one admin permission with non-admin permissions', () => {
    expect(hasAdminPermission([PERMISSION.POINTS_VIEW, PERMISSION.REALM_MANAGE])).toBe(true)
  })

  it('returns true for backend colon-formatted admin permissions', () => {
    expect(hasAdminPermission(['points:view', 'billing:manage'])).toBe(true)
  })

  it('returns false for an array of random non-permission strings', () => {
    expect(hasAdminPermission(['foo', 'bar', 'baz'])).toBe(false)
  })
})
