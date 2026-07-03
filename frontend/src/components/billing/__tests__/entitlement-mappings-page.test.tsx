import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor, within } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import type { EntitlementMappingResponse } from '@/lib/api-generated'

// --- Mocks ----------------------------------------------------------------

// Permission hook: default to a fully-privileged admin.
vi.mock('@/hooks/use-permission', () => ({
  usePermission: vi.fn(() => ({
    hasPermission: (_p: string) => true,
  })),
}))

// Query options: return canned flat price-granularity data.
vi.mock('@/data/query-options', () => ({
  queryKeys: {
    entitlementMappings: (realmId: string) => ['entitlement-mappings', realmId],
  },
  entitlementMappingsQueryOptions: () => ({
    queryKey: ['entitlement-mappings', 'realm-1'],
    queryFn: async () => undefined,
  }),
}))

// Mutations: controllable from each test. Use vi.hoisted so the factory
// (which Vitest hoists above imports) can reference these bindings.
const { mockBatchMutate, mockIsProtectedPriceError, mockExtractActiveSubscriptions } = vi.hoisted(
  () => {
    const mockBatchMutate = vi.fn()
    const mockIsProtectedPriceError = (e: unknown) =>
      !!e &&
      typeof e === 'object' &&
      (e as { code?: unknown }).code === 'mapping_in_use' &&
      typeof (e as { activeSubscriptions?: unknown }).activeSubscriptions === 'number'
    const mockExtractActiveSubscriptions = (e: unknown) =>
      mockIsProtectedPriceError(e)
        ? (e as { activeSubscriptions: number }).activeSubscriptions
        : null
    return { mockBatchMutate, mockIsProtectedPriceError, mockExtractActiveSubscriptions }
  }
)

vi.mock('@/data/entitlement-mapping-mutations', () => ({
  useBatchUpdateEntitlementMappings: () => ({
    mutate: mockBatchMutate,
    isPending: false,
  }),
  isProtectedPriceError: mockIsProtectedPriceError,
  extractActiveSubscriptions: mockExtractActiveSubscriptions,
}))

// Sync button: stub to avoid pulling in its mutation tree.
vi.mock('@/components/billing/provider-sync-button', () => ({
  ProviderSyncButton: () => <div data-testid="provider-sync-button">sync</div>,
}))

import { EntitlementMappingsPage } from '../entitlement-mappings-page'
import { usePermission } from '@/hooks/use-permission'
import { m } from '@/paraglide/messages'

// --- Fixtures --------------------------------------------------------------

function makeMapping(overrides: Partial<EntitlementMappingResponse>): EntitlementMappingResponse {
  return {
    id: overrides.id ?? 'm-1',
    bucketId: 'b-1',
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    paymentProvider: 'stripe',
    externalProductId: 'prod_pro',
    externalPriceId: null,
    entitlementKey: 'pro-plan',
    enabled: true,
    grantOnSubscribe: false,
    syncedAt: '2026-01-02T00:00:00Z',
    ...overrides,
  } as EntitlementMappingResponse
}

// Inject canned data into the query cache before render.
function seedData(client: QueryClient, items: EntitlementMappingResponse[]) {
  client.setQueryData(['entitlement-mappings', 'realm-1'], { items, total: items.length })
}

function makeWrapper(client: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>
  }
}

function renderPage(items: EntitlementMappingResponse[] = [], client?: QueryClient) {
  const qc =
    client ??
    new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
  seedData(qc, items)
  const wrapper = makeWrapper(qc)
  const view = render(<EntitlementMappingsPage realmId="realm-1" search={{}} />, { wrapper })
  return { qc, ...view }
}

beforeEach(() => {
  vi.clearAllMocks()
  // Reset permission to full admin by default.
  vi.mocked(usePermission).mockReturnValue({
    hasPermission: () => true,
  } as ReturnType<typeof usePermission>)
})

// --- Tests -----------------------------------------------------------------

describe('EntitlementMappingsPage (master-detail)', () => {
  it('groups a flat price list into product rows and price edit rows', async () => {
    // prod_pro has two prices sharing `pro-plan`; prod_starter has one.
    renderPage([
      makeMapping({
        id: 'm-1',
        externalProductId: 'prod_pro',
        externalPriceId: 'price_monthly',
        entitlementKey: 'pro-plan',
        enabled: true,
        billingType: 'recurring',
        pointsPerPeriod: 1000,
      }),
      makeMapping({
        id: 'm-2',
        externalProductId: 'prod_pro',
        externalPriceId: 'price_annual',
        entitlementKey: 'pro-plan',
        enabled: true,
        billingType: 'recurring',
        pointsPerPeriod: 12000,
      }),
      makeMapping({
        id: 'm-3',
        externalProductId: 'prod_starter',
        externalPriceId: 'price_once',
        entitlementKey: 'starter',
        enabled: false,
        billingType: 'one_time',
        pointsPerPeriod: 500,
      }),
    ])

    // Two product rows.
    expect(await screen.findByTestId('mapping-product-row-prod_pro')).toBeTruthy()
    expect(screen.getByTestId('mapping-product-row-prod_starter')).toBeTruthy()

    // Clicking prod_starter swaps the detail panel (client state, no nav).
    await userEvent.click(screen.getByTestId('mapping-product-row-prod_starter'))
    expect(screen.getByTestId('mapping-detail-panel')).toBeTruthy()
    // Its single price row renders.
    expect(screen.getByTestId('price-edit-row-price_once')).toBeTruthy()
  })

  it('renders a shared-key chip for grouped prices', async () => {
    renderPage([
      makeMapping({
        id: 'm-1',
        externalProductId: 'prod_pro',
        externalPriceId: 'price_monthly',
        entitlementKey: 'pro-plan',
      }),
      makeMapping({
        id: 'm-2',
        externalProductId: 'prod_pro',
        externalPriceId: 'price_annual',
        entitlementKey: 'pro-plan',
      }),
    ])
    expect(await screen.findByTestId('shared-key-chip-pro-plan')).toBeTruthy()
    // The "shared across N prices" hint shows count = 2 (asserted via the same
    // Paraglide message the production component renders, not a hardcoded en string).
    expect(screen.getByText(m['billing.shared_across_n_prices']({ count: 2 }))).toBeTruthy()
  })

  it('opens the protected-price confirmation dialog on a 409 from batch save', async () => {
    // Capture the onError callback by simulating the mutation throwing a 409
    // when save is invoked.
    mockBatchMutate.mockImplementation((_req: unknown, opts: { onError: (e: unknown) => void }) => {
      opts.onError({ code: 'mapping_in_use', activeSubscriptions: 28 })
    })

    renderPage([
      makeMapping({
        id: 'm-1',
        externalProductId: 'prod_pro',
        externalPriceId: 'price_team_monthly',
        entitlementKey: 'team',
        enabled: true,
        billingType: 'recurring',
        pointsPerPeriod: 1000,
      }),
    ])

    const save = await screen.findByTestId('save-mapping-button')
    await userEvent.click(save)

    await waitFor(() => {
      expect(screen.getByTestId('protected-price-confirm-dialog')).toBeTruthy()
    })
    // Active-subscription count surfaced.
    expect(screen.getByTestId('protected-price-active-subs').textContent).toContain('28')
  })

  it('renders the webhook-unresolved banner when an enabled synced row lacks billingType/points', async () => {
    // price_webhook_only: enabled + externalProductId set + no billingType/points → unresolved.
    renderPage([
      makeMapping({
        id: 'm-1',
        externalProductId: 'prod_x',
        externalPriceId: 'price_webhook_only',
        entitlementKey: 'unresolved-key',
        enabled: true,
        billingType: null,
        pointsPerPeriod: null,
      }),
    ])
    expect(await screen.findByTestId('webhook-price-unresolved-banner')).toBeTruthy()
    // The matching price row is present.
    expect(screen.getByTestId('price-edit-row-price_webhook_only')).toBeTruthy()
  })

  it('does not render the webhook-unresolved banner when all rows are configured', async () => {
    renderPage([
      makeMapping({
        id: 'm-1',
        externalProductId: 'prod_x',
        externalPriceId: 'price_ok',
        entitlementKey: 'ok',
        enabled: true,
        billingType: 'recurring',
        pointsPerPeriod: 100,
      }),
    ])
    await screen.findByTestId('mapping-detail-panel')
    expect(screen.queryByTestId('webhook-price-unresolved-banner')).toBeNull()
  })

  it('disables editor controls and shows the read-only banner without billing.manage', async () => {
    vi.mocked(usePermission).mockReturnValue({
      hasPermission: (p: string) => p !== 'billing.manage' && p !== 'points.manage',
    } as ReturnType<typeof usePermission>)

    renderPage([
      makeMapping({
        id: 'm-1',
        externalProductId: 'prod_pro',
        externalPriceId: 'price_monthly',
        entitlementKey: 'pro-plan',
        enabled: true,
        billingType: 'recurring',
        pointsPerPeriod: 1000,
      }),
    ])

    expect(await screen.findByTestId('readonly-perm-banner')).toBeTruthy()
    // No save button in read-only mode.
    expect(screen.queryByTestId('save-mapping-button')).toBeNull()
    // The enabled toggle is disabled.
    const toggle = screen.getByTestId('price-enabled-toggle-price_monthly')
    expect(toggle.hasAttribute('disabled')).toBe(true)
  })

  it('renders the empty-state CTA when no mappings are loaded', async () => {
    renderPage([])
    expect(await screen.findByTestId('entitlement-mappings-empty-state')).toBeTruthy()
    // Admin (billing.manage) sees the empty sync CTA.
    expect(screen.getByTestId('empty-sync-button')).toBeTruthy()
  })
})

// --- Pure helper coverage (re-exported surface for the test slot) ----------

describe('grouping helpers', () => {
  it('groupByProduct preserves first-seen order', async () => {
    const { groupByProduct } = await import('../entitlement-mapping-grouping')
    const items = [
      makeMapping({ id: '1', externalProductId: 'prod_b' }),
      makeMapping({ id: '2', externalProductId: 'prod_a' }),
      makeMapping({ id: '3', externalProductId: 'prod_b' }),
    ]
    const groups = groupByProduct(items)
    expect(groups.map((g) => g.externalProductId)).toEqual(['prod_b', 'prod_a'])
    expect(groups[0].prices).toHaveLength(2)
  })

  it('groupByEntitlementKey groups a product prices by key', async () => {
    const { groupByEntitlementKey } = await import('../entitlement-mapping-grouping')
    const prices = [
      makeMapping({ id: '1', entitlementKey: 'pro-plan' }),
      makeMapping({ id: '2', entitlementKey: 'pro-plan' }),
      makeMapping({ id: '3', entitlementKey: 'starter' }),
    ]
    const groups = groupByEntitlementKey(prices)
    expect(groups).toHaveLength(2)
    expect(groups[0].entitlementKey).toBe('pro-plan')
    expect(groups[0].prices).toHaveLength(2)
  })

  it('deriveSharedKeyColor is stable for the same key', async () => {
    const { deriveSharedKeyColor } = await import('../shared-key-color')
    const a = deriveSharedKeyColor('pro-plan')
    const b = deriveSharedKeyColor('pro-plan')
    const c = deriveSharedKeyColor('starter')
    expect(a.hue).toBe(b.hue)
    // The implementation guarantees stability, NOT uniqueness across keys, so
    // distinct keys may legitimately collide (see entitlement-mappings-helpers
    // test). Only assert the hue stays in the valid `[0, 360)` range.
    expect(c.hue).toBeGreaterThanOrEqual(0)
    expect(c.hue).toBeLessThan(360)
  })
})

// --- FE-T02: name-first primary label with i18n placeholder fallback -------

describe('EntitlementMappingsPage — primary label', () => {
  it('renders productName as the primary label when present (list row + detail head)', async () => {
    // snake_case JSONB: `readProviderProductInfo` narrows `name` → camelCase.
    renderPage([
      makeMapping({
        id: 'm-1',
        externalProductId: 'prod_pro',
        externalPriceId: 'price_monthly',
        entitlementKey: 'pro-plan',
        billingType: 'recurring',
        providerProductInfo: { name: 'Pro Plan' },
      }),
    ])

    // The product row label (id-keyed testid, unchanged) carries the synced name.
    const row = await screen.findByTestId('mapping-product-row-prod_pro')
    expect(row.textContent).toContain('Pro Plan')

    // The auto-selected detail panel head shows the SAME label.
    const head = await screen.findByTestId('detail-head')
    expect(head.textContent).toContain('Pro Plan')
  })

  it('falls back to externalProductId when productName is missing', async () => {
    // `providerProductInfo: null` → `readProviderProductInfo` returns `{}` → no
    // name → `primaryProductLabel` falls back to the externalProductId.
    renderPage([
      makeMapping({
        id: 'm-1',
        externalProductId: 'prod_pro',
        externalPriceId: 'price_monthly',
        entitlementKey: 'pro-plan',
        billingType: 'recurring',
        providerProductInfo: null,
      }),
    ])

    const row = await screen.findByTestId('mapping-product-row-prod_pro')
    expect(row.textContent).toContain('prod_pro')
    // And the detail head mirrors the fallback.
    const head = await screen.findByTestId('detail-head')
    expect(head.textContent).toContain('prod_pro')
  })
})

// --- FE-T02: read-only provider metadata block presence/absence -----------

describe('EntitlementMappingsPage — provider metadata block', () => {
  it('renders the metadata block when productMetadata has keys', async () => {
    // snake_case JSONB key `product_metadata` is narrowed to productMetadata.
    renderPage([
      makeMapping({
        id: 'm-1',
        externalProductId: 'prod_pro',
        externalPriceId: 'price_monthly',
        entitlementKey: 'pro-plan',
        billingType: 'recurring',
        providerProductInfo: { product_metadata: { tier: 'pro' } },
      }),
    ])

    expect(await screen.findByTestId('price-metadata-block-price_monthly')).toBeInTheDocument()
  })

  it('renders the metadata block when priceMetadata has keys', async () => {
    // `price_metadata` is price-scoped; OR of the two maps is sufficient.
    renderPage([
      makeMapping({
        id: 'm-1',
        externalProductId: 'prod_pro',
        externalPriceId: 'price_monthly',
        entitlementKey: 'pro-plan',
        billingType: 'recurring',
        providerProductInfo: { price_metadata: { interval: 'month' } },
      }),
    ])

    expect(await screen.findByTestId('price-metadata-block-price_monthly')).toBeInTheDocument()
  })

  it('omits the metadata block entirely when both metadata objects are empty', async () => {
    // Case A: no provider info at all.
    renderPage([
      makeMapping({
        id: 'm-1',
        externalProductId: 'prod_pro',
        externalPriceId: 'price_monthly_a',
        entitlementKey: 'pro-plan',
        billingType: 'recurring',
        providerProductInfo: null,
      }),
    ])
    await screen.findByTestId('price-edit-row-price_monthly_a')
    expect(screen.queryByTestId('price-metadata-block-price_monthly_a')).toBeNull()

    // Case B: info present but both metadata maps empty/null (no placeholder).
    renderPage([
      makeMapping({
        id: 'm-1',
        externalProductId: 'prod_pro',
        externalPriceId: 'price_monthly_b',
        entitlementKey: 'pro-plan',
        billingType: 'recurring',
        providerProductInfo: { product_metadata: null, price_metadata: {} },
      }),
    ])
    await screen.findByTestId('price-edit-row-price_monthly_b')
    expect(screen.queryByTestId('price-metadata-block-price_monthly_b')).toBeNull()
  })
})

// --- FE-T02: one_time field hiding (§4.5.4) --------------------------------

/**
 * Open the per-price "Advanced" panel for the seeded row so the lazily-mounted
 * advanced fields (Radix CollapsibleContent) appear. Returns the row scope.
 */
async function openAdvancedPanel(rowTestId: string) {
  await screen.findByTestId('mapping-product-row-prod_pro')
  await screen.findByTestId('mapping-detail-panel')
  const row = await screen.findByTestId(rowTestId)
  const advancedToggle = within(row).getByRole('button', { name: /Advanced/i })
  await userEvent.click(advancedToggle)
  return row
}

describe('EntitlementMappingsPage — one_time field hiding', () => {
  it('hides the four subscription-only fields for billingType one_time', async () => {
    renderPage([
      makeMapping({
        id: 'm-1',
        externalProductId: 'prod_pro',
        externalPriceId: 'price_once',
        entitlementKey: 'pro-plan',
        billingType: 'one_time',
        pointsPerPeriod: 500,
      }),
    ])

    const row = await openAdvancedPanel('price-edit-row-price_once')

    // The four subscription-only advanced fields are NOT rendered. The page's
    // `Field` wrapper renders the label text and control as siblings (no
    // htmlFor/id association), so presence is asserted via the rendered label
    // text rather than `getByLabelText`.
    expect(within(row).queryByText(m['billing.field_grant_period_type']())).toBeNull()
    expect(within(row).queryByText(m['billing.field_max_periods']())).toBeNull()
    expect(within(row).queryByText(m['billing.field_grant_on_subscribe']())).toBeNull()
    // quotaWindows renders the MultiWindowQuotaEditor (testid quota-window-editor).
    expect(within(row).queryByTestId('quota-window-editor')).toBeNull()

    // validityDays + pointsPerPeriod stay visible for one-time mappings.
    expect(within(row).getByText(m['billing.field_validity_days']())).toBeInTheDocument()
    // pointsPerPeriod is a top-level (non-advanced) field, always present.
    expect(within(row).getByText(m['billing.field_points_per_period']())).toBeInTheDocument()
  })

  it('renders the full field set for billingType recurring', async () => {
    renderPage([
      makeMapping({
        id: 'm-1',
        externalProductId: 'prod_pro',
        externalPriceId: 'price_monthly',
        entitlementKey: 'pro-plan',
        billingType: 'recurring',
        pointsPerPeriod: 500,
      }),
    ])

    const row = await openAdvancedPanel('price-edit-row-price_monthly')

    expect(within(row).getByText(m['billing.field_grant_period_type']())).toBeInTheDocument()
    expect(within(row).getByText(m['billing.field_max_periods']())).toBeInTheDocument()
    expect(within(row).getByText(m['billing.field_grant_on_subscribe']())).toBeInTheDocument()
    expect(within(row).getByTestId('quota-window-editor')).toBeInTheDocument()
  })

  it('renders the full field set for billingType null (recurring default)', async () => {
    renderPage([
      makeMapping({
        id: 'm-1',
        externalProductId: 'prod_pro',
        externalPriceId: 'price_unknown',
        entitlementKey: 'pro-plan',
        billingType: null,
        pointsPerPeriod: 500,
      }),
    ])

    const row = await openAdvancedPanel('price-edit-row-price_unknown')

    expect(within(row).getByText(m['billing.field_grant_period_type']())).toBeInTheDocument()
    expect(within(row).getByText(m['billing.field_max_periods']())).toBeInTheDocument()
    expect(within(row).getByText(m['billing.field_grant_on_subscribe']())).toBeInTheDocument()
    expect(within(row).getByTestId('quota-window-editor')).toBeInTheDocument()
  })
})
