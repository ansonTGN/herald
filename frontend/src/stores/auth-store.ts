/**
 * Authentication Store (Zustand)
 *
 * Centralized state management for authentication and authorization.
 * Uses DevTools integration for debugging and persist middleware for localStorage.
 *
 * Token model (design §4.4 — Bearer access/refresh token family):
 * - The rotating **refresh token** is persisted to localStorage (survives reload)
 *   so the app can restore the session on startup by refreshing.
 * - The short-lived **access token** is held in a module-scoped, non-reactive
 *   in-memory holder (`accessTokenHolder`) and is NEVER persisted. A full page
 *   reload clears it and triggers a refresh-first restore in `initializeAuth`.
 * - A transient **PKCE verifier** and **pending auth state** are persisted so an
 *   in-progress PKCE login that gets interrupted by a 2FA step (or a reload
 *   during the PKCE window) can still complete the token exchange.
 */

import { create } from 'zustand'
import { persist, devtools } from 'zustand/middleware'
import { useShallow } from 'zustand/react/shallow'
import type { UserProfile } from '@/lib/api-generated'
import { AUTH_STORAGE_KEY, AUTH_STORE_NAME } from '@/lib/constants/auth-constants'

/**
 * Shape of the persisted PKCE / pending-auth state. Carried through the
 * FirstParty login → 2FA → `oauthToken` exchange so the verifier survives a
 * reload or a second-factor detour within the PKCE code's TTL.
 */
export interface PersistedPkceState {
  /** The PKCE `code_verifier` used to complete the `oauthToken` exchange. */
  codeVerifier: string
  /** The OAuth `client_id` (e.g. `admin-web-console`) bound to this flow. */
  clientId: string
  /** The pre-registered `redirect_uri` the code was issued for. */
  redirectUri: string
  /** The CSRF `state` token used when seeding the OAuth authorize call. */
  state: string
}

/**
 * Non-reactive in-memory holder for the access token. Deliberately NOT part of
 * the Zustand store so it never triggers re-renders and never gets persisted.
 * `api-client.ts` reads `accessTokenHolder.get()` to inject `Authorization: Bearer`.
 */
let accessToken: string | null = null

export const accessTokenHolder = {
  get(): string | null {
    return accessToken
  },
  set(token: string | null): void {
    accessToken = token
  },
  clear(): void {
    accessToken = null
  },
}

/**
 * Authentication state
 */
export interface AuthState {
  // Authentication status
  isAuthenticated: boolean
  isLoading: boolean
  realmId: string | null

  // User data
  user: UserProfile | null
  permissions: string[]
  roles: string[]

  // Token model (Bearer family)
  /** Rotating refresh token — persisted; cleared only on logout/reset. */
  refreshToken: string | null
  /** The `clientId` the refresh token was issued for (needed to refresh). */
  refreshClientId: string | null
  /** Transient PKCE + pending-auth state — persisted across reloads. */
  pkceState: PersistedPkceState | null
}

/**
 * Authentication actions
 */
export interface AuthActions {
  // Status actions
  setAuthStatus: (authenticated: boolean, realmId?: string) => void
  setIsLoading: (isLoading: boolean) => void

  // User data actions
  setUserPermissions: (permissions: string[], roles: string[]) => void
  setUserProfile: (user: UserProfile | null) => void

  // Auth flow actions
  login: (realmId: string) => void
  logout: () => void

  /**
   * Store a freshly-issued Bearer token set (access + refresh). Access token
   * goes to the in-memory holder; refresh token + clientId are persisted.
   */
  setTokens: (tokens: { accessToken: string; refreshToken: string; clientId?: string }) => void

  /** Persist the PKCE verifier + bound OAuth params for the active flow. */
  setPkceState: (state: PersistedPkceState | null) => void

  /** Read the persisted PKCE state (or null if no flow is in progress). */
  getPkceState: () => PersistedPkceState | null

  /** Read the persisted refresh token + its bound clientId. */
  getRefreshToken: () => { refreshToken: string | null; clientId: string | null }

  // Store actions
  reset: () => void
  clearStorage: () => void
}

/**
 * Initial state
 */
const initialState: AuthState = {
  isAuthenticated: false,
  isLoading: false,
  realmId: null,
  user: null,
  permissions: [],
  roles: [],
  refreshToken: null,
  refreshClientId: null,
  pkceState: null,
}

/**
 * Create the authentication store
 */
export const useAuthStore = create<AuthState & AuthActions>()(
  devtools(
    persist(
      (set, get) => ({
        ...initialState,

        // Status actions
        setAuthStatus: (authenticated, realmId) =>
          set({ isAuthenticated: authenticated, realmId: realmId ?? get().realmId }),

        setIsLoading: (isLoading) => set({ isLoading }),

        // User data actions
        setUserPermissions: (permissions, roles) => set({ permissions, roles }),

        setUserProfile: (user) => set({ user }),

        // Auth flow actions
        login: (realmId) =>
          set({
            isAuthenticated: true,
            realmId,
          }),

        logout: () => {
          accessTokenHolder.clear()
          set({
            isAuthenticated: false,
            isLoading: false,
            user: null,
            permissions: [],
            roles: [],
            refreshToken: null,
            refreshClientId: null,
            pkceState: null,
          })
        },

        setTokens: ({ accessToken: at, refreshToken: rt, clientId }) => {
          accessTokenHolder.set(at)
          set({
            refreshToken: rt,
            ...(clientId ? { refreshClientId: clientId } : {}),
          })
        },

        setPkceState: (pkceState) => set({ pkceState }),

        getPkceState: () => get().pkceState,

        getRefreshToken: () => ({
          refreshToken: get().refreshToken,
          clientId: get().refreshClientId,
        }),

        // Store actions
        reset: () => {
          accessTokenHolder.clear()
          set(initialState)
        },

        clearStorage: () => {
          accessTokenHolder.clear()
          set(initialState)
        },
      }),
      {
        name: AUTH_STORAGE_KEY,
        partialize: (state) => ({
          // Persist UI/auth state, the refresh token + its bound clientId, and
          // the in-flight PKCE state so a reload can restore/complete the flow.
          isAuthenticated: state.isAuthenticated,
          realmId: state.realmId,
          user: state.user,
          permissions: state.permissions,
          roles: state.roles,
          refreshToken: state.refreshToken,
          refreshClientId: state.refreshClientId,
          pkceState: state.pkceState,
          // NOTE: access token is intentionally NOT here — it lives only in the
          // module-scoped in-memory holder and must not survive a reload.
        }),
      }
    ),
    { name: AUTH_STORE_NAME }
  )
)

/**
 * Get the persist storage instance to clear storage
 * This is needed for proper logout that clears both state and storage
 */
const persistStorage = useAuthStore.persist

/**
 * Clear all persisted auth data from storage
 */
export function clearAuthStorage(): void {
  accessTokenHolder.clear()
  persistStorage.clearStorage()
}

/**
 * Selector hooks for optimized re-renders
 */

/**
 * Get authentication status
 */
export const useIsAuthenticated = () => useAuthStore((state) => state.isAuthenticated)

/**
 * Get loading state
 */
export const useIsLoading = () => useAuthStore((state) => state.isLoading)

/**
 * Get user data
 */
export const useUser = () => useAuthStore((state) => state.user)

/**
 * Get permissions
 */
export const usePermissions = () => useAuthStore((state) => state.permissions)

/**
 * Get roles
 */
export const useRoles = () => useAuthStore((state) => state.roles)

/**
 * Get realm ID
 */
export const useRealmId = () => useAuthStore((state) => state.realmId || 'admin')

/**
 * Get actions
 */
export const useAuthActions = () =>
  useAuthStore(
    useShallow((state) => ({
      setAuthStatus: state.setAuthStatus,
      setIsLoading: state.setIsLoading,
      setUserPermissions: state.setUserPermissions,
      setUserProfile: state.setUserProfile,
      login: state.login,
      logout: state.logout,
      setTokens: state.setTokens,
      setPkceState: state.setPkceState,
      reset: state.reset,
    }))
  )
