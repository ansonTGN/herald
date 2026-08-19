/**
 * Auth store token-model persistence / cleanup boundaries.
 *
 * Uses the REAL `useAuthStore` (no mocks) to assert the Bearer family's
 * persistence contract:
 *   - the refresh token + PKCE state are persisted (survive a reload)
 *   - the access token is NEVER persisted (in-memory holder only)
 *   - `logout()` / `reset()` / `clearAuthStorage()` all purge RT + PKCE + AT
 *   - the storage key is the existing `AUTH_STORAGE_KEY` (no new keys)
 */

import { describe, it, expect, beforeEach } from 'vitest'
import {
  useAuthStore,
  accessTokenHolder,
  clearAuthStorage,
  type PersistedPkceState,
} from '@/stores/auth-store'
import { AUTH_STORAGE_KEY } from '@/lib/constants/auth-constants'
import { TOKEN_FIXTURE } from '@/test/fixtures/browser-token'

/** Read the persisted snapshot written to localStorage by the persist middleware. */
function readPersistedSnapshot(): Record<string, unknown> {
  const raw = window.localStorage.getItem(AUTH_STORAGE_KEY)
  if (!raw) return {}
  try {
    const parsed = JSON.parse(raw)
    // Zustand persist wraps the state in `{ state, version }`.
    return parsed && parsed.state ? parsed.state : (parsed ?? {})
  } catch {
    return {}
  }
}

const SAMPLE_PKCE: PersistedPkceState = {
  codeVerifier: 'verifier-abc',
  clientId: TOKEN_FIXTURE.clientId,
  redirectUri: 'http://localhost/callback',
  state: 'state-xyz',
}

beforeEach(() => {
  // Reset both the in-memory holder and the persisted store between tests.
  accessTokenHolder.clear()
  useAuthStore.getState().reset()
  window.localStorage.removeItem(AUTH_STORAGE_KEY)
})

describe('partialize: refresh token + PKCE state persisted, access token excluded', () => {
  it('persists refreshToken + refreshClientId + pkceState to localStorage', () => {
    useAuthStore.getState().setTokens({
      accessToken: TOKEN_FIXTURE.accessToken,
      refreshToken: TOKEN_FIXTURE.refreshToken,
      clientId: TOKEN_FIXTURE.clientId,
    })
    useAuthStore.getState().setPkceState(SAMPLE_PKCE)

    const snapshot = readPersistedSnapshot()

    expect(snapshot.refreshToken).toBe(TOKEN_FIXTURE.refreshToken)
    expect(snapshot.refreshClientId).toBe(TOKEN_FIXTURE.clientId)
    expect(snapshot.pkceState).toMatchObject(SAMPLE_PKCE)
  })

  it('does NOT persist the in-memory access token (no AT field in the snapshot)', () => {
    useAuthStore.getState().setTokens({
      accessToken: TOKEN_FIXTURE.accessToken,
      refreshToken: TOKEN_FIXTURE.refreshToken,
      clientId: TOKEN_FIXTURE.clientId,
    })

    const snapshot = readPersistedSnapshot()

    // The AT lives only in the in-memory holder; the persisted snapshot must
    // not carry it under any common key name.
    expect(snapshot).not.toHaveProperty('accessToken')
    expect(snapshot).not.toHaveProperty('access_token')
    // The holder DOES hold it in memory.
    expect(accessTokenHolder.get()).toBe(TOKEN_FIXTURE.accessToken)
  })
})

describe('access token is non-persistent across a simulated full reload', () => {
  it('a fresh module holder starts empty even when a refresh token was persisted', () => {
    // Seed a full session.
    useAuthStore.getState().setTokens({
      accessToken: TOKEN_FIXTURE.accessToken,
      refreshToken: TOKEN_FIXTURE.refreshToken,
      clientId: TOKEN_FIXTURE.clientId,
    })
    expect(accessTokenHolder.get()).toBe(TOKEN_FIXTURE.accessToken)

    // Simulate a full page reload: the in-memory holder is wiped, while the
    // persisted RT/clientId survive in localStorage.
    accessTokenHolder.clear()
    const persisted = readPersistedSnapshot()
    expect(persisted.refreshToken).toBe(TOKEN_FIXTURE.refreshToken)

    // After "reload", no access token is restored from storage — the app must
    // call the refresh endpoint to obtain a new AT (refresh-first restore).
    expect(accessTokenHolder.get()).toBeNull()
  })
})

describe('logout() / reset() / clearAuthStorage() all purge RT + PKCE + AT', () => {
  beforeEach(() => {
    useAuthStore.getState().setTokens({
      accessToken: TOKEN_FIXTURE.accessToken,
      refreshToken: TOKEN_FIXTURE.refreshToken,
      clientId: TOKEN_FIXTURE.clientId,
    })
    useAuthStore.getState().setPkceState(SAMPLE_PKCE)
    useAuthStore.getState().setAuthStatus(true, 'realm-1')
  })

  it('logout() clears the refresh token, PKCE state, and in-memory access token', () => {
    expect(accessTokenHolder.get()).toBe(TOKEN_FIXTURE.accessToken)

    useAuthStore.getState().logout()

    expect(useAuthStore.getState().refreshToken).toBeNull()
    expect(useAuthStore.getState().refreshClientId).toBeNull()
    expect(useAuthStore.getState().pkceState).toBeNull()
    expect(accessTokenHolder.get()).toBeNull()
    expect(useAuthStore.getState().isAuthenticated).toBe(false)
  })

  it('reset() clears the refresh token, PKCE state, and in-memory access token', () => {
    useAuthStore.getState().reset()

    expect(useAuthStore.getState().refreshToken).toBeNull()
    expect(useAuthStore.getState().refreshClientId).toBeNull()
    expect(useAuthStore.getState().pkceState).toBeNull()
    expect(accessTokenHolder.get()).toBeNull()
  })

  it('clearAuthStorage() wipes both the in-memory holder and the persisted localStorage (no RT/PKCE residue)', () => {
    clearAuthStorage()

    // In-memory AT cleared.
    expect(accessTokenHolder.get()).toBeNull()
    // Persisted snapshot in localStorage has no RT/PKCE residue.
    const snapshot = readPersistedSnapshot()
    expect(snapshot.refreshToken).toBeUndefined()
    expect(snapshot.pkceState).toBeUndefined()
  })
})

describe('storage key: uses the shared AUTH_STORAGE_KEY (no new keys introduced)', () => {
  it('the persist middleware writes under AUTH_STORAGE_KEY', () => {
    useAuthStore.getState().setTokens({
      accessToken: TOKEN_FIXTURE.accessToken,
      refreshToken: TOKEN_FIXTURE.refreshToken,
      clientId: TOKEN_FIXTURE.clientId,
    })

    expect(window.localStorage.getItem(AUTH_STORAGE_KEY)).not.toBeNull()
    // And the key name is exactly the shared constant.
    expect(AUTH_STORAGE_KEY).toBe('auth-storage')
  })
})

describe('PKCE state round-trip (getPkceState reads what setPkceState wrote)', () => {
  it('persists and reads back the PKCE verifier + bound OAuth params', () => {
    useAuthStore.getState().setPkceState(SAMPLE_PKCE)
    expect(useAuthStore.getState().getPkceState()).toMatchObject(SAMPLE_PKCE)

    // Clearing returns null.
    useAuthStore.getState().setPkceState(null)
    expect(useAuthStore.getState().getPkceState()).toBeNull()
  })
})

describe('getRefreshToken round-trip', () => {
  it('returns the stored refresh token + bound clientId', () => {
    useAuthStore.getState().setTokens({
      accessToken: TOKEN_FIXTURE.accessToken,
      refreshToken: TOKEN_FIXTURE.refreshToken,
      clientId: TOKEN_FIXTURE.clientId,
    })
    expect(useAuthStore.getState().getRefreshToken()).toEqual({
      refreshToken: TOKEN_FIXTURE.refreshToken,
      clientId: TOKEN_FIXTURE.clientId,
    })
  })
})
