/**
 * Passkey-status query-option tests.
 *
 * Target: `passkeyStatusQueryOptions(realmId)` — the public Passkey enablement
 * flag consumed by the login route to gate the passkey entry BEFORE the
 * PasskeyLoginForm is mounted (so a realm with passkey disabled never fires the
 * begin-options probe). Mirrors `emailOtpStatusQueryOptions`.
 *
 * High-value contracts this protects:
 *  - The query is realm-scoped: distinct realms must produce distinct cache
 *    entries (otherwise two realms' passkey flags would collapse into one).
 *  - It must NOT collide with the email-otp-status key (both gate login
 *    entries on the same login page; collapsing them would let one realm's OTP
 *    flag decide another realm's passkey entry).
 *  - The queryFn issues a GET to `/api/auth/{realmId}/passkey/status` and
 *    unwraps `response.data`; a non-200 must surface (no silent fold to a
 *    default that could hide a misconfigured realm).
 *
 * Per `testing.md`: do NOT assert `typeof queryFn === 'function'` /
 * `staleTime` constants (library-guaranteed / no business contract).
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/mocks/server'
import {
  passkeyStatusQueryOptions,
  emailOtpStatusQueryOptions,
  queryKeys,
} from '@/data/query-options'
import { QUERY_KEYS } from '@/lib/constants'

const API_BASE_URL = 'http://localhost:3000'
const PASSKEY_STATUS_URL = `${API_BASE_URL}/api/auth/:realmId/passkey/status`

beforeEach(() => {
  server.resetHandlers()
})

afterEach(() => {
  server.resetHandlers()
})

describe('passkeyStatusQueryOptions — realm-scoped isolation', () => {
  it('is realm-scoped (one representative shape)', () => {
    const options = passkeyStatusQueryOptions('realm-1')
    expect(options.queryKey).toEqual([QUERY_KEYS.PASSKEY_STATUS, 'realm-1'])
  })

  it('produces distinct keys for different realms', () => {
    const a = passkeyStatusQueryOptions('realm-1')
    const b = passkeyStatusQueryOptions('realm-2')
    expect(a.queryKey).not.toEqual(b.queryKey)
  })

  it('does not collide with the email-otp-status query key (different key constant)', () => {
    // The two login-entry-gating queries (Passkey status vs OTP status) must
    // not share a cache entry — distinct key constants, even when realmId
    // matches, otherwise one realm's OTP flag could decide its passkey entry.
    const passkey = passkeyStatusQueryOptions('realm-1').queryKey
    const otp = emailOtpStatusQueryOptions('realm-1').queryKey

    expect(passkey[0]).not.toBe(otp[0])
    expect(passkey).not.toEqual(otp)
  })

  it('queryKeys.passkeyStatus helper composes the same key shape', () => {
    expect(passkeyStatusQueryOptions('realm-1').queryKey).toEqual(
      queryKeys.passkeyStatus('realm-1')
    )
  })
})

describe('passkeyStatusQueryOptions — queryFn GET path and data unwrap', () => {
  it('issues a GET to the passkey/status path and unwraps data', async () => {
    let capturedRequest: Request | undefined
    server.use(
      http.get(PASSKEY_STATUS_URL, ({ request }) => {
        capturedRequest = request as Request
        return HttpResponse.json({ enabled: true })
      })
    )

    const options = passkeyStatusQueryOptions('realm-1')
    const data = await options.queryFn()

    expect(capturedRequest).toBeDefined()
    expect(new URL(capturedRequest!.url).pathname).toBe('/api/auth/realm-1/passkey/status')
    expect(capturedRequest!.method).toBe('GET')
    expect(data).toEqual({ enabled: true })
  })

  it('returns { enabled: false } when the realm has not configured passkey (200, not an error)', async () => {
    // Contract: a realm that never configured passkey reads enabled:false via
    // a 200 — the whole point of this endpoint vs the old 404-from-options
    // probe. The login page gates on the value, not the status code.
    server.use(http.get(PASSKEY_STATUS_URL, () => HttpResponse.json({ enabled: false })))

    const data = await passkeyStatusQueryOptions('realm-1').queryFn()
    expect(data).toEqual({ enabled: false })
  })

  it('surfaces a non-200 response by throwing response.error (no silent fold)', async () => {
    server.use(
      http.get(PASSKEY_STATUS_URL, () =>
        HttpResponse.json({ message: 'Internal server error' }, { status: 500 })
      )
    )

    await expect(passkeyStatusQueryOptions('realm-1').queryFn()).rejects.toMatchObject({
      message: 'Internal server error',
    })
  })
})
