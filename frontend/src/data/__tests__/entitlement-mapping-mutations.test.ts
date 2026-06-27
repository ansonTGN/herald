import { describe, it, expect } from 'vitest'
import { isProtectedPriceError, extractActiveSubscriptions } from '../entitlement-mapping-mutations'
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
