import { describe, it, expect } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import type { ReactNode } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { server } from '@/test/mocks/server'
import { ProfileIndex } from '../profile'

// Wire contracts for the preferred-currency override on the profile page.
// The PUT body is tri-state: `preferredCurrency: "<CODE>"` sets the override,
// `preferredCurrency: null` clears it, and `nickname` must stay absent so the
// nickname is never clobbered by a currency-only save. Bodies are captured at
// the MSW boundary rather than by mocking the generated client, per testing.md.

const PROFILE_URL = 'http://localhost:3000/api/user/profile'

function makeProfile(overrides: Record<string, unknown> = {}) {
  return {
    id: 'user-1',
    email: 'user@example.com',
    nickname: 'Nick',
    preferredCurrency: null,
    status: 1,
    ...overrides,
  }
}

let currentProfile = makeProfile()
let putCalls: (Record<string, unknown> | undefined)[] = []

function seedProfileHandlers() {
  server.use(
    http.get(PROFILE_URL, () => HttpResponse.json(currentProfile)),
    http.put(PROFILE_URL, async ({ request }) => {
      const body = (await request.json()) as Record<string, unknown> | undefined
      putCalls.push(body)
      if (body && typeof body.preferredCurrency === 'string') {
        currentProfile = { ...currentProfile, preferredCurrency: body.preferredCurrency }
      } else if (body && body.preferredCurrency === null) {
        currentProfile = { ...currentProfile, preferredCurrency: null }
      }
      return HttpResponse.json(currentProfile)
    })
  )
}

function renderPage() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  return render(
    <QueryClientProvider client={client}>
      <ProfileIndex />
    </QueryClientProvider>
  )
}

function setup(overrides: Record<string, unknown> = {}) {
  currentProfile = makeProfile(overrides)
  putCalls = []
  seedProfileHandlers()
  return renderPage()
}

async function renderLoaded(overrides: Record<string, unknown> = {}) {
  const view = setup(overrides)
  await screen.findByTestId('preferred-currency-input')
  return view
}

describe('profile preferred currency', () => {
  it('shows the current override', async () => {
    await renderLoaded({ preferredCurrency: 'CNY' })
    expect(screen.getByTestId('preferred-currency-current')).toHaveTextContent('CNY')
  })

  it('shows the not-set fallback when no override exists', async () => {
    await renderLoaded({ preferredCurrency: null })
    expect(screen.getByTestId('preferred-currency-current')).toHaveTextContent(
      'Not set (realm default applies)'
    )
  })

  it('normalizes a lowercase entry to the uppercase ISO code on save', async () => {
    const user = userEvent.setup()
    await renderLoaded({ preferredCurrency: null })

    await user.type(screen.getByTestId('preferred-currency-input'), 'eur')
    await user.click(screen.getByTestId('preferred-currency-save'))

    await waitFor(() => {
      expect(putCalls).toHaveLength(1)
    })
    expect(putCalls[0]).toEqual({ preferredCurrency: 'EUR' })
    // nickname is absent on the wire — a currency-only save must not touch it.
    expect(putCalls[0]).not.toHaveProperty('nickname')
  })

  it('clears the override with an explicit null (not an absent key)', async () => {
    const user = userEvent.setup()
    await renderLoaded({ preferredCurrency: 'CNY' })

    await user.click(screen.getByTestId('preferred-currency-clear'))

    await waitFor(() => {
      expect(putCalls).toHaveLength(1)
    })
    // null must be present as a key: an absent key would mean "leave
    // unchanged" under the tri-state contract and the override would survive.
    expect(putCalls[0]).toEqual({ preferredCurrency: null })
  })

  it('rejects an invalid code at the form and sends no request', async () => {
    const user = userEvent.setup()
    await renderLoaded({ preferredCurrency: null })

    await user.type(screen.getByTestId('preferred-currency-input'), 'XXX')
    await user.click(screen.getByTestId('preferred-currency-save'))

    expect(await screen.findByTestId('preferred-currency-error')).toBeInTheDocument()
    expect(putCalls).toHaveLength(0)
  })

  it('keeps Clear disabled while no override exists', async () => {
    await renderLoaded({ preferredCurrency: null })
    expect(screen.getByTestId('preferred-currency-clear')).toBeDisabled()
  })
})
