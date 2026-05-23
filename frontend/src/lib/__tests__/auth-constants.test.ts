import { describe, it, expect } from 'vitest'
import { PERMISSION, ADMIN_PERMISSIONS } from '@/lib/constants/auth-constants'

describe('PERMISSION constant object', () => {
  it('does NOT contain legacy REALM_ADMIN or REALM_CREATE keys', () => {
    expect(PERMISSION).not.toHaveProperty('REALM_ADMIN')
    expect(PERMISSION).not.toHaveProperty('REALM_CREATE')
  })

  it('contains DASHBOARD_VIEW, AUDIT_VIEW, API_KEYS_VIEW, REALM_MANAGE keys', () => {
    expect(PERMISSION).toHaveProperty('DASHBOARD_VIEW')
    expect(PERMISSION).toHaveProperty('AUDIT_VIEW')
    expect(PERMISSION).toHaveProperty('API_KEYS_VIEW')
    expect(PERMISSION).toHaveProperty('REALM_MANAGE')
  })

  it('maps each new key to the correct resource.action string value', () => {
    expect(PERMISSION.DASHBOARD_VIEW).toBe('dashboard.view')
    expect(PERMISSION.AUDIT_VIEW).toBe('audit.view')
    expect(PERMISSION.API_KEYS_VIEW).toBe('api_keys.view')
    expect(PERMISSION.REALM_MANAGE).toBe('realm.manage')
  })
})

describe('ADMIN_PERMISSIONS array', () => {
  it('does NOT contain REALM_ADMIN, REALM_CREATE, or POINTS_VIEW', () => {
    // REALM_ADMIN and REALM_CREATE no longer exist in PERMISSION;
    // guard against accidental re-addition via string literals
    expect(ADMIN_PERMISSIONS).not.toContain('realm.admin')
    expect(ADMIN_PERMISSIONS).not.toContain('realm.create')
    // POINTS_VIEW belongs to the user role, not admin
    expect(ADMIN_PERMISSIONS).not.toContain(PERMISSION.POINTS_VIEW)
  })

  it('contains DASHBOARD_VIEW, AUDIT_VIEW, API_KEYS_VIEW, REALM_MANAGE', () => {
    expect(ADMIN_PERMISSIONS).toContain(PERMISSION.DASHBOARD_VIEW)
    expect(ADMIN_PERMISSIONS).toContain(PERMISSION.AUDIT_VIEW)
    expect(ADMIN_PERMISSIONS).toContain(PERMISSION.API_KEYS_VIEW)
    expect(ADMIN_PERMISSIONS).toContain(PERMISSION.REALM_MANAGE)
  })

  it('has no duplicate entries', () => {
    const unique = new Set(ADMIN_PERMISSIONS)
    expect(ADMIN_PERMISSIONS).toHaveLength(unique.size)
  })

  it('every value corresponds to a key in the PERMISSION object', () => {
    const permissionValues = new Set(Object.values(PERMISSION))
    for (const entry of ADMIN_PERMISSIONS) {
      expect(permissionValues).toContain(entry)
    }
  })
})
