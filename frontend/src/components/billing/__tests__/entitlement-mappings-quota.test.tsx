import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor, within } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { server } from '@/test/mocks/server'
import {
  entitlementMappingListHandler,
  batchUpdateOkCaptureHandler,
  batchUpdate409Handler,
} from '@/test/mocks/handlers/entitlement-mappings'
import {
  multiPriceWithQuotaWindowsList,
  batchUpdateOkBody,
} from '@/test/fixtures/entitlement-mappings'
import { EntitlementMappingsPage } from '../entitlement-mappings-page'

// This is an MSW INTEGRATION test (per FE-T05 spec): the real generated
// `batchUpdateEntitlementMappings` client fires a real PUT against the MSW
// handler, so the test observes the actual wire payload shape
// (`updates[*].quotaWindows`) rather than a mocked mutation function.
//
// Only `usePermission` is mocked, because the editor's `pointsDisabled` gate
// (read from `billing.manage` + `points.manage`) is environmental, not
// behavior-under-test — gating the editor's disabled state is covered by the
// component-level editor test. Here we grant both perms to exercise the
// writable path.

vi.mock('@/hooks/use-permission', () => ({
  usePermission: vi.fn(() => ({
    hasPermission: (_p: string) => true,
  })),
}))

// The ProviderSyncButton pulls in its own mutation tree unrelated to this test;
// stub it to keep the surface minimal.
vi.mock('@/components/billing/provider-sync-button', () => ({
  ProviderSyncButton: () => <div data-testid="provider-sync-button">sync</div>,
}))

import { usePermission } from '@/hooks/use-permission'

function makeWrapper(client: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>
  }
}

function renderPage(client?: QueryClient) {
  const qc =
    client ??
    new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
  const view = render(<EntitlementMappingsPage realmId="realm-1" search={{}} />, {
    wrapper: makeWrapper(qc),
  })
  return { qc, ...view }
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(usePermission).mockReturnValue({
    hasPermission: () => true,
  } as ReturnType<typeof usePermission>)
  // Seed the list endpoint with a product whose monthly price already has two
  // quota windows and whose annual price has none.
  server.use(entitlementMappingListHandler(multiPriceWithQuotaWindowsList()))
})

/**
 * Open the per-price "Advanced" panel so the embedded `MultiWindowQuotaEditor`
 * mounts (Radix `CollapsibleContent` is lazily mounted when open). Returns the
 * scope of the monthly price row, where the editor under test lives.
 */
async function openMonthlyAdvancedPanel() {
  // Wait for the product list + auto-selected detail panel to mount before
  // querying the row (the detail panel renders one cycle after the list query
  // resolves, via the `effectiveSelectedProductKey` memo).
  await screen.findByTestId('mapping-product-row-prod_pro')
  await screen.findByTestId('mapping-detail-panel')
  const monthlyRow = await screen.findByTestId('price-edit-row-price_pro_monthly')
  // The "Advanced" toggle button lives inside the row (its label flips to
  // "Hide advanced" once open — match by the open-state-independent substring).
  const advancedToggle = within(monthlyRow).getByRole('button', { name: /Advanced/i })
  await userEvent.click(advancedToggle)
  return monthlyRow
}

describe('EntitlementMappingsPage — quota-editor integration', () => {
  it('seeds the editor with the monthly price existing quota windows', async () => {
    renderPage()
    await openMonthlyAdvancedPanel()

    // The editor mounts and shows the impact-alert for the entitlement-mapping
    // context (this is the only context-driven difference in wording).
    expect(await screen.findByTestId('quota-window-editor')).toBeInTheDocument()
    expect(screen.getByTestId('quota-window-impact-alert')).toBeInTheDocument()

    // Two pre-existing windows were seeded (rows are index-based: row-0, row-1).
    // Empty-row marker must NOT be present.
    expect(screen.queryByTestId('quota-window-empty-row')).not.toBeInTheDocument()
    expect(screen.getByTestId('quota-window-row-0')).toBeInTheDocument()
    expect(screen.getByTestId('quota-window-row-1')).toBeInTheDocument()
  })

  it('adds and removes a window, updating local row state', async () => {
    renderPage()
    await openMonthlyAdvancedPanel()

    // Start: 2 windows.
    expect(screen.getByTestId('quota-window-row-0')).toBeInTheDocument()
    expect(screen.getByTestId('quota-window-row-1')).toBeInTheDocument()
    expect(screen.queryByTestId('quota-window-row-2')).not.toBeInTheDocument()

    // Add appends a default (1 hour, limit 0) window as row-2.
    await userEvent.click(screen.getByTestId('quota-window-add-button'))
    expect(screen.getByTestId('quota-window-row-2')).toBeInTheDocument()

    // Removing row-0 drops it from the rendered list (the editor re-indexes,
    // but we only assert the count shrank by one and row-0 is gone in this
    // click; subsequent indices are reassigned, so assert on the new last row).
    await userEvent.click(screen.getByTestId('quota-window-delete-row-0'))
    expect(screen.queryByTestId('quota-window-row-2')).not.toBeInTheDocument()
    // Two rows remain after one add + one delete.
    expect(screen.getAllByTestId(/^quota-window-row-\d+$/)).toHaveLength(2)
  })

  it('keeps the batch save button enabled when window rows are valid', async () => {
    // The save gate is the page's own `batchEntitlementMappingsSchema` (which
    // composes `quotaWindowSchema`: windowSeconds int>0, limit int>=0). The
    // seeded windows are valid (3600/100, 86400/1000), so save stays enabled.
    renderPage()
    await openMonthlyAdvancedPanel()

    const save = await screen.findByTestId('save-mapping-button')
    expect(save).not.toBeDisabled()
  })

  it('sends a PUT batch whose updates[*].quotaWindows mirrors the editor state', async () => {
    // Capture the wire payload by observing the MSW request (testing-guide
    // rule: do NOT mock the internal API function).
    const captured: { body: unknown } = { body: null }
    const okBody = batchUpdateOkBody({
      // Echo back the quota-windows-bearing monthly row so the success branch
      // reflects the editor state in the post-save snapshot.
      prices: multiPriceWithQuotaWindowsList(),
      saved: 2,
    })
    server.use(batchUpdateOkCaptureHandler(captured, okBody))

    renderPage()
    await openMonthlyAdvancedPanel()

    // Append one window so the payload is NOT identical to the seed (this is
    // the regression guard: `quotaWindows` must round-trip the editor's
    // latest value, not the loaded value).
    await userEvent.click(screen.getByTestId('quota-window-add-button'))
    const newRowLimitInput = screen.getByTestId('quota-window-limit-row-2')
    await userEvent.clear(newRowLimitInput)
    await userEvent.type(newRowLimitInput, '250')

    const save = screen.getByTestId('save-mapping-button')
    await userEvent.click(save)

    await waitFor(() => {
      expect(captured.body).not.toBeNull()
    })

    const body = captured.body as {
      paymentProvider: string
      externalProductId: string
      updates: Array<{ mappingId: string; quotaWindows?: Array<{ windowSeconds: number; limit: number }> | null }>
    }
    expect(body.paymentProvider).toBe('stripe')
    expect(body.externalProductId).toBe('prod_pro')
    const monthly = body.updates.find((u) => u.mappingId === 'map_pro_monthly')
    expect(monthly).toBeDefined()
    expect(monthly?.quotaWindows).toEqual([
      { windowSeconds: 3600, limit: 100 },
      { windowSeconds: 86_400, limit: 1000 },
      // Appended default: 1 hour (3600s) + limit overridden to 250.
      { windowSeconds: 3600, limit: 250 },
    ])
    // The annual row had no windows; `toPriceMappingUpdate` forwards
    // `quotaWindows: null ?? undefined`, i.e. the key is omitted from the
    // wire payload (leave-unchanged semantics). Assert it is NOT present so a
    // future regression that always sends `[]` (clearing) fails loud.
    const annual = body.updates.find((u) => u.mappingId === 'map_pro_annual')
    expect(annual).toBeDefined()
    expect(annual?.quotaWindows).toBeUndefined()
  })

  it('opens the protected-price confirm dialog on a 409 active-subscription lock', async () => {
    // The 409 path is load-bearing for the entitlement-mapping page: a batch
    // that toggles `enabled` off on a price with active subs is rolled back
    // and must surface the active-sub count (NOT a generic error toast).
    server.use(batchUpdate409Handler(28))

    renderPage()
    await openMonthlyAdvancedPanel()

    // Confirm dialog must be absent before save.
    expect(screen.queryByTestId('protected-price-confirm-dialog')).not.toBeInTheDocument()

    await userEvent.click(screen.getByTestId('save-mapping-button'))

    // The dialog opens (Radix Dialog mounts the content lazily; wait for it).
    await waitFor(() => {
      expect(screen.getByTestId('protected-price-confirm-dialog')).toBeInTheDocument()
    })
    // Active-subscription count surfaced from the 409 body.
    expect(screen.getByTestId('protected-price-active-subs').textContent).toContain('28')
  })
})
