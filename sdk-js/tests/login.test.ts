import { describe, it, expect, afterEach } from 'vitest'
import { http, HttpResponse } from 'msw'
import { server } from './mocks/server'
import { performPasskeyAssertion } from '../src'
import { fakeCredential, makeClient, makeTokens, urls } from './helpers'

describe('login (US-JS-004 / DEC-010)', () => {
  it('password_login_success_returns_session', async () => {
    const tokens = makeTokens({ accessToken: 'at-ok', refreshToken: 'rt-ok' })
    server.use(http.post(urls.login, () => HttpResponse.json(tokens)))

    const { client, events } = makeClient()
    const result = await client.login({ email: 'a@b.c', password: 'pw' })

    expect(result.kind).toBe('success')
    if (result.kind !== 'success') return
    expect(result.session.authenticated).toBe(true)
    expect(result.session.credentialClass).toBe('custom_user_ui')
    // Success stores the rotated token set.
    expect(client.storage.getRefreshToken()).toBe('rt-ok')
    expect(events.some((e) => e.type === 'authenticated')).toBe(true)
  })

  it('requires_second_factor_only_totp_passkey', async () => {
    server.use(
      http.post(
        urls.login,
        () =>
          HttpResponse.json({
            // The backend only ever returns totp/passkey; unknown values are
            // filtered out by the SDK.
            secondFactors: ['totp', 'passkey', 'sms'],
            tempToken: 'tt-1',
            expiresInSeconds: 300,
            userId: 'u-1',
            realmId: 'realm-1',
            message: '2fa',
          }),
        { once: true },
      ),
    )

    const { client } = makeClient()
    const result = await client.login({ email: 'a@b.c', password: 'pw' })
    expect(result.kind).toBe('requires-second-factor')
    if (result.kind !== 'requires-second-factor') return
    expect(result.secondFactors).toEqual(['totp', 'passkey'])
    expect(result.tempToken).toBe('tt-1')
    expect(result.expiresInSeconds).toBe(300)
    // No tokens are issued at the challenge stage.
    expect(client.storage.getRefreshToken()).toBeNull()
  })

  it('consent_required_returns_agreements', async () => {
    server.use(
      http.post(
        urls.login,
        () =>
          HttpResponse.json({
            consentRequired: true,
            agreements: [
              {
                agreement_type: 'terms_of_service',
                version_id: 'v1',
                title: 'Terms',
                version_no: 1,
                effective_at: '2026-01-01',
                mode: 'active',
              },
            ],
          }),
        { once: true },
      ),
    )

    const { client } = makeClient()
    const result = await client.login({ email: 'a@b.c', password: 'pw' })
    expect(result.kind).toBe('consent-required')
    if (result.kind !== 'consent-required') return
    // snake_case summaries normalized to the camelCase re-submit shape.
    expect(result.agreements).toEqual([{ agreementType: 'terms_of_service', versionId: 'v1' }])
  })

  it('oauth_redirect_branch', async () => {
    server.use(
      http.post(urls.login, () => HttpResponse.json({ redirectTo: 'https://app/callback' }), {
        once: true,
      }),
    )
    const { client } = makeClient()
    const result = await client.login({ email: 'a@b.c', password: 'pw' })
    expect(result.kind).toBe('oauth-redirect')
    if (result.kind !== 'oauth-redirect') return
    expect(result.redirectTo).toBe('https://app/callback')
  })

  it('verify_totp_success_returns_browser_token_response', async () => {
    server.use(
      http.post(urls.verifyTotp, () => HttpResponse.json(makeTokens({ accessToken: 'at-2' }))),
    )
    const { client } = makeClient()
    const result = await client.verifyTotp({ tempToken: 'tt-1', code: '123456' })
    expect(result.kind).toBe('success')
    expect(client.storage.getRefreshToken()).toBe('rt-1')
  })

  describe('passkey login', () => {
    const originalCredentials = (navigator as unknown as { credentials?: unknown }).credentials

    afterEach(() => {
      Object.defineProperty(navigator, 'credentials', {
        value: originalCredentials,
        configurable: true,
      })
    })

    it('passkey_login_begin_finish', async () => {
      Object.defineProperty(navigator, 'credentials', {
        value: { get: async () => fakeCredential() },
        configurable: true,
      })

      server.use(
        http.post(urls.passkeyOptions, () =>
          HttpResponse.json({ authToken: 'pk-auth', options: { challenge: 'Y2hhbGxlbmdl', rpId: 'example.com' } }),
        ),
        http.post(urls.passkeyVerify, () => HttpResponse.json(makeTokens({ accessToken: 'at-pk' }))),
      )

      const { client } = makeClient()
      const begin = await client.passkey.loginBegin({})
      expect(begin.authToken).toBe('pk-auth')
      expect((begin.options as { challenge: string }).challenge).toBe('Y2hhbGxlbmdl')

      const assertion = await performPasskeyAssertion(
        begin.options as { challenge: string },
      )
      expect(assertion.type).toBe('public-key')
      expect(assertion.response.signature).toBeTruthy()

      const result = await client.passkey.loginFinish({ authToken: begin.authToken, assertion })
      expect(result.kind).toBe('success')
      expect(client.storage.getRefreshToken()).toBe('rt-1')
    })
  })

  it('email_otp_is_passwordless_two_step', async () => {
    server.use(
      http.post(urls.emailOtpSend, () =>
        HttpResponse.json({ message: 'code sent', expiresInSeconds: 300 }),
      ),
      http.post(urls.emailOtpVerify, () => HttpResponse.json(makeTokens({ accessToken: 'at-otp' }))),
    )

    const { client } = makeClient()
    const send = await client.loginWithEmailOtp.send({ email: 'a@b.c' })
    expect(send).toEqual({ message: 'code sent', expiresInSeconds: 300 })

    const result = await client.loginWithEmailOtp.verify({ email: 'a@b.c', code: '123456' })
    expect(result.kind).toBe('success')
    expect(client.storage.getRefreshToken()).toBe('rt-1')
  })

  it('login_failure_401_returns_unauthorized', async () => {
    server.use(
      http.post(urls.login, () =>
        HttpResponse.json({ status: 401, code: 'invalid_credentials', message: 'no' }, { status: 401 }),
      ),
    )
    const { client } = makeClient()
    await expect(client.login({ email: 'a@b.c', password: 'bad' })).rejects.toMatchObject({
      kind: 'unauthorized',
    })
  })

  it('rate_limited_429', async () => {
    server.use(
      http.post(urls.login, () =>
        HttpResponse.json({ status: 429, code: 'rate_limited', message: 'slow' }, { status: 429 }),
      ),
    )
    const { client } = makeClient()
    await expect(client.login({ email: 'a@b.c', password: 'pw' })).rejects.toMatchObject({
      kind: 'rate-limited',
    })
  })
})
