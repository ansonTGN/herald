import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ProfileHeader } from '../profile-header'

// Mock useRealmId
vi.mock('@/stores/auth-store', () => ({
  useRealmId: vi.fn(),
}))

import { useRealmId } from '@/stores/auth-store'

describe('ProfileHeader', () => {
  const mockRealmId = 'test-realm'

  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(useRealmId).mockReturnValue(mockRealmId)
  })

  it('GIVEN ProfileHeader component WHEN rendering THEN displays header with correct testids', async () => {
    const screen = render(<ProfileHeader />)

    // Verify header container
    expect(screen.getByTestId('profile-header')).toBeInTheDocument()

    // Verify heading element
    expect(screen.getByTestId('profile-heading')).toBeInTheDocument()

    // Verify heading text includes realm ID
    expect(screen.getByText(`${mockRealmId} - Profile`)).toBeInTheDocument()
  })

  it('GIVEN ProfileHeader WHEN rendering THEN has correct layout structure', async () => {
    const { container } = render(<ProfileHeader />)

    // Verify header container has correct classes
    const header = container.querySelector('[data-testid="profile-header"]')
    expect(header).toBeInTheDocument()
    expect(header).toHaveClass('bg-white', 'border-b', 'border-gray-200', 'px-6', 'py-4')

    // Verify heading has correct classes
    const heading = container.querySelector('[data-testid="profile-heading"]')
    expect(heading).toBeInTheDocument()
    expect(heading).toHaveClass('text-2xl', 'font-bold', 'text-gray-900')
  })

  it('GIVEN different realm ID WHEN rendering THEN displays correct realm ID in heading', async () => {
    const differentRealmId = 'my-app'
    vi.mocked(useRealmId).mockReturnValue(differentRealmId)

    const screen = render(<ProfileHeader />)

    // Verify heading text includes the new realm ID
    expect(screen.getByText(`${differentRealmId} - Profile`)).toBeInTheDocument()
  })
})
