/**
 * Public SDK types (design §5.1 / §5.2 / DEC-js-sdk-010).
 *
 * DTO response types are re-exported from the (internal) generated layer so
 * consumers get a stable, typed surface without depending on generated paths.
 */

import type {
  BrowserTokenResponse,
  CredentialClass,
  CredentialScope,
  StatusResponse,
} from './generated/types.gen'

export type { BrowserTokenResponse, StatusResponse }

export type HeraldCredentialClass = CredentialClass

/** A normalized session view, derived from `/api/auth/status`. */
export interface HeraldSession {
  authenticated: boolean
  realmId: string | null
  userId: string | null
  clientAppId: string | null
  clientId: string | null
  credentialClass: HeraldCredentialClass | null
  permissions: string[]
  scopes: CredentialScope[]
}

export type SessionEvent =
  | { type: 'authenticated'; session: HeraldSession }
  | {
      type: 'session-expired'
      reason: 'refresh-failed' | 'family-revoked' | 'client-app-disabled'
    }
  | { type: 'logged-out' }

/** Second factors the backend may request on `POST /login` (DEC-js-sdk-010). */
export type SecondFactor = 'totp' | 'passkey'

/** Agreement a caller must re-submit (via `agreements`) to pass a consent gate. */
export interface ConsentAgreement {
  agreementType: string
  versionId: string
}

// --- Login result discriminated union (DEC-js-sdk-010) ---

export interface LoginSuccess {
  kind: 'success'
  session: HeraldSession
}

export interface LoginRequiresSecondFactor {
  kind: 'requires-second-factor'
  tempToken: string
  expiresInSeconds: number
  secondFactors: SecondFactor[]
  userId: string
  realmId: string
}

export interface LoginConsentRequired {
  kind: 'consent-required'
  /** Agreements the integrator must render + re-submit via `login`/`verify` `agreements`. */
  agreements: ConsentAgreement[]
}

export interface LoginOauthRedirect {
  kind: 'oauth-redirect'
  redirectTo: string
}

export type LoginResult =
  | LoginSuccess
  | LoginRequiresSecondFactor
  | LoginConsentRequired
  | LoginOauthRedirect

/** Result of `passkey.loginBegin` (1FA or 2FA). `options` is the WebAuthn
 *  `PublicKeyCredentialRequestOptions` JSON returned by the server. */
export interface PasskeyLoginBeginResult {
  authToken: string
  options: unknown
}
