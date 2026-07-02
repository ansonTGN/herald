/**
 * @vitest-environment jsdom
 *
 * FE-T06 — Integration tests for the realm default-config free-periodic quota
 * editor (`MultiWindowQuotaEditor` embedded in the
 * `/$realmId/manage/points/default-config` route).
 *
 * This is an MSW INTEGRATION test: the real `pointsDefaultConfigQueryOptions`
 * and `updatePointsDefaultConfigMutation` fire against MSW handlers, so the
 * assertions observe the real wire payload (top-level
 * `freePeriodicQuotaWindows` array, `PUT /api/points/{realmId}/default-config`)
 * rather than a mocked mutation function. The only mock is
 * `@tanstack/react-router`, because the page reads `Route.useParams()` and
 * there is no real Router context in jsdom — the router is environmental, not
 * behavior-under-test.
 *
 * Save gate (pinned): `useAppForm` wires the zod schema as an `onChange`
 * validator, so `state.canSubmit` reflects field validity live. An invalid
 * window row (windowSeconds 0 / negative, or limit < 0) therefore disables the
 * Save button without a submit round-trip.
 *
 * Mutation semantics (pinned): `freePeriodicQuotaWindows` is a top-level body
 * field. None=unchanged / Some([])=clear / Some[..]=replace — here the form
 * always carries a concrete array, so the editor's current value is sent
 * verbatim (replace semantics).
 *
 * Rendering note: we import `RealmConfigPage` directly rather than
 * `Route.component`. The TanStack Router vite plugin (`autoCodeSplitting: true`
 * in vitest.config.ts) rewrites `Route.component` into a `Lazy` wrapper that
 * only resolves inside a real Router context; in jsdom that lazy wrapper throws
 * a promise that never settles, so the page hangs forever under `<Suspense>`.
 * Importing the page component directly (mirroring FE-T04/FE-T05, which import
 * their page components directly) sidesteps the lazy wrapper entirely.
 */
import { describe, it, expect, afterEach, beforeEach, vi } from 'vitest'

import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import type { ReactNode } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { server } from '@/test/mocks/server'
import {
  createRealmConfigHandlerWithQuotaWindows,
  createRealmConfigErrorHandler,
} from '@/test/mocks/handlers/points'
import { toast } from 'sonner'
import { queryKeys } from '@/data/query-options'

// --- Only mock: the Router. The page calls `Route.useParams()` and the route
// is defined via `createFileRoute`. Provide a stable realmId + a passthrough
// `createFileRoute` so the real `RealmConfigPage` (imported below) sees a
// `Route.useParams()` that returns a stable realmId. Preserve all other real
// router exports.
vi.mock('@tanstack/react-router', async (importOriginal) => {
  const actual = (await importOriginal()) as Record<string, unknown>
  const routeObj = {
    useParams: () => ({ realmId: 'test-realm' }),
  }
  return {
    ...actual,
    createFileRoute: () => (config: Record<string, unknown>) => ({ ...routeObj, ...config }),
  }
})

// The page now reads `useAuth()` for the SETTINGS_VIEW / SETTINGS_MANAGE gate
// (FE-A04) — matching backend realm_configs.rs + design §3.3/§4.5, which gate
// the default-config endpoints on settings.view/settings.manage. Mock the hook
// as a vi.fn so authorized-admin state is the default for the editor-behavior
// cases above, and per-test `mockReturnValue` overrides can flip the permission
// set to exercise the gate. `permissions` is the only field RealmConfigPage reads.
vi.mock('@/hooks/use-auth', () => ({
  useAuth: vi.fn(() => ({ permissions: ['settings.view', 'settings.manage'] })),
}))

// `sonner` is globally mocked in setup.ts; re-import the mock to assert the
// success/error toasts.
// The route file exports both `Route` (consumed by the page for `useParams`)
// and `RealmConfigPage` (the unwrapped page component). We render
// `RealmConfigPage` directly to avoid the autoCodeSplitting Lazy wrapper (see
// the rendering note above).
import { RealmConfigPage } from '@/routes/$realmId/manage/points/default-config'
import { pointsDefaultConfigSchema } from '@/lib/schemas/points-forms'
import { useAuth } from '@/hooks/use-auth'

const SEEDED_WINDOWS = [
  { windowSeconds: 3600, limit: 100 }, // 1 hour, limit 100
  { windowSeconds: 86_400, limit: 1000 }, // 1 day, limit 1000
]

function makeWrapper(client: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>
  }
}

function renderPage(client?: QueryClient) {
  const qc =
    client ??
    new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    })
  const view = render(<RealmConfigPage />, { wrapper: makeWrapper(qc) })
  return { qc, ...view }
}

describe('Realm default-config — free-periodic quota editor integration', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // Seed the GET endpoint with two pre-existing quota windows.
    server.use(createRealmConfigHandlerWithQuotaWindows(SEEDED_WINDOWS))
  })

  afterEach(() => {
    server.resetHandlers()
  })

  it('seeds the editor with the loaded free-periodic quota windows', async () => {
    renderPage()

    // The GET resolves via MSW on the first microtask; await the form root.
    await screen.findByTestId('points-default-config-form')

    // Editor mounts for the realm-default context (testIdPrefix=realm-default-window).
    expect(await screen.findByTestId('realm-default-window-editor')).toBeInTheDocument()
    expect(screen.getByTestId('realm-default-window-impact-tooltip')).toBeInTheDocument()

    // Two pre-existing rows seeded; the empty-row marker must NOT be present.
    expect(screen.queryByTestId('realm-default-window-empty-row')).not.toBeInTheDocument()
    expect(screen.getByTestId('realm-default-window-row-0')).toBeInTheDocument()
    expect(screen.getByTestId('realm-default-window-row-1')).toBeInTheDocument()

    // Seeded length value mirrors windowSeconds[0]=3600s, displayed as "1" in
    // the largest evenly-dividing unit (hours). This is the regression guard
    // for the seeded-value round-trip.
    expect(screen.getByTestId('realm-default-window-length-row-0')).toHaveValue(1)
  })

  it('adds and removes a window, updating local editor state', async () => {
    renderPage()
    await screen.findByTestId('realm-default-window-row-1')

    // Start: 2 windows.
    expect(screen.getAllByTestId(/^realm-default-window-row-\d+$/)).toHaveLength(2)

    // Add appends a default (1 hour, limit 0) window.
    await userEvent.click(screen.getByTestId('realm-default-window-add-button'))
    expect(screen.getAllByTestId(/^realm-default-window-row-\d+$/)).toHaveLength(3)
    expect(screen.getByTestId('realm-default-window-row-2')).toBeInTheDocument()

    // Removing the first row drops it; the editor re-indexes, so assert on the
    // new count (one fewer) rather than a specific id.
    await userEvent.click(screen.getByTestId('realm-default-window-delete-row-0'))
    expect(screen.getAllByTestId(/^realm-default-window-row-\d+$/)).toHaveLength(2)
  })

  it('disables Save when an added window row is invalid (windowSeconds=0)', async () => {
    // WHY: the form uses the zod schema as an onChange validator, so an
    // invalid window must gate the Save button live (no submit round-trip).
    // `quotaWindowSchema` requires windowSeconds int >= 1.
    renderPage()
    const save = await screen.findByTestId('save-config-button')
    expect(save).not.toBeDisabled()

    // Add a window (default 3600s, valid) then clear its length to 0.
    await userEvent.click(screen.getByTestId('realm-default-window-add-button'))
    const newLength = screen.getByTestId('realm-default-window-length-row-2')
    await userEvent.clear(newLength)
    await userEvent.type(newLength, '0')

    // 0 hours * 3600 = 0s → fails windowSeconds >= 1 → Save disabled.
    await waitFor(() => {
      expect(save).toBeDisabled()
    })
  })

  it('sends a PUT whose freePeriodicQuotaWindows mirrors the editor state', async () => {
    // Capture the wire payload by observing the MSW request (do NOT mock the
    // internal mutation function — testing-guide rule).
    const captured: { body: unknown } = { body: null }
    server.use(
      http.put(
        'http://localhost:3000/api/points/test-realm/default-config',
        async ({ request }) => {
          captured.body = await request.json()
          return HttpResponse.json({
            realmId: 'test-realm',
            registrationBonusPoints: 1000,
            freePeriodicPointsAmount: 50,
            freePeriodicGrantPeriodType: 'daily',
            freePeriodicValidityDays: 1,
            freePeriodicQuotaWindows: SEEDED_WINDOWS,
            updatedAt: '2026-03-24T00:00:00Z',
          })
        }
      )
    )

    renderPage()
    await screen.findByTestId('realm-default-window-row-1')

    // Append one window and override its limit so the payload is NOT identical
    // to the seed (regression guard: the array must round-trip the editor's
    // latest value, not the loaded value).
    await userEvent.click(screen.getByTestId('realm-default-window-add-button'))
    const newRowLimit = screen.getByTestId('realm-default-window-limit-row-2')
    await userEvent.clear(newRowLimit)
    await userEvent.type(newRowLimit, '250')

    await userEvent.click(screen.getByTestId('save-config-button'))

    await waitFor(() => {
      expect(captured.body).not.toBeNull()
    })

    const body = captured.body as {
      registrationBonusPoints: number
      freePeriodicPointsAmount: number
      freePeriodicGrantPeriodType: string
      freePeriodicValidityDays: number
      freePeriodicQuotaWindows?: Array<{ windowSeconds: number; limit: number }>
    }
    // freePeriodicQuotaWindows is a top-level field, replace semantics.
    expect(body.freePeriodicQuotaWindows).toEqual([
      { windowSeconds: 3600, limit: 100 },
      { windowSeconds: 86_400, limit: 1000 },
      // Appended default window: 1 hour (3600s) + limit overridden to 250.
      { windowSeconds: 3600, limit: 250 },
    ])
    // The existing periodic fields coexist alongside the windows.
    expect(body.registrationBonusPoints).toBe(1000)
    expect(body.freePeriodicGrantPeriodType).toBe('daily')
  })

  it('invalidates the pointsDefaultConfig query and shows success toast on save', async () => {
    const qc = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    })
    const invalidateSpy = vi.spyOn(qc, 'invalidateQueries')

    renderPage(qc)
    await screen.findByTestId('realm-default-window-row-1')

    await userEvent.click(screen.getByTestId('save-config-button'))

    // Success path: the query is invalidated under the real query key, and the
    // success toast fires. Both are load-bearing — a regression that forgets
    // to invalidate leaves stale editor state on next mount.
    await waitFor(() => {
      expect(invalidateSpy).toHaveBeenCalledWith({
        queryKey: queryKeys.pointsDefaultConfig('test-realm'),
      })
    })
    await waitFor(() => {
      expect(toast.success).toHaveBeenCalled()
    })
  })

  it('surfaces an error toast when the save PUT fails', async () => {
    // WHY: a failed save must NOT invalidate the cache and must tell the user.
    server.use(createRealmConfigErrorHandler(500, 'Server error, please try later'))

    const qc = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    })
    const invalidateSpy = vi.spyOn(qc, 'invalidateQueries')

    renderPage(qc)
    await screen.findByTestId('realm-default-window-row-1')

    await userEvent.click(screen.getByTestId('save-config-button'))

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalled()
    })
    // On error the config query must NOT be invalidated (no stale overwrite).
    expect(invalidateSpy).not.toHaveBeenCalledWith({
      queryKey: queryKeys.pointsDefaultConfig('test-realm'),
    })
  })

  it('coexists with existing periodic fields: changing registration bonus submits both', async () => {
    // WHY: the redesign added freePeriodicQuotaWindows alongside the existing
    // periodic fields; a regression that drops or overwrites either side fails
    // here. Edit an existing field AND keep the seeded windows, then assert
    // both are present in one PUT.
    const captured: { body: unknown } = { body: null }
    server.use(
      http.put(
        'http://localhost:3000/api/points/test-realm/default-config',
        async ({ request }) => {
          captured.body = await request.json()
          return HttpResponse.json({
            realmId: 'test-realm',
            registrationBonusPoints: 2000,
            freePeriodicPointsAmount: 50,
            freePeriodicGrantPeriodType: 'weekly',
            freePeriodicValidityDays: 7,
            freePeriodicQuotaWindows: SEEDED_WINDOWS,
            updatedAt: '2026-03-24T00:00:00Z',
          })
        }
      )
    )

    renderPage()
    const bonusInput = await screen.findByTestId('registration-bonus-points-input')
    await userEvent.clear(bonusInput)
    await userEvent.type(bonusInput, '2000')

    await userEvent.click(screen.getByTestId('save-config-button'))

    await waitFor(() => {
      expect(captured.body).not.toBeNull()
    })
    const body = captured.body as {
      registrationBonusPoints: number
      freePeriodicQuotaWindows?: Array<{ windowSeconds: number; limit: number }>
    }
    expect(body.registrationBonusPoints).toBe(2000)
    // Seeded windows travel alongside the edited bonus — both sides intact.
    expect(body.freePeriodicQuotaWindows).toEqual(SEEDED_WINDOWS)
  })
})

describe('pointsDefaultConfigSchema — freePeriodicQuotaWindows array contract', () => {
  // WHY: the schema is the save gate (onChange validator). These cases pin the
  // accept/reject boundary so a future loosening (e.g. dropping .max(8) or the
  // per-window min) fails the editor's own protection.
  it.each([
    ['accepts a valid windows array', [{ windowSeconds: 3600, limit: 0 }], true],
    ['accepts an empty array (clear semantics)', [], true],
    ['accepts exactly 8 windows (the cap)', Array(8).fill({ windowSeconds: 60, limit: 1 }), true],
    ['rejects windowSeconds 0', [{ windowSeconds: 0, limit: 1 }], false],
    ['rejects a negative limit', [{ windowSeconds: 60, limit: -1 }], false],
  ])(
    '%s',
    (_label: string, windows: Array<{ windowSeconds: number; limit: number }>, ok: boolean) => {
      const result = pointsDefaultConfigSchema.safeParse({
        registrationBonusPoints: 1000,
        freePeriodicPointsAmount: 50,
        freePeriodicGrantPeriodType: 'daily',
        freePeriodicValidityDays: 1,
        freePeriodicQuotaWindows: windows,
      })
      expect(result.success).toBe(ok)
    }
  )

  it('rejects more than 8 windows', () => {
    const result = pointsDefaultConfigSchema.safeParse({
      registrationBonusPoints: 1000,
      freePeriodicPointsAmount: 50,
      freePeriodicGrantPeriodType: 'daily',
      freePeriodicValidityDays: 1,
      freePeriodicQuotaWindows: Array(9).fill({ windowSeconds: 60, limit: 1 }),
    })
    expect(result.success).toBe(false)
  })
})

describe('RealmConfigPage — settings permission gate (FE-A04)', () => {
  // WHY: FE-A04 added defense-in-depth permission parity with settings.tsx.
  // These cases make the gate load-bearing: removing the early-return or the
  // input/save disable fails here.
  afterEach(() => {
    vi.clearAllMocks()
    // Restore the authorized-admin default so cases here don't leak into the
    // integration suite above (module-level mock returns view+manage).
    vi.mocked(useAuth).mockReturnValue({
      permissions: ['settings.view', 'settings.manage'],
    } as ReturnType<typeof useAuth>)
    server.resetHandlers()
  })

  it('renders AccessDenied when the user lacks settings.view', async () => {
    vi.mocked(useAuth).mockReturnValue({ permissions: [] } as ReturnType<typeof useAuth>)
    renderPage()

    // WHY: without settings.view the page must early-return AccessDenied and must
    // NOT mount the editor (which would have fired the config query). The
    // editor's root testid therefore must be absent.
    await waitFor(() => {
      expect(screen.getByText(/Access denied/i)).toBeInTheDocument()
    })
    expect(screen.queryByTestId('points-default-config-form')).not.toBeInTheDocument()
  })

  it('disables inputs and Save when the user has settings.view but not settings.manage', async () => {
    vi.mocked(useAuth).mockReturnValue({
      permissions: ['settings.view'],
    } as ReturnType<typeof useAuth>)
    renderPage()
    await screen.findByTestId('points-default-config-form')

    // WHY: a viewer (no settings.manage) must not be able to edit or save. The
    // registration-bonus input and the Save button are the two named controls;
    // both must be disabled purely from the missing permission.
    expect(screen.getByTestId('registration-bonus-points-input')).toBeDisabled()
    expect(screen.getByTestId('save-config-button')).toBeDisabled()
  })
})
