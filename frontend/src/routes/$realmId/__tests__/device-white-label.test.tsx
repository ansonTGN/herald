import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

vi.mock('@/data/query-options', () => ({
  publicConfigQueryOptions: (realmId: string) => ({
    queryKey: ['public-config', realmId],
    queryFn: async () => ({
      realmName: 'Device Realm',
      whiteLabel: {
        brandName: 'Device Brand',
        logoUrl: 'https://cdn.example.com/device.svg',
      },
    }),
  }),
}))

import { DeviceVerificationIndexPage } from '../device.index'

function renderPage() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <DeviceVerificationIndexPage />
    </QueryClientProvider>
  )
}

describe('device route white-label integration', () => {
  beforeEach(() => {
    window.history.replaceState({}, '', '/test-realm/device/')
  })

  it('loads the realm public config and renders its logo at the route entry', async () => {
    renderPage()
    expect(await screen.findByTestId('auth-brand-logo')).toHaveAttribute(
      'src',
      'https://cdn.example.com/device.svg'
    )
    expect(document.title).toBe('Device Brand')
  })
})
