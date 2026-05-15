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

    expect(screen.getByTestId('profile-header')).toBeInTheDocument()
    expect(screen.getByTestId('profile-heading')).toBeInTheDocument()
    expect(screen.getByText(`${mockRealmId} - Profile`)).toBeInTheDocument()
  })
})
