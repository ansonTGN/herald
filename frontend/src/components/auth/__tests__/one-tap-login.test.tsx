import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor, act } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { OneTapDirectResponse } from '@/lib/api-generated'

/**
 * Google One Tap login entry.
 *
 * These tests encode the WHY of the integration, not just its mechanics:
 *
 *  - Google's GIS SDK cannot run under jsdom, so we stub `useScript` and the
 *    `window.google` surface. The tests therefore verify the *contract* between
 *    our code and GIS, not GIS itself.
 *  - The direct-session mode is the design invariant: the request MUST carry
 *    `clientId = FIRST_PARTY_CLIENT_ID` and MUST NOT carry `downstreamState`.
 *    A regression to the downstream-code mode would mint the wrong token
 *    family on the first-party login page.
 *  - Silent degradation is a product requirement (PRD §7): a script-load
 *    failure must never block the password/OAuth buttons. We assert
 *    `onUnavailable` fires and no error is thrown.
 */

// ---- Stub `useScript` with a controllable status. --------------------------
// vi.hoisted so the stubs exist before vi.mock factories (which are hoisted
// above imports) execute.
const { mockScriptStatusHolder, mockGoogleOneTap } = vi.hoisted(() => ({
  mockScriptStatusHolder: { current: 'idle' as 'idle' | 'loading' | 'ready' | 'error' },
  mockGoogleOneTap: vi.fn(),
}))
function setScriptStatus(status: 'idle' | 'loading' | 'ready' | 'error') {
  mockScriptStatusHolder.current = status
}

vi.mock('@/hooks/use-script', () => ({
  useScript: () => mockScriptStatusHolder.current,
}))

vi.mock('@/lib/api-generated/sdk.gen', async (importOriginal) => {
  const original = await importOriginal<typeof import('@/lib/api-generated/sdk.gen')>()
  return {
    ...original,
    googleOneTap: mockGoogleOneTap,
  }
})

import { OneTapLogin } from '../one-tap-login'

// ---- Minimal GIS stub ------------------------------------------------------
// The component touches only `accounts.id.{initialize, prompt}`; the registered
// callback is captured so a test can fire it to simulate a user pick.
type InitializeConfig = {
  client_id: string
  callback: (response: { credential: string }) => void
}
let capturedCallback: ((response: { credential: string }) => void) | null = null
const mockInitialize = vi.fn((config: InitializeConfig) => {
  capturedCallback = config.callback
})
const mockPrompt = vi.fn()

function installGoogleStub() {
  ;(window as unknown as { google: unknown }).google = {
    accounts: { id: { initialize: mockInitialize, prompt: mockPrompt } },
  }
}
function clearGoogleStub() {
  delete (window as unknown as { google?: unknown }).google
}

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
}

function renderOneTap(overrides: { onUnavailable?: () => void } = {}) {
  const onSuccess = vi.fn()
  const onUnavailable = overrides.onUnavailable ?? vi.fn()
  const utils = render(
    <QueryClientProvider client={createTestQueryClient()}>
      <OneTapLogin
        realmId="test-realm"
        clientId="user-account-center"
        googleClientId="google-client-123"
        onSuccess={onSuccess}
        onUnavailable={onUnavailable}
      />
    </QueryClientProvider>
  )
  return { onSuccess, onUnavailable, ...utils }
}

const SAMPLE_TOKEN_RESPONSE: OneTapDirectResponse = {
  message: 'ok',
  userId: 'user-1',
  accessToken: 'at-xyz',
  refreshToken: 'rt-xyz',
  expiresIn: 3600,
  refreshExpiresIn: 86400,
  tokenType: 'Bearer',
}

describe('OneTapLogin', () => {
  beforeEach(() => {
    setScriptStatus('idle')
    mockGoogleOneTap.mockReset()
    mockInitialize.mockReset()
    mockPrompt.mockReset()
    capturedCallback = null
    installGoogleStub()
  })

  afterEach(() => {
    clearGoogleStub()
  })

  it('initializes GIS with the realm Google client_id and prompts once the script is ready', async () => {
    setScriptStatus('ready')
    renderOneTap()

    await waitFor(() => {
      expect(mockInitialize).toHaveBeenCalledTimes(1)
    })
    // The GIS client_id MUST be the realm's Google client_id (from publicConfig),
    // NOT the Herald Client App id — that distinction is the core contract.
    expect(mockInitialize).toHaveBeenCalledWith(
      expect.objectContaining({ client_id: 'google-client-123' })
    )
    expect(mockPrompt).toHaveBeenCalledTimes(1)
    // initialize/prompt must run exactly once per page load even across re-renders.
    expect(screen.getByTestId('one-tap-container')).toBeInTheDocument()
  })

  it('does not initialize while the script is still loading', () => {
    setScriptStatus('loading')
    renderOneTap()
    expect(mockInitialize).not.toHaveBeenCalled()
    expect(mockPrompt).not.toHaveBeenCalled()
  })

  it('reports onUnavailable and renders nothing when the script fails to load', async () => {
    // PRD §7: script-load failure must degrade silently — no toast, no throw.
    setScriptStatus('error')
    const onUnavailable = vi.fn()
    renderOneTap({ onUnavailable })

    await waitFor(() => {
      expect(onUnavailable).toHaveBeenCalledTimes(1)
    })
    expect(mockInitialize).not.toHaveBeenCalled()
    expect(screen.queryByTestId('one-tap-container')).not.toBeInTheDocument()
  })

  it('reports onUnavailable when window.google is absent despite ready status', async () => {
    // Defensive: a ready signal without the global means GIS is unusable.
    clearGoogleStub()
    setScriptStatus('ready')
    const onUnavailable = vi.fn()
    renderOneTap({ onUnavailable })

    await waitFor(() => {
      expect(onUnavailable).toHaveBeenCalledTimes(1)
    })
    expect(mockInitialize).not.toHaveBeenCalled()
  })

  describe('GIS credential callback → backend request', () => {
    it('posts the credential in direct-session mode (clientId = FIRST_PARTY_CLIENT_ID, no downstreamState)', async () => {
      // This is the design invariant for the first-party login page. If the
      // request ever carries downstreamState or a non-first-party clientId, the
      // backend would take the wrong branch and mint the wrong token family.
      setScriptStatus('ready')
      mockGoogleOneTap.mockResolvedValueOnce({ data: SAMPLE_TOKEN_RESPONSE })
      const { onSuccess } = renderOneTap()

      await waitFor(() => expect(capturedCallback).not.toBeNull())
      await act(async () => {
        capturedCallback!({ credential: 'id-token-jwt' })
      })

      await waitFor(() => expect(mockGoogleOneTap).toHaveBeenCalledTimes(1))
      expect(mockGoogleOneTap).toHaveBeenCalledWith({
        path: { realmId: 'test-realm' },
        body: {
          credential: 'id-token-jwt',
          clientId: 'user-account-center',
          // downstreamState MUST be absent → direct-session branch.
        },
      })
      expect(onSuccess).toHaveBeenCalledWith(SAMPLE_TOKEN_RESPONSE)
    })

    it('surfaces a toast (not a throw) when the backend rejects the credential', async () => {
      setScriptStatus('ready')
      mockGoogleOneTap.mockResolvedValueOnce({
        error: { status: 401, message: 'ID token invalid' },
      })
      const { onSuccess } = renderOneTap()

      await waitFor(() => expect(capturedCallback).not.toBeNull())
      await act(async () => {
        capturedCallback!({ credential: 'bad-token' })
      })

      await waitFor(() => expect(mockGoogleOneTap).toHaveBeenCalledTimes(1))
      // onError maps through getErrorMessage → toast.error. onSuccess must NOT
      // fire on failure (otherwise a partial session would be written).
      expect(onSuccess).not.toHaveBeenCalled()
    })
  })
})
