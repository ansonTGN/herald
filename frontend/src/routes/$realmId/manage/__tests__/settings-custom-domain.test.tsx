/**
 * @vitest-environment jsdom
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor, within } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { http, HttpResponse } from 'msw'
import { Suspense } from 'react'
import { server } from '@/test/mocks/server'
import { queryKeys } from '@/data/query-options'

/**
 * FE-T02 — Settings custom-domain Tab lifecycle integration.
 *
 * Renders the real `SettingsPage` against MSW so the page's own mutations run
 * end-to-end (no internal mutation functions are mocked). The suite verifies:
 *   - the request payload each action sends (PUT /draft carries `{ hostname }`;
 *     POST /publish and POST /restore send NO body per design §4.2.2, since the
 *     generated client types them as `body?: never` — they publish/restore the
 *     already-saved draft),
 *   - the query cache invalidation boundary from design §4.4.2:
 *       save-draft / discard-draft → only `customDomainRealmConfig(realmId)`,
 *       publish / restore           → `customDomainRealmConfig` AND `publicConfig`.
 *       The WHY: publish/restore change the published config that drives
 *       host→realm resolution and terminal-user auth, so `publicConfig` must be
 *       invalidated or end users keep serving stale state.
 *   - the 409 conflict path (design §4.3): when the PUT /draft handler returns
 *     409 "Custom domain already in use", the mutation rejects, `onError` runs
 *     and `toast.error` fires with the server message.
 *
 * Invalidation is asserted via `vi.spyOn(queryClient, 'invalidateQueries')` —
 * the only reliable signal for `publicConfig`, which the Settings page does NOT
 * mount (its GET handler would never be re-invoked even when correctly
 * invalidated). Asserting the call args directly tests the boundary contract.
 */

const API_BASE_URL = 'http://localhost:3000'

// Mock `sonner` so the 409 test can observe `toast.error` being called with the
// server's conflict message. Hoisted alongside the other vi.mock calls below.
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}))

// --- Route + auth mocks ------------------------------------------------------
// The page reads `realmId` via `Route.useParams()` (the `Route` object is built
// by `createFileRoute` at module load). Stub the router so params resolve, and
// stub `useAuth` so the user holds both `settings.view` and `settings.manage`.
//
// `importActual` is required because the TanStack Router Vite plugin rewrites
// `createFileRoute(...)({ component })` to wrap the component in
// `lazyRouteComponent` (auto code-splitting) — the mock must preserve the real
// module's exports. The test imports `SettingsPage` directly so it renders
// synchronously, sidestepping the `lazyRouteComponent` Suspense dance.
vi.mock('@tanstack/react-router', async () => {
  const actual =
    await vi.importActual<typeof import('@tanstack/react-router')>('@tanstack/react-router')
  return {
    ...actual,
    createFileRoute: () => (config: Record<string, unknown>) => ({
      useParams: () => ({ realmId: 'test-realm' }),
      ...config,
    }),
  }
})

vi.mock('@/hooks/use-auth', () => ({
  useAuth: () => ({
    isAuthenticated: true,
    isLoading: false,
    realmId: 'test-realm',
    user: null,
    // `settings.view` lets the page render; `settings.manage` enables the
    // action buttons (the page gates every mutation behind `canUpdateConfig`).
    permissions: ['settings.view', 'settings.manage'],
    roles: [],
    hasAdminPermission: true,
    login: vi.fn(),
    logout: vi.fn(),
    setAuthStatus: vi.fn(),
    setIsLoading: vi.fn(),
    setUserPermissions: vi.fn(),
    setUserProfile: vi.fn(),
    reset: vi.fn(),
  }),
}))

// The custom-domain path is the subject under test, so it runs end-to-end
// against MSW. The sibling tabs (Providers, Legal) fire their own queries that
// are not under test; stubbing them keeps the suite focused and avoids a long
// tail of incidental MSW handlers.
vi.mock('@/components/oauth-config/provider-config-page', () => ({
  ProviderConfigPage: () => <div data-testid="providers-stub" />,
}))
vi.mock('@/components/settings/LegalAgreementTab', () => ({
  LegalAgreementTab: () => <div data-testid="legal-stub" />,
}))

// Imported after the `vi.mock` calls so the hoisted router/auth/sonner mocks
// apply before the page module evaluates. `SettingsPage` is the direct
// component (not the Vite-plugin-wrapped `Route.component`) so the render is
// synchronous.
import { SettingsPage } from '../settings'
import { toast } from 'sonner'

// --- Fixtures ----------------------------------------------------------------

const PUBLISHED_HOSTNAME = 'login.published.example.com'
const DRAFT_HOSTNAME = 'login.draft.example.com'

/** A realm with a published config and no draft. */
function publishedState(hasPrevious = false) {
  return {
    published: { hostname: PUBLISHED_HOSTNAME },
    draft: null,
    hasPrevious,
    cnameTarget: 'custom.herald.com',
    status: null,
  }
}

/** A realm with both a published config and an unpublished draft. */
function publishedWithDraftState(hasPrevious = false) {
  return {
    published: { hostname: PUBLISHED_HOSTNAME },
    draft: { hostname: DRAFT_HOSTNAME },
    hasPrevious,
    cnameTarget: 'custom.herald.com',
    status: null,
  }
}

/**
 * Default MSW handlers for the custom-domain lifecycle endpoints. Each handler
 * records the requests it saw so tests can assert payload + call count.
 *
 * The GET handler closes over a mutable `state`; the DELETE handler clears
 * `draft` so subsequent GETs (the post-discard refetch) report the
 * published-only state. The PUT /draft handler records the JSON body (the only
 * lifecycle op that sends a request body — `{ hostname }`).
 *
 * NOTE (design §4.2.2): publish and restore send NO request body (generated
 * client `body?: never`). Their handlers record the call count only and MUST
 * NOT assert a request body — publish publishes the already-saved draft, and
 * restore swaps `settings`/`previous_settings` server-side.
 */
function installCustomDomainHandlers(initialState: ReturnType<typeof publishedState>) {
  let state = initialState
  const calls = {
    get: vi.fn(),
    putDraft: vi.fn(),
    deleteDraft: vi.fn(),
    publish: vi.fn(),
    restore: vi.fn(),
  }

  const baseUrl = `${API_BASE_URL}/api/realms/test-realm/config/custom-domain`

  server.use(
    http.get(baseUrl, () => {
      calls.get()
      return HttpResponse.json(state)
    }),
    http.put(`${baseUrl}/draft`, async ({ request }) => {
      const body = await request.json()
      calls.putDraft(body)
      return HttpResponse.json({ ...state, draft: body, message: 'draft saved' })
    }),
    http.delete(`${baseUrl}/draft`, () => {
      calls.deleteDraft()
      // After discard the draft is gone — reflect that in subsequent GETs.
      state = { ...state, draft: null }
      return HttpResponse.json({ ...state, message: 'draft discarded' })
    }),
    // publish: no request body asserted (generated client sends `body?: never`).
    http.post(`${baseUrl}/publish`, () => {
      calls.publish()
      return HttpResponse.json({
        hasPrevious: true,
        status: state.status,
        message: 'published',
      })
    }),
    // restore: no request body asserted (generated client sends `body?: never`).
    http.post(`${baseUrl}/restore`, () => {
      calls.restore()
      return HttpResponse.json({
        hasPrevious: true,
        status: state.status,
        message: 'restored',
      })
    })
  )

  return { calls }
}

/**
 * Minimal handlers for the *other* Settings-page queries so mounting every tab
 * (TabsContent renders all children) does not fire unhandled requests. These
 * are not under test; returning empty/valid shapes keeps the page stable.
 * Includes the white-label tab GET so its query resolves without an unhandled
 * request warning.
 */
function installSiblingTabHandlers() {
  server.use(
    // GeneralTab reads the realm (name/description) — needed so the tab mounts.
    http.get(`${API_BASE_URL}/api/realms/test-realm`, () =>
      HttpResponse.json({ id: 'test-realm', name: 'Test Realm', description: '' })
    ),
    // listRealmConfigs (TOTP/Turnstile/Registration/Email parse from this)
    http.get(`${API_BASE_URL}/api/configs/test-realm`, () => HttpResponse.json([])),
    // email status
    http.get(`${API_BASE_URL}/api/configs/test-realm/email/status`, () =>
      HttpResponse.json({ configured: false })
    ),
    // passkey realm config
    http.get(`${API_BASE_URL}/api/realms/test-realm/config/passkey`, () =>
      HttpResponse.json({
        enabled: false,
        forceEnabled: false,
        userVerification: 'preferred',
        crossPlatformAuthenticator: true,
      })
    ),
    // white-label tab GET — sibling config tab also mounts on the page.
    http.get(`${API_BASE_URL}/api/realms/test-realm/config/white-label`, () =>
      HttpResponse.json({
        published: null,
        draft: null,
        hasPrevious: false,
        publishedUpdatedAt: null,
        draftUpdatedAt: null,
      })
    )
  )
}

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0, staleTime: 0 },
      mutations: { retry: false },
    },
  })
}

function renderSettingsPage(queryClient: QueryClient) {
  // `Route.component` is the router-plugin's `lazyRouteComponent`-wrapped page,
  // which suspends on first render until its dynamic import resolves. Wrap it
  // in Suspense so the tree resolves instead of staying in an `act` limbo.
  return render(
    <QueryClientProvider client={queryClient}>
      <Suspense fallback={<div data-testid="settings-suspense">Loading…</div>}>
        <SettingsPage />
      </Suspense>
    </QueryClientProvider>
  )
}

/** Switches the visible tab to the custom-domain editor. */
async function openCustomDomainTab(user: ReturnType<typeof userEvent.setup>) {
  // The page mounts with loading state while queries resolve; wait for the
  // tab strip (always rendered once not-loading / not-access-denied) first.
  await screen.findByTestId('custom-domain-tab', undefined, { timeout: 5000 })
  await user.click(screen.getByTestId('custom-domain-tab'))
  // Wait for the editor (loaded state resolves to the form) to appear.
  await screen.findByTestId('custom-domain-save-draft')
}

describe('Settings custom-domain tab lifecycle (FE-T02)', () => {
  let queryClient: QueryClient
  let invalidateSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    vi.clearAllMocks()
    queryClient = createTestQueryClient()
    // Spy on invalidateQueries to assert the exact invalidation boundary. The
    // page's mutations call this directly in their `onSuccess`; recording the
    // queryKey args is the authoritative signal for which caches were touched.
    invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries')
  })

  afterEach(() => {
    invalidateSpy.mockRestore()
  })

  describe('save draft', () => {
    it('sends a PUT /draft with the edited hostname and invalidates only customDomainRealmConfig', async () => {
      const user = userEvent.setup({ delay: null })
      const { calls } = installCustomDomainHandlers(publishedState(false))
      installSiblingTabHandlers()

      renderSettingsPage(queryClient)
      await openCustomDomainTab(user)

      // Edit the hostname. The form initial value is the published hostname;
      // clearing it and typing a new value makes the form dirty and changes
      // the payload.
      const hostnameInput = screen.getByTestId('custom-domain-hostname') as HTMLInputElement
      await user.clear(hostnameInput)
      await user.type(hostnameInput, 'login.new.example.com')

      await user.click(screen.getByTestId('custom-domain-save-draft'))

      // PUT /draft was called once with the edited hostname.
      await waitFor(() => {
        expect(calls.putDraft).toHaveBeenCalledTimes(1)
      })
      expect(calls.putDraft).toHaveBeenCalledWith(
        expect.objectContaining({ hostname: 'login.new.example.com' })
      )

      // Invalidation boundary: only customDomainRealmConfig, NOT publicConfig.
      // save-draft touches only the admin draft state, not the published config
      // that drives terminal-user auth, so publicConfig must stay untouched.
      await waitFor(() => {
        expect(invalidateSpy).toHaveBeenCalledWith({
          queryKey: queryKeys.customDomainRealmConfig('test-realm'),
        })
      })
      expect(invalidateSpy).not.toHaveBeenCalledWith({
        queryKey: queryKeys.publicConfig('test-realm'),
      })

      // Observability cross-check: customDomainRealmConfig IS mounted by the
      // page, so invalidating it triggers a refetch the GET spy observes.
      await waitFor(() => {
        expect(calls.get.mock.calls.length).toBeGreaterThan(1)
      })
    })
  })

  describe('publish', () => {
    it('posts to /publish with no body and invalidates both customDomainRealmConfig and publicConfig', async () => {
      const user = userEvent.setup({ delay: null })
      const { calls } = installCustomDomainHandlers(publishedState(false))
      installSiblingTabHandlers()

      renderSettingsPage(queryClient)
      await openCustomDomainTab(user)

      await user.click(screen.getByTestId('custom-domain-publish'))

      // POST /publish fired once. Per design §4.2.2 the generated client types
      // publish as `body?: never` — it publishes the already-saved draft — so
      // we assert call count only, NOT a request body.
      await waitFor(() => {
        expect(calls.publish).toHaveBeenCalledTimes(1)
      })

      // Invalidation boundary: publish changes the published config that drives
      // host→realm resolution and terminal-user auth, so BOTH the admin state
      // and the publicConfig must be invalidated.
      await waitFor(() => {
        expect(invalidateSpy).toHaveBeenCalledWith({
          queryKey: queryKeys.customDomainRealmConfig('test-realm'),
        })
        expect(invalidateSpy).toHaveBeenCalledWith({
          queryKey: queryKeys.publicConfig('test-realm'),
        })
      })
    })
  })

  describe('restore', () => {
    it('gates restore behind hasPrevious, confirms via dialog, and invalidates both caches', async () => {
      const user = userEvent.setup({ delay: null })
      // hasPrevious=true enables the restore button.
      const { calls } = installCustomDomainHandlers(publishedState(true))
      installSiblingTabHandlers()

      renderSettingsPage(queryClient)
      await openCustomDomainTab(user)

      const restoreButton = screen.getByTestId('custom-domain-restore')
      // Gate: restore is only enabled when a previous version exists.
      expect(restoreButton).not.toBeDisabled()

      // Opening the confirm dialog must NOT fire restore yet.
      await user.click(restoreButton)
      const dialog = await screen.findByTestId('custom-domain-restore-dialog')
      expect(calls.restore).not.toHaveBeenCalled()

      // Confirm inside the dialog.
      const confirmButton = within(dialog).getByTestId('custom-domain-restore-confirm')
      await user.click(confirmButton)

      // restore was called once. Per design §4.2.2 the generated client types
      // restore as `body?: never` — assert call count only, NOT a request body.
      await waitFor(() => {
        expect(calls.restore).toHaveBeenCalledTimes(1)
      })

      // Restore changes the published config → both caches invalidated.
      await waitFor(() => {
        expect(invalidateSpy).toHaveBeenCalledWith({
          queryKey: queryKeys.customDomainRealmConfig('test-realm'),
        })
        expect(invalidateSpy).toHaveBeenCalledWith({
          queryKey: queryKeys.publicConfig('test-realm'),
        })
      })
    })

    it('disables restore when there is no previous version', async () => {
      const user = userEvent.setup({ delay: null })
      installCustomDomainHandlers(publishedState(false))
      installSiblingTabHandlers()

      renderSettingsPage(queryClient)
      await openCustomDomainTab(user)

      expect(screen.getByTestId('custom-domain-restore')).toBeDisabled()
    })
  })

  describe('discard draft', () => {
    it('deletes the draft, refetches admin state (draft cleared), and does not invalidate publicConfig', async () => {
      const user = userEvent.setup({ delay: null })
      // Load a state that already has an unpublished draft.
      const { calls } = installCustomDomainHandlers(publishedWithDraftState(false))
      installSiblingTabHandlers()

      renderSettingsPage(queryClient)
      await openCustomDomainTab(user)

      // The form edits `draft ?? published`, so the editor starts on the draft
      // hostname — confirms the draft was loaded into the editor.
      expect(screen.getByTestId('custom-domain-hostname')).toHaveValue(DRAFT_HOSTNAME)

      // Edit a field to ensure the form is dirty before discarding.
      await user.type(screen.getByTestId('custom-domain-hostname'), '-edited')

      await user.click(screen.getByTestId('custom-domain-discard-draft'))

      // DELETE /draft fired exactly once.
      await waitFor(() => {
        expect(calls.deleteDraft).toHaveBeenCalledTimes(1)
      })

      // Invalidation boundary: discard only touches admin draft state — only
      // customDomainRealmConfig is invalidated, publicConfig is NOT.
      await waitFor(() => {
        expect(invalidateSpy).toHaveBeenCalledWith({
          queryKey: queryKeys.customDomainRealmConfig('test-realm'),
        })
      })
      expect(invalidateSpy).not.toHaveBeenCalledWith({
        queryKey: queryKeys.publicConfig('test-realm'),
      })

      // The customDomainRealmConfig query IS mounted by the page, so
      // invalidating it triggers a refetch. The DELETE handler's response
      // already cleared `draft`, so the next GET observes the published-only
      // state — i.e. the admin cache no longer reports a draft (hasDraft is
      // false afterwards).
      await waitFor(() => {
        // get spy called at least twice: initial mount + post-discard refetch.
        expect(calls.get.mock.calls.length).toBeGreaterThanOrEqual(2)
      })

      // The discard button is gated behind `hasDraft`; once the refetched state
      // reports no draft, the button disables — a stable, observable signal
      // that the admin state reflects "no draft" after discard.
      await waitFor(() => {
        expect(screen.getByTestId('custom-domain-discard-draft')).toBeDisabled()
      })

      // After discard the form must revert to the `published` config (the
      // source flips from draft to published on refetch). The editor started on
      // the draft hostname and was edited mid-flight, so both the draft value
      // and the "-edited" tail must be gone.
      await waitFor(() => {
        expect(screen.getByTestId('custom-domain-hostname')).toHaveValue(PUBLISHED_HOSTNAME)
      })
    })
  })

  describe('409 conflict', () => {
    it('shows the localized conflict toast when PUT /draft returns 409 "already in use"', async () => {
      const user = userEvent.setup({ delay: null })

      const baseUrl = `${API_BASE_URL}/api/realms/test-realm/config/custom-domain`
      server.use(
        http.get(baseUrl, () => HttpResponse.json(publishedState(false))),
        // Override ONLY the PUT handler to return a 409 conflict (design §4.3:
        // "hostname 已被其他 Realm 占用"). The mandatory
        // `.then((response) => { if (response.error) throw response.error })`
        // rethrow in the save-draft mutation turns this into a rejection. The
        // onError handler detects `status === 409` and shows the dedicated
        // localized `domain_in_use` message instead of the raw server string,
        // so a zh-CN user sees a translated message rather than English.
        http.put(`${baseUrl}/draft`, () =>
          HttpResponse.json({ message: 'Custom domain already in use' }, { status: 409 })
        ),
        http.delete(`${baseUrl}/draft`, () => HttpResponse.json({ message: 'ok' })),
        http.post(`${baseUrl}/publish`, () => HttpResponse.json({ message: 'ok' })),
        http.post(`${baseUrl}/restore`, () => HttpResponse.json({ message: 'ok' }))
      )
      installSiblingTabHandlers()

      renderSettingsPage(queryClient)
      await openCustomDomainTab(user)

      // Edit the hostname to a value that collides with another realm.
      const hostnameInput = screen.getByTestId('custom-domain-hostname') as HTMLInputElement
      await user.clear(hostnameInput)
      await user.type(hostnameInput, 'login.taken.example.com')

      await user.click(screen.getByTestId('custom-domain-save-draft'))

      // The mutation rejects, onError runs, and the 409 branch fires the
      // localized conflict message (tests run under the fixed `en` locale from
      // test/setup.ts). Asserting the localized string — NOT the raw server
      // body — is what verifies the domain_in_use key is actually wired.
      await waitFor(() => {
        expect(toast.error).toHaveBeenCalledWith('This domain is already in use by another realm.')
      })
    })
  })
})
