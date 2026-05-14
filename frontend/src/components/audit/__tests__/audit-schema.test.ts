import { describe, it, expect } from 'vitest'
import { auditSearchSchema } from '@/lib/schemas/search-params'
import { CATEGORY_ACTIONS } from '../audit-filter-bar'

describe('auditSearchSchema', () => {
  describe('valid inputs', () => {
    it('accepts empty object (all fields optional)', () => {
      const result = auditSearchSchema.safeParse({})
      expect(result.success).toBe(true)
    })

    it('accepts valid page and pageSize', () => {
      const result = auditSearchSchema.safeParse({ page: 0, pageSize: 20 })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.page).toBe(0)
        expect(result.data.pageSize).toBe(20)
      }
    })

    it('accepts all filter fields', () => {
      const result = auditSearchSchema.safeParse({
        page: 1,
        pageSize: 50,
        category: 'rbac',
        action: 'role.create',
        actorId: 'user-001',
        startTime: '2026-01-01T00:00:00Z',
        endTime: '2026-12-31T23:59:59Z',
      })
      expect(result.success).toBe(true)
    })

    it('accepts pageSize at boundary 1', () => {
      const result = auditSearchSchema.safeParse({ pageSize: 1 })
      expect(result.success).toBe(true)
    })

    it('accepts pageSize at boundary 100', () => {
      const result = auditSearchSchema.safeParse({ pageSize: 100 })
      expect(result.success).toBe(true)
    })

    it('accepts page at boundary 0', () => {
      const result = auditSearchSchema.safeParse({ page: 0 })
      expect(result.success).toBe(true)
    })
  })

  describe('invalid inputs', () => {
    it('rejects negative page', () => {
      const result = auditSearchSchema.safeParse({ page: -1 })
      expect(result.success).toBe(false)
    })

    it('rejects pageSize of 0', () => {
      const result = auditSearchSchema.safeParse({ pageSize: 0 })
      expect(result.success).toBe(false)
    })

    it('rejects pageSize above 100', () => {
      const result = auditSearchSchema.safeParse({ pageSize: 101 })
      expect(result.success).toBe(false)
    })

    it('rejects non-integer page', () => {
      const result = auditSearchSchema.safeParse({ page: 1.5 })
      expect(result.success).toBe(false)
    })

    it('rejects non-integer pageSize', () => {
      const result = auditSearchSchema.safeParse({ pageSize: 20.5 })
      expect(result.success).toBe(false)
    })

    it('strips unknown fields (does not include them in output)', () => {
      const result = auditSearchSchema.safeParse({ unknownField: 'value', page: 0 })
      expect(result.success).toBe(true)
      if (result.success) {
        expect((result.data as Record<string, unknown>).unknownField).toBeUndefined()
        expect(result.data.page).toBe(0)
      }
    })
  })
})

describe('CATEGORY_ACTIONS', () => {
  it('has 4 categories', () => {
    expect(Object.keys(CATEGORY_ACTIONS)).toHaveLength(4)
  })

  it('user_management has 3 actions in dot notation', () => {
    const actions = CATEGORY_ACTIONS.user_management
    expect(actions).toEqual(['user.create', 'user.update', 'user.delete'])
    expect(actions).toHaveLength(3)
    actions.forEach((action) => {
      expect(action).toMatch(/^\w+\.\w+$/)
    })
  })

  it('rbac has 9 actions in dot notation', () => {
    const actions = CATEGORY_ACTIONS.rbac
    expect(actions).toHaveLength(9)
    expect(actions).toEqual([
      'role.create',
      'role.update',
      'role.delete',
      'permission.create',
      'permission.delete',
      'role.assign',
      'role.unassign',
      'permission.grant',
      'permission.revoke',
    ])
    actions.forEach((action) => {
      expect(action).toMatch(/^\w+\.\w+$/)
    })
  })

  it('realm_management has 2 actions in dot notation', () => {
    const actions = CATEGORY_ACTIONS.realm_management
    expect(actions).toEqual(['realm.create', 'realm.rbac_init'])
    expect(actions).toHaveLength(2)
    actions.forEach((action) => {
      expect(action).toMatch(/^\w+\.\w+$/)
    })
  })

  it('auth has 3 actions in dot notation', () => {
    const actions = CATEGORY_ACTIONS.auth
    expect(actions).toEqual(['auth.login', 'auth.logout', 'auth.login_failed'])
    expect(actions).toHaveLength(3)
    actions.forEach((action) => {
      expect(action).toMatch(/^\w+\.\w+$/)
    })
  })

  it('totals 17 actions across all categories', () => {
    const total = Object.values(CATEGORY_ACTIONS).reduce((sum, actions) => sum + actions.length, 0)
    expect(total).toBe(17)
  })

  it('has no duplicate actions across categories', () => {
    const allActions = Object.values(CATEGORY_ACTIONS).flat()
    const uniqueActions = new Set(allActions)
    expect(uniqueActions.size).toBe(allActions.length)
  })
})
