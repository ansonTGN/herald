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
  it('shows profile sections only when backend feature availability marks them visible', () => {
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

    expect(screen.getByTestId('profile-menu-profile')).toBeInTheDocument()
    expect(screen.getByTestId('profile-menu-security')).toBeInTheDocument()
    expect(screen.queryByTestId('profile-menu-points')).not.toBeInTheDocument()
    expect(screen.getByTestId('profile-menu-subscription')).toBeInTheDocument()
    expect(screen.queryByTestId('profile-menu-invoices')).not.toBeInTheDocument()
  })
})
