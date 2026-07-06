/**
 * @vitest-environment jsdom
 */

import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ProfileSidebar } from '../profile-sidebar'
import type { ReactNode } from 'react'
import { LocaleProvider } from '@/components/shared/locale-provider'

let featureData = {
  user: {
    pointsVisible: false,
    subscriptionVisible: false,
    invoicesVisible: false,
  },
}

vi.mock('@tanstack/react-query', () => ({
  useQuery: () => ({ data: featureData }),
}))

vi.mock('@tanstack/react-router', () => ({
  Link: ({ to, children, ...props }: { to: string; children: ReactNode }) => (
    <a href={to} {...props}>
      {children}
    </a>
  ),
  useLocation: () => ({ pathname: '/test-realm/user/profile' }),
}))

vi.mock('@/stores/auth-store', () => ({
  useRealmId: () => 'test-realm',
}))

vi.mock('@/lib/auth-utils', () => ({
  logoutFlow: vi.fn(),
}))

vi.mock('@/data/query-options', () => ({
  featureAvailabilityQueryOptions: () => ({ queryKey: ['feature-availability', 'test-realm'] }),
}))

describe('ProfileSidebar', () => {
  it('shows points and purchase records together when points area is available', () => {
    // After the gate merge, `pointsVisible` drives both the Points and the
    // PurchaseRecords entries — they belong to the same points area and no
    // longer gate independently.
    featureData = {
      user: {
        pointsVisible: true,
        subscriptionVisible: false,
        invoicesVisible: false,
      },
    }

    render(
      <LocaleProvider>
        <ProfileSidebar />
      </LocaleProvider>
    )

    expect(screen.getByTestId('profile-menu-profile')).toBeInTheDocument()
    expect(screen.getByTestId('profile-menu-security')).toBeInTheDocument()
    expect(screen.getByTestId('profile-menu-points')).toBeInTheDocument()
    expect(screen.getByTestId('profile-menu-purchaserecords')).toBeInTheDocument()
    expect(screen.queryByTestId('profile-menu-subscription')).not.toBeInTheDocument()
    expect(screen.queryByTestId('profile-menu-invoices')).not.toBeInTheDocument()
  })

  it('shows invoices when invoice features are available', () => {
    featureData = {
      user: {
        pointsVisible: false,
        subscriptionVisible: false,
        invoicesVisible: true,
      },
    }

    render(
      <LocaleProvider>
        <ProfileSidebar />
      </LocaleProvider>
    )

    expect(screen.getByTestId('profile-menu-invoices')).toBeInTheDocument()
  })

  it('does not show points/purchase records when points area is hidden (e.g. subscription-only realm with no enabled mappings on the points axis)', () => {
    featureData = {
      user: {
        pointsVisible: false,
        subscriptionVisible: true,
        invoicesVisible: false,
      },
    }

    render(
      <LocaleProvider>
        <ProfileSidebar />
      </LocaleProvider>
    )

    expect(screen.queryByTestId('profile-menu-purchaserecords')).not.toBeInTheDocument()
    expect(screen.queryByTestId('profile-menu-points')).not.toBeInTheDocument()
  })
})
