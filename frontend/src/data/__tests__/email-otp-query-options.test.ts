/**
 * Email-OTP query-option isolation tests (FE-T01 step 2, design §4.5).
 *
 * Target: the migrated `turnstileStatusQueryOptions(realmId, clientId)` (FE-D01)
 * + `emailOtpStatusQueryOptions(realmId)`.
 *
 * High-value contract this protects:
 *  - The Turnstile query was migrated from realm-only to **Client-App-keyed**
 *    (design §4.5 D-PROTECT-01). The old realm-only signature would COLLAPSE
 *    distinct Client Apps' Turnstile status into one cache entry — so the query
 *    key MUST include `clientId`. This is a targeted regression for that
 *    migration: two distinct `clientId` values must produce distinct keys.
 *  - The shipped contract `GetTurnstileStatusData` is `{ body?: never; path;
 *    query: { clientId } }` (FE-D01 CONTRACT NOTE) — i.e. clientId travels as
 *    a **GET query param**, not a POST body. The `queryFn` must issue the
 *    request with `query: { clientId }`.
 *  - `emailOtpStatusQueryOptions` is realm-scoped and distinct from the
 *    turnstile key (no collision between the two OTP-area queries).
 *
 * Per `testing.md`: do NOT assert `typeof queryFn === 'function'` /
 * `staleTime` constants (library-guaranteed / no business contract). Choose
 * 1-2 representative key shapes rather than enumerating every field.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/mocks/server'
import {
  turnstileStatusQueryOptions,
  emailOtpStatusQueryOptions,
  queryKeys,
} from '@/data/query-options'
import { QUERY_KEYS } from '@/lib/constants'

const API_BASE_URL = 'http://localhost:3000'
const TURNSTILE_STATUS_URL = `${API_BASE_URL}/api/auth/:realmId/turnstile/status`
const EMAIL_OTP_STATUS_URL = `${API_BASE_URL}/api/auth/:realmId/email-otp/status`

beforeEach(() => {
  server.resetHandlers()
})

afterEach(() => {
  server.resetHandlers()
})

// ===========================================================================
// turnstileStatusQueryOptions — Client-App-keyed isolation (§4.5 regression)
// ===========================================================================

describe('turnstileStatusQueryOptions — clientId-keyed query key', () => {
  it('includes clientId in the query key (one representative shape)', () => {
    const options = turnstileStatusQueryOptions('realm-1', 'client-a')
    // Representative key shape: [TURNSTILE_STATUS, realmId, clientId]. Asserting
    // the full tuple protects the client-app migration (the old realm-only key
    // would not have the third element).
    expect(options.queryKey).toEqual([QUERY_KEYS.TURNSTILE_STATUS, 'realm-1', 'client-a'])
  })

  it('two different clientId values produce distinct query keys', () => {
    // Regression for design §4.5: the realm-only signature collapsed distinct
    // Client Apps; the client-app migration must NOT.
    const a = turnstileStatusQueryOptions('realm-1', 'client-a')
    const b = turnstileStatusQueryOptions('realm-1', 'client-b')

    expect(a.queryKey).not.toEqual(b.queryKey)
    // Same realm, different clientId → only the clientId slot differs.
    expect(a.queryKey[0]).toBe(b.queryKey[0]) // same key constant
    expect(a.queryKey[1]).toBe(b.queryKey[1]) // same realmId
    expect(a.queryKey[2]).not.toBe(b.queryKey[2]) // different clientId
  })

  it('different realms also produce distinct keys (realm + clientId both participate)', () => {
    const a = turnstileStatusQueryOptions('realm-1', 'client-a')
    const b = turnstileStatusQueryOptions('realm-2', 'client-a')

    expect(a.queryKey).not.toEqual(b.queryKey)
  })

  it('queryKeys.turnstileStatus helper composes the same key shape', () => {
    // The options factory must use the same key helper as the rest of the app
    // so prefetch / invalidate call sites hit the same cache entries.
    expect(turnstileStatusQueryOptions('realm-1', 'client-a').queryKey).toEqual(
      queryKeys.turnstileStatus('realm-1', 'client-a')
    )
  })
})

// ===========================================================================
// turnstileStatusQueryOptions — queryFn issues a GET with ?clientId
// (shipped contract: body?: never, query: { clientId })
// ===========================================================================

describe('turnstileStatusQueryOptions — queryFn GET with clientId query param', () => {
  it('issues a GET (not POST) with clientId as a query parameter, no request body', async () => {
    let capturedRequest: Request | undefined
    server.use(
      http.get(TURNSTILE_STATUS_URL, ({ request }) => {
        capturedRequest = request as Request
        return HttpResponse.json({ enabled: true, siteKey: 'site-key-a' })
      })
    )

    const options = turnstileStatusQueryOptions('realm-1', 'client-a')
    const data = await options.queryFn()

    expect(capturedRequest).toBeDefined()
    const url = new URL(capturedRequest!.url)
    // clientId travels as a query param.
    expect(url.searchParams.get('clientId')).toBe('client-a')
    // Path carries the realmId.
    expect(url.pathname).toBe('/api/auth/realm-1/turnstile/status')
    // Method is GET (the shipped contract `GetTurnstileStatusData.body?: never`).
    expect(capturedRequest!.method).toBe('GET')
    // No request body on a GET (cannot have one; asserted defensively).
    expect(capturedRequest!.body).toBeNull()

    // queryFn unwraps response.data on success.
    expect(data).toEqual({ enabled: true, siteKey: 'site-key-a' })
  })

  it('a different clientId produces a separate request targeting that clientId', async () => {
    const seenClientIds: string[] = []
    server.use(
      http.get(TURNSTILE_STATUS_URL, ({ request }) => {
        seenClientIds.push(new URL(request.url).searchParams.get('clientId')!)
        return HttpResponse.json({ enabled: false, siteKey: null })
      })
    )

    await turnstileStatusQueryOptions('realm-1', 'client-a').queryFn()
    await turnstileStatusQueryOptions('realm-1', 'client-b').queryFn()

    // The two clientIds reached the wire — they were not collapsed.
    expect(seenClientIds).toEqual(['client-a', 'client-b'])
  })

  it('surfaces a non-200 response by throwing response.error (no silent fold)', async () => {
    server.use(
      http.get(TURNSTILE_STATUS_URL, () =>
        HttpResponse.json({ message: 'Client app not found' }, { status: 401 })
      )
    )

    const options = turnstileStatusQueryOptions('realm-1', 'missing-client')
    await expect(options.queryFn()).rejects.toMatchObject({
      message: 'Client app not found',
    })
  })
})

// ===========================================================================
// emailOtpStatusQueryOptions — realm-scoped, distinct from turnstile key
// ===========================================================================

describe('emailOtpStatusQueryOptions — realm-scoped isolation', () => {
  it('is realm-scoped (one representative shape)', () => {
    const options = emailOtpStatusQueryOptions('realm-1')
    expect(options.queryKey).toEqual([QUERY_KEYS.EMAIL_OTP_STATUS, 'realm-1'])
  })

  it('produces distinct keys for different realms', () => {
    const a = emailOtpStatusQueryOptions('realm-1')
    const b = emailOtpStatusQueryOptions('realm-2')
    expect(a.queryKey).not.toEqual(b.queryKey)
  })

  it('does not collide with the turnstile-status query key (different key constant)', () => {
    // The two OTP-area queries (OTP status vs Turnstile status) must not share a
    // cache entry — distinct key constants, even when realmId matches.
    const otp = emailOtpStatusQueryOptions('realm-1').queryKey
    const turnstile = turnstileStatusQueryOptions('realm-1', 'client-a').queryKey

    expect(otp[0]).not.toBe(turnstile[0])
    expect(otp).not.toEqual(turnstile)
  })

  it('queryFn issues a GET to the email-otp/status path and unwraps data', async () => {
    let capturedRequest: Request | undefined
    server.use(
      http.get(EMAIL_OTP_STATUS_URL, ({ request }) => {
        capturedRequest = request as Request
        return HttpResponse.json({ enabled: true })
      })
    )

    const options = emailOtpStatusQueryOptions('realm-1')
    const data = await options.queryFn()

    expect(capturedRequest).toBeDefined()
    expect(new URL(capturedRequest!.url).pathname).toBe('/api/auth/realm-1/email-otp/status')
    expect(capturedRequest!.method).toBe('GET')
    expect(data).toEqual({ enabled: true })
  })
})
