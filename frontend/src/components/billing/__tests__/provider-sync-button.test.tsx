import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

// --- Mocks ----------------------------------------------------------------

const { providersHolder, syncMutate } = vi.hoisted(() => ({
  // Configured payment providers per test; shape mirrors the slim
  // listPaymentProviders payload the component consumes (`platform` only).
  providersHolder: { current: [] as Array<{ platform: string }> },
  syncMutate: vi.fn(),
}))

vi.mock('@/data/query-options', () => ({
  paymentProvidersQueryOptions: () => ({
    queryKey: ['payment-providers', 'realm-1'],
    queryFn: async () => providersHolder.current,
  }),
}))

vi.mock('@/data/entitlement-mapping-mutations', () => ({
  useSyncProviderProducts: () => ({
    mutate: syncMutate,
    isPending: false,
  }),
}))

import { ProviderSyncButton } from '../provider-sync-button'
import { m } from '@/paraglide/messages'

// --- Fixtures -------------------------------------------------------------

function renderSyncButton(onSyncComplete = vi.fn()) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  return render(
    <QueryClientProvider client={client}>
      <ProviderSyncButton realmId="realm-1" onSyncComplete={onSyncComplete} />
    </QueryClientProvider>
  )
}

/**
 * Wait for the providers query to settle before asserting. While the query is
 * in flight the component renders its loading fallback (a disabled generic
 * sync button carrying the same `sync-button` testid) — assertions made
 * synchronously would test the placeholder, not the configured state.
 */
async function settledButtonWithProvider(provider: string) {
  return waitFor(() => {
    // One `sync-button` is rendered per configured provider (same testid,
    // distinguished by `data-provider`).
    const el = screen
      .getAllByTestId('sync-button')
      .find((b) => b.getAttribute('data-provider') === provider)
    expect(el).toBeDefined()
    expect(el).not.toBeDisabled()
    return el as HTMLElement
  })
}

beforeEach(() => {
  vi.clearAllMocks()
  providersHolder.current = []
})

// --- Tests ----------------------------------------------------------------
//
// Intent: product sync only applies to providers with a hosted catalogue
// (Stripe / Creem). WeChat Pay is order-based with no catalogue
// (DEC-wechat-support-006) and in-app purchase stores are never synced — for
// those realms a "Sync provider" affordance is meaningless, and a disabled
// one plus a "configure a provider first" hint actively misleads (the
// reported ambiguity: an admin with WeChat configured reads the hint as
// "WeChat should be syncable but something is broken").

describe('ProviderSyncButton — per-configuration states', () => {
  it('renders a per-provider sync button for each configured catalogue provider', async () => {
    providersHolder.current = [{ platform: 'stripe' }, { platform: 'creem' }]
    const user = userEvent.setup()
    const onSyncComplete = vi.fn()
    renderSyncButton(onSyncComplete)

    const stripe = await settledButtonWithProvider('stripe')
    expect(stripe).toHaveTextContent(m['billing.sync_provider_with_name']({ name: 'Stripe' }))
    expect(screen.getByTestId('provider-sync-button').textContent).toContain('Creem')

    // Clicking drives the sync mutation; its success callback fires
    // onSyncComplete (the page's refetch + next-step guidance hook).
    syncMutate.mockImplementation((_body: unknown, opts: { onSuccess?: () => void }) =>
      opts.onSuccess?.()
    )
    await user.click(stripe)
    expect(syncMutate).toHaveBeenCalledWith(
      { paymentProvider: 'stripe' },
      expect.objectContaining({ onSuccess: expect.any(Function) })
    )
    expect(onSyncComplete).toHaveBeenCalled()
  })

  it.each([
    ['wechat only', [{ platform: 'wechat' }]],
    ['wechat + in-app purchase stores', [{ platform: 'wechat' }, { platform: 'apple' }]],
  ])(
    'renders NO sync affordance when only non-syncable providers are configured (%s)',
    async (_label, providers) => {
      providersHolder.current = providers
      renderSyncButton()

      // Wait out the loading fallback, then nothing may remain: sync does not
      // apply to WeChat / IAP, and a disabled button + "configure a provider
      // first" hint would read as broken while providers ARE configured.
      await waitFor(() => {
        expect(screen.queryByTestId('sync-button')).toBeNull()
      })
      expect(screen.queryByTestId('provider-sync-button')).toBeNull()
    }
  )

  it('renders a single disabled sync button with the configure-first hint when nothing is configured', async () => {
    renderSyncButton()

    const button = await screen.findByTestId('sync-button')
    expect(button).toBeDisabled()
    // The hint names the non-syncable providers (WeChat + IAP) so an admin
    // configuring them next isn't surprised by the button staying absent.
    expect(m['billing.sync_provider_none_configured_hint']()).toContain('WeChat Pay')
  })

  it('syncs only catalogue providers when a non-syncable provider is also configured', async () => {
    providersHolder.current = [{ platform: 'wechat' }, { platform: 'stripe' }]
    renderSyncButton()

    // WeChat gets no button even though it IS configured — only Stripe does.
    await settledButtonWithProvider('stripe')
    const buttons = screen.getAllByTestId('sync-button')
    expect(buttons).toHaveLength(1)
  })
})
