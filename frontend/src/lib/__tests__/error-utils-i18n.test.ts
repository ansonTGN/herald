import { describe, expect, it, vi } from 'vitest'
import { ApiResponseError, getErrorMessage, resolveApiError } from '../error-utils'

vi.mock('@/paraglide/messages', () => ({
  m: new Proxy({}, { get: (_target: unknown, prop: string) => () => `[${prop}]` }),
}))

describe('API error resolution', () => {
  it('shows an actionable backend message for a business error', () => {
    expect(
      getErrorMessage({
        status: 409,
        code: 'email_already_exists',
        message: 'Email already registered',
      })
    ).toBe('Email already registered')
  })

  it('never exposes a backend cause for server errors', () => {
    expect(
      getErrorMessage({ status: 500, code: 'internal_error', message: 'SQL password=secret' })
    ).toBe('[error.server_error]')
  })

  it('keeps the request id visible when the server message is hidden', () => {
    expect(
      getErrorMessage({
        status: 502,
        code: 'upstream_error',
        message: 'provider secret',
        requestId: 'req-123',
      })
    ).toBe('[error.server_error] (req-123)')
  })

  it('understands nested client errors', () => {
    expect(
      resolveApiError({ error: { status: 422, code: 'validation_error', detail: 'Invalid' } })
    ).toMatchObject({ status: 422, code: 'validation_error', message: 'Invalid' })
  })

  it('preserves structured fields when wrapping an API response', () => {
    const error = new ApiResponseError({
      status: 429,
      code: 'rate_limit_exceeded',
      message: 'Try later',
      details: { retryAfter: 10 },
      requestId: 'req-429',
    })

    expect(error).toMatchObject({
      status: 429,
      code: 'rate_limit_exceeded',
      message: 'Try later',
      details: { retryAfter: 10 },
      requestId: 'req-429',
    })
  })

  it.each([
    [new Error('test error'), 'test error'],
    [{ message: 'object message' }, 'object message'],
    [{ detail: 'object detail' }, 'object detail'],
    [{ error_description: 'oauth error' }, 'oauth error'],
    [{ error: 'simple error' }, 'simple error'],
    ['plain string', 'plain string'],
    [null, '[error.generic]'],
  ])('supports existing non-API error shape %#', (error, expected) => {
    expect(getErrorMessage(error)).toBe(expected)
  })
})
