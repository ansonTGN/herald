import { describe, expect, it } from 'vitest'
import { queryKeys } from '@/data/query-options'
import { QUERY_KEYS } from '@/lib/constants'

describe('queryKeys', () => {
  it('builds stable prefix keys for reusable admin resources', () => {
    expect(queryKeys.usersList('realm-1')).toEqual([QUERY_KEYS.USERS, 'realm-1'])
    expect(queryKeys.clientAppsList('realm-1')).toEqual([QUERY_KEYS.CLIENT_APPS, 'realm-1'])
    expect(queryKeys.planAssignmentsList('realm-1')).toEqual([
      QUERY_KEYS.PLAN_ASSIGNMENTS,
      'realm-1',
    ])
  })

  it('keeps detailed keys namespaced under the same family', () => {
    expect(queryKeys.adminUserRoles('realm-1', 'user-1')).toEqual([
      QUERY_KEYS.ADMIN_USER_ROLES,
      'realm-1',
      'user-1',
    ])
    expect(queryKeys.subscriptionDetails('realm-1', 'sub-1')).toEqual([
      QUERY_KEYS.SUBSCRIPTION_DETAILS,
      'realm-1',
      'sub-1',
    ])
    expect(queryKeys.turnstileStatus('realm-1')).toEqual([QUERY_KEYS.TURNSTILE_STATUS, 'realm-1'])
  })
})
