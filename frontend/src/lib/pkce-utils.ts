/**
 * PKCE (Proof Key for Code Exchange, RFC 7636) helpers for the Herald FirstParty
 * login flow (design §4.4 — OAuth Authorization Code + PKCE).
 *
 * The flow:
 *   1. `generatePkcePair()` → random `code_verifier` + S256 `code_challenge`.
 *   2. The verifier is persisted in the auth store; the challenge is sent to
 *      `/api/oauth/{realmId}/authorize` (which seeds Redis state).
 *   3. After login, the backend returns `redirectTo = {redirect_uri}?code=...`.
 *   4. `extractAuthorizationCode()` pulls the `code` off that URL.
 *   5. The code + verifier go to `performPkceTokenExchange` → token set.
 *
 * Uses the Web Crypto API (`crypto.subtle` / `crypto.getRandomValues`), which is
 * available in all evergreen browsers and in Vitest's jsdom + `@happy-dom` /
 * `jsdom` environments where `crypto.subtle` is polyfilled.
 */

/** Unreserved URL-safe alphabet per RFC 7636 §4.1. */
const PKCE_CHARSET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~'

/**
 * Generate a high-entropy PKCE `code_verifier` (43–128 chars, RFC 7636 §4.1).
 * We emit 64 random characters from the unreserved set (≈384 bits of entropy).
 */
export function generateCodeVerifier(size: number = 64): string {
  const randomValues = new Uint32Array(size)
  crypto.getRandomValues(randomValues)
  let verifier = ''
  for (let i = 0; i < size; i++) {
    verifier += PKCE_CHARSET[randomValues[i] % PKCE_CHARSET.length]
  }
  return verifier
}

/**
 * Derive the S256 `code_challenge` = BASE64URL(SHA-256(verifier)) (RFC 7636 §4.2).
 */
export async function computeCodeChallenge(verifier: string): Promise<string> {
  const encoder = new TextEncoder()
  const digest = await crypto.subtle.digest('SHA-256', encoder.encode(verifier))
  return base64UrlEncode(new Uint8Array(digest))
}

/**
 * Generate a complete PKCE pair: a fresh verifier and its S256 challenge.
 */
export async function generatePkcePair(): Promise<{
  codeVerifier: string
  codeChallenge: string
}> {
  const codeVerifier = generateCodeVerifier()
  const codeChallenge = await computeCodeChallenge(codeVerifier)
  return { codeVerifier, codeChallenge }
}

/**
 * Generate a random opaque `state` token for CSRF protection of the OAuth
 * authorize → token round-trip. 32 hex chars (128 bits).
 */
export function generateStateToken(): string {
  const bytes = new Uint8Array(16)
  crypto.getRandomValues(bytes)
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('')
}

/**
 * Extract the OAuth `code` (and `state`) from a login `redirectTo` URL.
 *
 * The backend issues `redirectTo = {redirect_uri}?code={code}&state={state}`.
 * Returns `null` when there is no `code` query parameter.
 */
export function extractAuthorizationCode(redirectTo: string): {
  code: string
  state: string | null
} | null {
  try {
    const url = new URL(
      redirectTo,
      typeof window !== 'undefined' ? window.location.origin : 'http://localhost'
    )
    const code = url.searchParams.get('code')
    if (!code) return null
    return { code, state: url.searchParams.get('state') }
  } catch {
    return null
  }
}

/** BASE64URL-encode (no padding) per RFC 4648 §5. Browser-only (`btoa`). */
function base64UrlEncode(bytes: Uint8Array): string {
  let binary = ''
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i])
  }
  const base64 = btoa(binary)
  return base64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}
