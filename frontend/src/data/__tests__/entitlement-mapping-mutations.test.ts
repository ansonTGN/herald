import { describe, it, expect } from 'vitest'
import {
  isProtectedPriceError,
  extractActiveSubscriptions,
  isRoleNotInRealmError,
} from '../entitlement-mapping-mutations'
import type { ErrorResponse } from '@/lib/api-generated'

// The batch-save mutation follows the repo convention
// `if (response.error) throw response.error`, so the helpers receive the
// thrown `response.error` value. For a 409 the typed error union member is
// `MappingActiveSubscriptionLockErrorBody`; for a 400 it is `ErrorResponse`.

describe('isProtectedPriceError', () => {
  it('returns true for the 409 mapping_in_use lock body', () => {
    // Real thrown shape for a 409 on PUT .../entitlement-mappings/batch
    const lock409 = { code: 'mapping_in_use', activeSubscriptions: 28 }
    expect(isProtectedPriceError(lock409)).toBe(true)
  })

  it('returns true with zero active subscriptions', () => {
    // Edge case: backend may report 0 in a degenerate state; the shape still
    // matches the lock contract and must be treated as protected.
    expect(isProtectedPriceError({ code: 'mapping_in_use', activeSubscriptions: 0 })).toBe(true)
  })

  it('returns false for a 400 ErrorResponse (wrong code)', () => {
    const error400: ErrorResponse = {
      code: 400,
      message: 'Entitlement key does not match ^[a-z0-9-]{1,64}$',
    }
    expect(isProtectedPriceError(error400)).toBe(false)
  })

  it('returns false for a 400 cross-product shared-key rename', () => {
    const error400: ErrorResponse = {
      code: 400,
      message: 'Cross-product shared-key rename is not allowed',
    }
    expect(isProtectedPriceError(error400)).toBe(false)
  })

  it('returns false when code matches but activeSubscriptions is not a number', () => {
    // Defensive: if the backend ever ships the lock without the count, we must
    // not treat it as a lock (the confirmation dialog needs the count).
    expect(isProtectedPriceError({ code: 'mapping_in_use', activeSubscriptions: undefined })).toBe(
      false
    )
  })

  it('returns false for null, undefined, primitives, and Errors', () => {
    expect(isProtectedPriceError(null)).toBe(false)
    expect(isProtectedPriceError(undefined)).toBe(false)
    expect(isProtectedPriceError('mapping_in_use')).toBe(false)
    expect(isProtectedPriceError(new Error('boom'))).toBe(false)
  })
})

describe('extractActiveSubscriptions', () => {
  it('returns the count for the 409 lock body', () => {
    expect(extractActiveSubscriptions({ code: 'mapping_in_use', activeSubscriptions: 28 })).toBe(28)
  })

  it('returns null for a non-lock error', () => {
    const error400: ErrorResponse = { code: 400, message: 'bad' }
    expect(extractActiveSubscriptions(error400)).toBeNull()
  })

  it('returns null for null input', () => {
    expect(extractActiveSubscriptions(null)).toBeNull()
  })
})

// `isRoleNotInRealmError` detects the batch-save 400 cross-realm role body
// (`{ code: 'role_not_in_realm', roleId, realmId }`, design §4.2.2). The helper
// narrows ONLY on `code` (not roleId) — see the real implementation in
// entitlement-mapping-mutations.ts — so the false cases below assert that
// contract rather than a stricter shape the implementation does not enforce.

describe('isRoleNotInRealmError', () => {
  it('returns true for the 400 role_not_in_realm body', () => {
    // Real thrown shape for a 400 when grantedRoleIds contains a role outside
    // the target realm.
    const roleError = { code: 'role_not_in_realm', roleId: 'role-x', realmId: 'realm-1' }
    expect(isRoleNotInRealmError(roleError)).toBe(true)
  })

  it('returns false for the 409 mapping_in_use lock body', () => {
    // Different error family (protected-price lock) — must not be mistaken for
    // a cross-realm role error.
    expect(isRoleNotInRealmError({ code: 'mapping_in_use', activeSubscriptions: 5 })).toBe(false)
  })

  it('returns false for a 400 ErrorResponse (numeric code)', () => {
    // A generic validation 400 carries a numeric `code`, not the role string.
    const error400: ErrorResponse = { code: 400, message: 'bad request' }
    expect(isRoleNotInRealmError(error400)).toBe(false)
  })

  it.each([
    ['null', null],
    ['undefined', undefined],
    ['a primitive string', 'role_not_in_realm'],
    ['an Error instance', new Error('boom')],
  ])('returns false for %s', (_label, value) => {
    expect(isRoleNotInRealmError(value)).toBe(false)
  })
})
