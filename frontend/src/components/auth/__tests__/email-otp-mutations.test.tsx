/**
 * Email-OTP mutation logic tests (FE-T01 step 1, design §4.2 error matrix).
 *
 * The dev component test (FE-D01 `email-otp-login-form.test.tsx`) mocks the
 * generated `send`/`verify` SDK functions with `vi.mock` and asserts form-level
 * behavior. This file instead protects the **logic layer** the form test mocks
 * away: the payload composition inside `useEmailOtpSendMutation` /
 * `useEmailOtpVerifyMutation` and the 409-conflict classification
 * (`consent_required` vs `email_not_registered`) — asserted at the **network
 * boundary** via MSW so the SDK → wire contract is protected, not at internal
 * function-call args.
 *
 * Per `testing.md` ("用 MSW 验证 request body，而不是 mock 内部函数调用参数"),
 * we capture the request body from `server.use(http.post(...))` rather than
 * mocking the generated `send`/`verify` functions. The generated client's body
 * shape here is reliable (it just forwards `body` as JSON), so MSW is the right
 * boundary.
 *
 * Verification-step note (FE-T01 CONTRACT NOTE): `EmailOtpVerifyRequest`
 * carries an optional `turnstileToken?`, but design §4.2.2 verify table does
 * NOT list it and FE-D01 forwards `turnstileToken` only into the SEND mutation.
 * The verify mutation is therefore NOT expected to send `turnstileToken` on the
 * wire. We intentionally do NOT assert a `turnstileToken` field on the verify
 * request body — doing so would be a false test (unused type artifact on the
 * verify path).
 *
 * Resend countdown: FE-D01 inlined the `expiresInSeconds` → countdown inside
 * the component with `setInterval`/`useEffect` (no extracted pure helper). The
 * countdown behavior is already covered by FE-D01's component test
 * ("advances to the code step and shows the resend countdown after a successful
 * send"), so no timer-based logic test is authored here — per FE-T01 step 1.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { http, HttpResponse } from 'msw'
import { render, waitFor } from '@testing-library/react'
import { act } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { server } from '@/test/mocks/server'
import {
  useEmailOtpSendMutation,
  useEmailOtpVerifyMutation,
  type EmailOtpSendResult,
} from '../email-otp-mutations'
import type { BrowserTokenResponse, LegalAgreementSummary } from '@/lib/api-generated'

const API_BASE_URL = 'http://localhost:3000'
const SEND_URL = `${API_BASE_URL}/api/auth/:realmId/login/email-otp/send`
const VERIFY_URL = `${API_BASE_URL}/api/auth/:realmId/login/email-otp/verify`

// --- Test harness ----------------------------------------------------------
//
// Tiny probe components that expose the mutation hooks via a button click, so
// the tests can drive the hook directly without re-rendering the full form
// (which would pull in Turnstile / OTP-input / router) and keep the focus on
// the mutation logic.

interface SendProbePayload {
  email: string
  clientId: string
  turnstileToken?: string
  agreements?: Array<{ agreementType: string; versionId: string }>
}

function SendProbe({
  realmId,
  payload,
  onSuccess,
  onError,
}: {
  realmId: string
  payload: SendProbePayload
  onSuccess?: (result: EmailOtpSendResult) => void
  onError?: (error: unknown) => void
}) {
  const mutation = useEmailOtpSendMutation({ realmId, onSuccess, onError })
  return <button data-testid="send-probe" onClick={() => mutation.mutate(payload)} />
}

interface VerifyProbePayload {
  email: string
  code: string
  clientId: string
  agreements?: Array<{ agreementType: string; versionId: string }>
}

function VerifyProbe({
  realmId,
  payload,
  onSuccess,
  onError,
}: {
  realmId: string
  payload: VerifyProbePayload
  onSuccess?: (token: BrowserTokenResponse) => void
  onError?: (error: unknown) => void
}) {
  const mutation = useEmailOtpVerifyMutation({ realmId, onSuccess, onError })
  return <button data-testid="verify-probe" onClick={() => mutation.mutate(payload)} />
}

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  })
}

function renderProbe(ui: React.ReactNode) {
  const queryClient = createTestQueryClient()
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>)
}

function clickProbe(testId: string) {
  return act(async () => {
    // Multiple probe components may be mounted across sequential sub-test
    // renders; click the LAST mounted one so each probe drives its own mutation.
    const nodes = document.querySelectorAll<HTMLButtonElement>(`[data-testid="${testId}"]`)
    nodes[nodes.length - 1]!.click()
  })
}

// --- Factories -------------------------------------------------------------

function makeSendResponse(overrides?: { expiresInSeconds?: number; message?: string }) {
  return {
    message: overrides?.message ?? 'Verification code sent',
    expiresInSeconds: overrides?.expiresInSeconds ?? 300,
  }
}

function makeAgreement(
  agreementType: string,
  versionId: string,
  versionNo: number
): LegalAgreementSummary {
  return {
    agreement_type: agreementType,
    version_id: versionId,
    version_no: versionNo,
    effective_at: '2026-06-30T00:00:00Z',
    title: null,
    summary: null,
  }
}

function makeBrowserTokenResponse(overrides?: Partial<BrowserTokenResponse>): BrowserTokenResponse {
  return {
    accessToken: 'at-abc',
    expiresIn: 3600,
    refreshExpiresIn: 86400,
    refreshToken: 'rt-def',
    tokenType: 'Bearer',
    ...overrides,
  }
}

// --- Setup -----------------------------------------------------------------

beforeEach(() => {
  server.resetHandlers()
})

afterEach(() => {
  vi.clearAllMocks()
})

// ===========================================================================
// Send mutation — payload composition + expiresInSeconds surfacing
// ===========================================================================

describe('useEmailOtpSendMutation — payload composition', () => {
  it('posts the composed request body (clientId, email, turnstileToken, agreements) to the send endpoint', async () => {
    let capturedBody: Record<string, unknown> | undefined
    server.use(
      http.post(SEND_URL, async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>
        return HttpResponse.json(makeSendResponse())
      })
    )

    renderProbe(
      <SendProbe
        realmId="realm-1"
        payload={{
          email: 'user@example.com',
          clientId: 'admin-web-console',
          turnstileToken: 'cf-token-xyz',
          agreements: [{ agreementType: 'terms_of_service', versionId: 'tos-v2' }],
        }}
      />
    )

    await clickProbe('send-probe')

    await waitFor(() => {
      expect(capturedBody).toBeDefined()
    })

    // The body must contain the contract fields, composed from the hook payload.
    expect(capturedBody).toMatchObject({
      clientId: 'admin-web-console',
      email: 'user@example.com',
      turnstileToken: 'cf-token-xyz',
      agreements: [{ agreementType: 'terms_of_service', versionId: 'tos-v2' }],
    })
  })

  it('omits turnstileToken/agreements when not provided (no empty/null keys on the wire)', async () => {
    let capturedBody: Record<string, unknown> | undefined
    server.use(
      http.post(SEND_URL, async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>
        return HttpResponse.json(makeSendResponse())
      })
    )

    renderProbe(
      <SendProbe
        realmId="realm-1"
        payload={{ email: 'user@example.com', clientId: 'admin-web-console' }}
      />
    )

    await clickProbe('send-probe')

    await waitFor(() => {
      expect(capturedBody).toBeDefined()
    })

    expect(capturedBody).toMatchObject({
      clientId: 'admin-web-console',
      email: 'user@example.com',
    })
    expect(capturedBody).not.toHaveProperty('turnstileToken')
    expect(capturedBody).not.toHaveProperty('agreements')
  })

  it('surfaces the 200 body (incl. expiresInSeconds) via onSuccess so the form can seed the resend countdown', async () => {
    server.use(
      http.post(SEND_URL, () => HttpResponse.json(makeSendResponse({ expiresInSeconds: 300 })))
    )

    const onResult = vi.fn<(result: EmailOtpSendResult) => void>()
    renderProbe(
      <SendProbe
        realmId="realm-1"
        payload={{ email: 'user@example.com', clientId: 'admin-web-console' }}
        onSuccess={onResult}
      />
    )

    await clickProbe('send-probe')

    await waitFor(() => {
      expect(onResult).toHaveBeenCalledTimes(1)
    })

    const result = onResult.mock.calls[0][0]
    // Success: data populated, no conflict.
    expect(result.conflict).toBeNull()
    expect(result.data).not.toBeNull()
    expect(result.data?.expiresInSeconds).toBe(300)
    expect(result.data?.message).toBe('Verification code sent')
  })
})

// ===========================================================================
// Send mutation — 409 conflict classification (design §4.2 error matrix)
// ===========================================================================

describe('useEmailOtpSendMutation — 409 conflict classification', () => {
  // The two conflict `code` values (backend `email_otp.rs`). Each must surface
  // a distinguishable `EmailOtpSendResult.conflict` payload so the form can
  // branch: consent gate vs not-registered guidance.
  it.each([
    {
      code: 'consent_required',
      label: 'consent_required exposes consentRequired=true + agreements list',
      body: {
        code: 'consent_required',
        consentRequired: true,
        agreements: [
          makeAgreement('terms_of_service', 'tos-v2', 2),
          makeAgreement('privacy_policy', 'privacy-v3', 3),
        ],
        message: 'consent required',
      },
      expectConsent: true,
    },
    {
      code: 'email_not_registered',
      label: 'email_not_registered exposes guidance message and no agreements',
      body: {
        code: 'email_not_registered',
        message: 'Please register first.',
      },
      expectConsent: false,
    },
  ] as const)('$label', async ({ body, expectConsent }) => {
    server.use(http.post(SEND_URL, () => HttpResponse.json(body, { status: 409 })))

    const onResult = vi.fn<(result: EmailOtpSendResult) => void>()
    renderProbe(
      <SendProbe
        realmId="realm-1"
        payload={{ email: 'user@example.com', clientId: 'admin-web-console' }}
        onSuccess={onResult}
      />
    )

    await clickProbe('send-probe')

    await waitFor(() => {
      expect(onResult).toHaveBeenCalledTimes(1)
    })

    const result = onResult.mock.calls[0][0]
    // 409 is NOT thrown — it is surfaced as a non-null conflict (no data).
    expect(result.data).toBeNull()
    expect(result.conflict).not.toBeNull()
    expect(result.conflict?.code).toBe(body.code)

    if (expectConsent) {
      // consent_required: the form renders `agreements` + re-sends on agree.
      expect(result.conflict?.consentRequired).toBe(true)
      expect(result.conflict?.agreements).toEqual(body.agreements ?? null)
    } else {
      // email_not_registered: no agreements; the form shows the guidance message.
      expect(result.conflict?.consentRequired).not.toBe(true)
      expect(result.conflict?.agreements).toBeNull()
      expect(result.conflict?.message).toBe('Please register first.')
    }
  })

  it('the two 409 branches yield distinguishable surfaced payloads', async () => {
    // Regression for design §4.2: the two conflict codes must not collapse to
    // the same shape — the form relies on `consentRequired`/`agreements` being
    // present ONLY for the consent branch.
    const consentBody = {
      code: 'consent_required',
      consentRequired: true,
      agreements: [makeAgreement('terms_of_service', 'tos-v2', 2)],
      message: 'consent required',
    }
    const notRegisteredBody = {
      code: 'email_not_registered',
      message: 'Please register first.',
    }

    // First send: consent_required.
    const consentResult = await runSendOnce(consentBody, 409)
    // Second send (new MSW handler + new probe): email_not_registered.
    const notRegisteredResult = await runSendOnce(notRegisteredBody, 409)

    expect(consentResult.conflict?.code).toBe('consent_required')
    expect(notRegisteredResult.conflict?.code).toBe('email_not_registered')
    // The branches are distinguishable on every field the form branches on.
    expect(consentResult.conflict?.code).not.toBe(notRegisteredResult.conflict?.code)
    expect(Boolean(consentResult.conflict?.agreements?.length)).toBe(true)
    expect(Boolean(notRegisteredResult.conflict?.agreements?.length)).toBe(false)
    expect(consentResult.conflict?.consentRequired).toBe(true)
    expect(notRegisteredResult.conflict?.consentRequired).not.toBe(true)
  })

  async function runSendOnce(
    responseBody: Record<string, unknown>,
    status: number
  ): Promise<EmailOtpSendResult> {
    server.use(http.post(SEND_URL, () => HttpResponse.json(responseBody, { status })))
    const onResult = vi.fn<(result: EmailOtpSendResult) => void>()
    renderProbe(
      <SendProbe
        realmId="realm-1"
        payload={{ email: 'user@example.com', clientId: 'admin-web-console' }}
        onSuccess={onResult}
      />
    )
    await clickProbe('send-probe')
    await waitFor(() => {
      expect(onResult).toHaveBeenCalledTimes(1)
    })
    return onResult.mock.calls[0][0]
  }
})

// ===========================================================================
// Send mutation — non-conflict errors still throw (flow to onError)
// ===========================================================================

describe('useEmailOtpSendMutation — non-conflict errors throw', () => {
  it('surfaces a 429 rate-limit via onError (not as a conflict)', async () => {
    server.use(
      http.post(SEND_URL, () =>
        HttpResponse.json({ message: 'Too many requests' }, { status: 429 })
      )
    )

    const onError = vi.fn<(error: unknown) => void>()
    renderProbe(
      <SendProbe
        realmId="realm-1"
        payload={{ email: 'u@e.com', clientId: 'admin-web-console' }}
        onError={onError}
      />
    )

    await clickProbe('send-probe')

    await waitFor(() => {
      expect(onError).toHaveBeenCalledTimes(1)
    })
    // The rate-limit body arrives on the thrown error — the form renders the
    // error region, NOT a conflict.
    expect(onError.mock.calls[0][0]).toMatchObject({ message: 'Too many requests' })
  })

  it('surfaces an unrecognized 409 body (missing code/message) via onError, not as a conflict', async () => {
    // A 409 whose body is not one of the two known conflict shapes must NOT be
    // misclassified as a conflict — it flows to onError like any other error.
    server.use(
      http.post(SEND_URL, () =>
        HttpResponse.json({ unrelated: 'server-side-state' }, { status: 409 })
      )
    )

    const onError = vi.fn<(error: unknown) => void>()
    renderProbe(
      <SendProbe
        realmId="realm-1"
        payload={{ email: 'u@e.com', clientId: 'admin-web-console' }}
        onError={onError}
      />
    )

    await clickProbe('send-probe')

    await waitFor(() => {
      expect(onError).toHaveBeenCalledTimes(1)
    })
    expect(onError.mock.calls[0][0]).toMatchObject({ unrelated: 'server-side-state' })
  })
})

// ===========================================================================
// Verify mutation — payload composition + BrowserTokenResponse handoff
// ===========================================================================

describe('useEmailOtpVerifyMutation — payload composition + success handoff', () => {
  it('posts the composed verify body (clientId, email, code, agreements) to the verify endpoint', async () => {
    let capturedBody: Record<string, unknown> | undefined
    server.use(
      http.post(VERIFY_URL, async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>
        return HttpResponse.json(makeBrowserTokenResponse())
      })
    )

    renderProbe(
      <VerifyProbe
        realmId="realm-1"
        payload={{
          email: 'user@example.com',
          code: '123456',
          clientId: 'admin-web-console',
          agreements: [{ agreementType: 'terms_of_service', versionId: 'tos-v2' }],
        }}
      />
    )

    await clickProbe('verify-probe')

    await waitFor(() => {
      expect(capturedBody).toBeDefined()
    })

    expect(capturedBody).toMatchObject({
      clientId: 'admin-web-console',
      email: 'user@example.com',
      code: '123456',
      agreements: [{ agreementType: 'terms_of_service', versionId: 'tos-v2' }],
    })

    // NOTE: `turnstileToken` is intentionally NOT asserted here. The
    // EmailOtpVerifyRequest type carries an optional turnstileToken?, but
    // FE-D01 forwards turnstileToken only into the SEND mutation and design
    // §4.2.2's verify table does not list it. Asserting its presence would be
    // a false test against an unused type artifact.
    expect(capturedBody).not.toHaveProperty('turnstileToken')
  })

  it('hands the raw BrowserTokenResponse up via onSuccess on 200 (route owns the session handoff)', async () => {
    const tokenResponse = makeBrowserTokenResponse({
      accessToken: 'at-verify-success',
      refreshToken: 'rt-verify-success',
    })
    server.use(http.post(VERIFY_URL, () => HttpResponse.json(tokenResponse)))

    const onToken = vi.fn<(token: BrowserTokenResponse) => void>()
    renderProbe(
      <VerifyProbe
        realmId="realm-1"
        payload={{ email: 'user@example.com', code: '654321', clientId: 'admin-web-console' }}
        onSuccess={onToken}
      />
    )

    await clickProbe('verify-probe')

    await waitFor(() => {
      expect(onToken).toHaveBeenCalledTimes(1)
    })
    // The route owns completeLoginAfterEmailOtp + navigation; the mutation must
    // hand up the RAW BrowserTokenResponse unchanged.
    expect(onToken).toHaveBeenCalledWith(tokenResponse)
  })

  it('surfaces a 401 (wrong/expired/exhausted) via onError so the form can retry vs resend', async () => {
    server.use(
      http.post(VERIFY_URL, () =>
        HttpResponse.json(
          { code: 'invalid_code', message: 'Invalid or expired code.' },
          { status: 401 }
        )
      )
    )

    const onError = vi.fn<(error: unknown) => void>()
    renderProbe(
      <VerifyProbe
        realmId="realm-1"
        payload={{ email: 'u@e.com', code: '000000', clientId: 'admin-web-console' }}
        onError={onError}
      />
    )

    await clickProbe('verify-probe')

    await waitFor(() => {
      expect(onError).toHaveBeenCalledTimes(1)
    })
    // The verify 401 body arrives on the thrown error — the form renders the
    // error region with the backend's authoritative message (retry vs resend).
    expect(onError.mock.calls[0][0]).toMatchObject({ message: 'Invalid or expired code.' })
  })
})
