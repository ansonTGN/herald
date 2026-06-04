import { describe, it, expect, vi, beforeEach } from 'vitest'
import { handleApiError, getErrorMessage } from '../error-utils'

// Mock paraglide messages to return predictable identifiers for each key.
// The real module exports keys with dot notation: m["error.bad_request"]().
// We build a Proxy-based mock so any m["..."] access returns a function
// that produces the key name as a string -- easy to assert against.
const messageMock = new Proxy({} as Record<string, () => string>, {
  get: (_target, prop: string) => {
    // Return a function that produces the key name
    return () => `[${prop}]`
  },
})

vi.mock('@/paraglide/messages', () => ({
  m: new Proxy(
    {},
    {
      get: (_target: unknown, prop: string) => () => `[${prop}]`,
    }
  ),
}))

describe('handleApiError — HTTP status code translation mapping', () => {
  beforeEach(() => {
    vi.spyOn(console, 'error').mockImplementation(() => {})
  })

  const statusCases = [
    { status: 400, expectedKey: 'error.bad_request' },
    { status: 401, expectedKey: 'error.unauthorized' },
    { status: 403, expectedKey: 'error.forbidden' },
    { status: 404, expectedKey: 'error.not_found' },
    { status: 500, expectedKey: 'error.server_error' },
  ] as const

  it.each(statusCases)(
    'maps status $status to translated key $expectedKey',
    ({ status, expectedKey }) => {
      const result = handleApiError({ status })
      expect(result).toBe(`[${expectedKey}]`)
    }
  )

  it('maps 409 without detail to error.conflict', () => {
    const result = handleApiError({ status: 409 })
    expect(result).toBe('[error.conflict]')
  })

  it('maps 409 with detail to the detail string', () => {
    const result = handleApiError({ status: 409, detail: 'Email already registered' })
    expect(result).toBe('Email already registered')
  })

  it('falls back to defaultMessage for unknown status codes', () => {
    const result = handleApiError({ status: 429 }, 'Custom fallback')
    expect(result).toBe('Custom fallback')
  })

  it('falls back to translated generic for unknown status codes without defaultMessage', () => {
    const result = handleApiError({ status: 502 })
    expect(result).toBe('[error.generic]')
  })

  it('passes through string errors', () => {
    const result = handleApiError('Network failure')
    expect(result).toBe('Network failure')
  })

  it('passes through Error instances', () => {
    const result = handleApiError(new Error('Something broke'))
    expect(result).toBe('Something broke')
  })

  it('prefers .message over status code when both present', () => {
    const result = handleApiError({ status: 500, message: 'from message field' })
    expect(result).toBe('from message field')
  })

  it('prefers .detail over status code when message absent', () => {
    const result = handleApiError({ status: 404, detail: 'from detail field' })
    expect(result).toBe('from detail field')
  })
})

describe('getErrorMessage — fallback uses paraglide generic', () => {
  it('returns error.message for Error instances', () => {
    expect(getErrorMessage(new Error('test error'))).toBe('test error')
  })

  it('returns .message from objects with message field', () => {
    expect(getErrorMessage({ message: 'obj message' })).toBe('obj message')
  })

  it('returns .detail from objects with detail field', () => {
    expect(getErrorMessage({ detail: 'obj detail' })).toBe('obj detail')
  })

  it('returns .error_description from objects with that field', () => {
    expect(getErrorMessage({ error_description: 'oauth error' })).toBe('oauth error')
  })

  it('returns .error from objects with error field', () => {
    expect(getErrorMessage({ error: 'simple error' })).toBe('simple error')
  })

  it('returns strings as-is', () => {
    expect(getErrorMessage('plain string')).toBe('plain string')
  })

  it('falls back to translated generic for null', () => {
    expect(getErrorMessage(null)).toBe('[error.generic]')
  })

  it('falls back to translated generic for undefined', () => {
    expect(getErrorMessage(undefined)).toBe('[error.generic]')
  })

  it('falls back to translated generic for objects without known fields', () => {
    expect(getErrorMessage({ code: 123 })).toBe('[error.generic]')
  })
})
