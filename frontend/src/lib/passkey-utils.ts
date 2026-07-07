/**
 * Pure WebAuthn / passkey helpers.
 *
 * No React dependency — independently unit-testable. These functions bridge the
 * gap between the backend's JSON-serialised WebAuthn options (base64url strings)
 * and the browser's `navigator.credentials` API (ArrayBuffer fields), and back
 * again when serialising the resulting credential for the server.
 *
 * The backend endpoints serialise WebAuthn challenge / credential objects to
 * camelCase JSON (matching the W3C WebAuthn JSON encoding that `webauthn-rs`
 * uses), with binary fields encoded as **unpadded** base64url strings.
 *
 * NOTE: request timeout wrapping is NOT redefined here. Call sites that need a
 * timeout around a generated API call should import `withTimeout` from
 * `@/lib/totp-utils` (see design §5.2 — passkey-utils reuses the shared helper).
 */

/**
 * SSR-safe detection of WebAuthn / `navigator.credentials` support.
 *
 * Returns `true` only when running in a browser that exposes
 * `window.PublicKeyCredential`. During SSR / non-browser contexts it returns
 * `false` so callers can hide passkey entry points without crashing.
 */
export function isWebAuthnSupported(): boolean {
  return typeof window !== 'undefined' && 'PublicKeyCredential' in window
}

/**
 * Decode an **unpadded** base64url string into an `ArrayBuffer`.
 *
 * WebAuthn challenge / id fields are transmitted as base64url without `=`
 * padding; this restores the padding before delegating to `atob`.
 */
export function base64urlToBuffer(value: string): ArrayBuffer {
  const base64 = value.replace(/-/g, '+').replace(/_/g, '/')
  const padded = base64.length % 4 === 0 ? base64 : base64 + '='.repeat(4 - (base64.length % 4))
  const binary = atob(padded)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i)
  }
  return bytes.buffer
}

/**
 * Encode an `ArrayBuffer` (or `Uint8Array` view) as an **unpadded** base64url
 * string suitable for the backend's WebAuthn request DTOs.
 */
export function bufferToBase64url(value: ArrayBuffer | Uint8Array): string {
  const bytes = value instanceof Uint8Array ? value : new Uint8Array(value)
  let binary = ''
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i])
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

/**
 * Narrow an unknown server options object to its inner `publicKey` payload.
 *
 * `webauthn-rs` returns the `PublicKeyCredentialCreationOptionsJSON` /
 * `PublicKeyCredentialRequestOptionsJSON` directly, but some flows wrap it under
 * a `publicKey` key. This normalises both shapes to the inner object.
 */
function unwrapPublicKey(serverOptions: unknown): Record<string, unknown> {
  if (
    serverOptions !== null &&
    typeof serverOptions === 'object' &&
    'publicKey' in serverOptions &&
    typeof (serverOptions as Record<string, unknown>).publicKey === 'object'
  ) {
    return (serverOptions as Record<string, unknown>).publicKey as Record<string, unknown>
  }
  return (serverOptions ?? {}) as Record<string, unknown>
}

/** Structured clone of a plain-JSON value (the server options are JSON only). */
function cloneJSON<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

/**
 * Convert the backend's registration options (`BeginRegistrationResponse.options`)
 * into the argument shape expected by `navigator.credentials.create`.
 *
 * Deep-clones the input (never mutates the caller's object) and decodes the
 * base64url fields (`challenge`, `user.id`, `excludeCredentials[].id`) into
 * `ArrayBuffer` values the browser requires.
 */
export function prepareCreationOptions(serverOptions: unknown): CredentialCreationOptions {
  const source = cloneJSON(unwrapPublicKey(serverOptions))
  const publicKey = source as Record<string, unknown>

  if (typeof publicKey.challenge === 'string') {
    publicKey.challenge = base64urlToBuffer(publicKey.challenge)
  }

  const user = publicKey.user as Record<string, unknown> | undefined
  if (user && typeof user.id === 'string') {
    user.id = base64urlToBuffer(user.id)
  }

  if (Array.isArray(publicKey.excludeCredentials)) {
    for (const cred of publicKey.excludeCredentials as Array<Record<string, unknown>>) {
      if (cred && typeof cred.id === 'string') {
        cred.id = base64urlToBuffer(cred.id)
      }
    }
  }

  return { publicKey: publicKey as unknown as PublicKeyCredentialCreationOptions }
}

/**
 * Convert the backend's authentication options (`PasskeyOptionsResponse.options`
 * / `Passkey2FaOptionsResponse.options`) into the argument shape expected by
 * `navigator.credentials.get`.
 *
 * Deep-clones the input (never mutates the caller's object) and decodes the
 * base64url fields (`challenge`, `allowCredentials[].id`) into `ArrayBuffer`.
 * `mediation` is forwarded verbatim when provided (e.g. `'conditional'` for
 * usernameless autofill UI, `'optional'` for an explicit button press).
 */
export function prepareRequestOptions(
  serverOptions: unknown,
  mediation?: 'conditional' | 'optional'
): CredentialRequestOptions {
  const source = cloneJSON(unwrapPublicKey(serverOptions))
  const publicKey = source as Record<string, unknown>

  if (typeof publicKey.challenge === 'string') {
    publicKey.challenge = base64urlToBuffer(publicKey.challenge)
  }

  if (Array.isArray(publicKey.allowCredentials)) {
    for (const cred of publicKey.allowCredentials as Array<Record<string, unknown>>) {
      if (cred && typeof cred.id === 'string') {
        cred.id = base64urlToBuffer(cred.id)
      }
    }
  }

  const options: CredentialRequestOptions = {
    publicKey: publicKey as unknown as PublicKeyCredentialRequestOptions,
  }
  if (mediation) {
    options.mediation = mediation
  }
  return options
}

/**
 * Serialise the result of `navigator.credentials.create` into the JSON shape
 * expected by `FinishRegistrationRequest.attestation` (camelCase, base64url
 * binary fields — matches `webauthn-rs` proto wire format).
 *
 * Includes `response.transports` when the browser exposes
 * `getTransports()` so the server can persist transport hints for future
 * conditional-UI prompts.
 */
export function serializeAttestation(cred: PublicKeyCredential): unknown {
  const response = cred.response as AuthenticatorAttestationResponse
  const encodedResponse: Record<string, unknown> = {
    clientDataJSON: bufferToBase64url(response.clientDataJSON),
    attestationObject: bufferToBase64url(response.attestationObject),
  }

  if (typeof response.getTransports === 'function') {
    const transports = response.getTransports()
    if (Array.isArray(transports)) {
      encodedResponse.transports = transports
    }
  }

  return {
    id: cred.id,
    rawId: bufferToBase64url(cred.rawId),
    type: cred.type,
    response: encodedResponse,
  }
}

/**
 * Serialise the result of `navigator.credentials.get` into the JSON shape
 * expected by `PasskeyVerifyRequest.assertion` /
 * `Passkey2FaVerifyRequest.assertion` (camelCase, base64url binary fields —
 * matches `webauthn-rs` proto wire format).
 *
 * `userHandle` is included only when the authenticator returned one.
 */
export function serializeAssertion(cred: PublicKeyCredential): unknown {
  const response = cred.response as AuthenticatorAssertionResponse
  const encodedResponse: Record<string, unknown> = {
    authenticatorData: bufferToBase64url(response.authenticatorData),
    clientDataJSON: bufferToBase64url(response.clientDataJSON),
    signature: bufferToBase64url(response.signature),
  }

  if (response.userHandle) {
    encodedResponse.userHandle = bufferToBase64url(response.userHandle)
  }

  return {
    id: cred.id,
    rawId: bufferToBase64url(cred.rawId),
    type: cred.type,
    response: encodedResponse,
  }
}
