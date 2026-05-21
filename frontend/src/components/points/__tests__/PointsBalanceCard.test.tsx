import { describe, it, expect, afterEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PointsBalanceCard } from '../PointsBalanceCard'
import {
  mockPointsWallet,
  mockPointsWalletFrozen,
  mockPointsWalletClosed,
} from '@/fixtures/points-wallet.fixture'

describe('PointsBalanceCard', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('rendering', () => {
    it('GIVEN account with balance WHEN rendering THEN should display balance correctly', () => {
      render(<PointsBalanceCard account={mockPointsWallet} />)

      const balance = screen.getByTestId('points-balance')
      expect(balance).toBeInTheDocument()
      expect(balance).toHaveTextContent('5,000')
    })
  })

  describe('status display', () => {
    it('GIVEN account with active status WHEN rendering THEN should show active status', () => {
      render(<PointsBalanceCard account={mockPointsWallet} />)

      const status = screen.getByTestId('points-wallet-status')
      expect(status).toBeInTheDocument()
      expect(status).toHaveTextContent('Active')
      expect(status).toHaveClass(/text-green-600/)
      expect(status).toHaveClass(/bg-green-50/)
    })

    it('GIVEN account with frozen status WHEN rendering THEN should show frozen status', () => {
      render(<PointsBalanceCard account={mockPointsWalletFrozen} />)

      const status = screen.getByTestId('points-wallet-status')
      expect(status).toBeInTheDocument()
      expect(status).toHaveTextContent('Frozen')
      expect(status).toHaveClass(/text-red-600/)
      expect(status).toHaveClass(/bg-red-50/)
    })

    it('GIVEN account with closed status WHEN rendering THEN should show closed status', () => {
      render(<PointsBalanceCard account={mockPointsWalletClosed} />)

      const status = screen.getByTestId('points-wallet-status')
      expect(status).toBeInTheDocument()
      expect(status).toHaveTextContent('Closed')
      expect(status).toHaveClass(/text-gray-600/)
      expect(status).toHaveClass(/bg-gray-50/)
    })

    it('GIVEN account with unknown status WHEN rendering THEN should show unknown status', () => {
      const accountWithUnknownStatus = {
        ...mockPointsWallet,
        status: 'unknown' as any,
      }
      render(<PointsBalanceCard account={accountWithUnknownStatus} />)

      const status = screen.getByTestId('points-wallet-status')
      expect(status).toBeInTheDocument()
      expect(status).toHaveTextContent('Unknown')
      expect(status).toHaveClass(/text-yellow-600/)
      expect(status).toHaveClass(/bg-yellow-50/)
    })
  })

  describe('loading state', () => {
    it('GIVEN loading is true WHEN rendering THEN should show loading skeleton', () => {
      render(<PointsBalanceCard account={mockPointsWallet} loading={true} />)

      const balance = screen.queryByTestId('points-balance')
      expect(balance).not.toBeInTheDocument()
    })
  })

  describe('null account', () => {
    it('GIVEN account is null WHEN rendering THEN should return null', () => {
      const { container } = render(<PointsBalanceCard account={null} />)

      expect(container.firstChild).toBeNull()
    })
  })
})
