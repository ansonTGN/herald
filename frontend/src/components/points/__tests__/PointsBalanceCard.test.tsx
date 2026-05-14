import { describe, it, expect, afterEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PointsBalanceCard } from '../PointsBalanceCard'
import {
  mockPointsAccount,
  mockPointsAccountFrozen,
  mockPointsAccountClosed,
} from '@/fixtures/points-account.fixture'

describe('PointsBalanceCard', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  describe('rendering', () => {
    it('GIVEN account with balance WHEN rendering THEN should display balance correctly', () => {
      render(<PointsBalanceCard account={mockPointsAccount} />)

      const balance = screen.getByTestId('points-balance')
      expect(balance).toBeInTheDocument()
      expect(balance).toHaveTextContent('5,000')
    })

    it('GIVEN account WHEN rendering THEN should display total recharged', () => {
      render(<PointsBalanceCard account={mockPointsAccount} />)

      const totalRecharged = screen.getByTestId('points-total-recharged')
      expect(totalRecharged).toBeInTheDocument()
      expect(totalRecharged).toHaveTextContent('10,000')
    })

    it('GIVEN account WHEN rendering THEN should display total consumed', () => {
      render(<PointsBalanceCard account={mockPointsAccount} />)

      const totalConsumed = screen.getByTestId('points-total-consumed')
      expect(totalConsumed).toBeInTheDocument()
      expect(totalConsumed).toHaveTextContent('5,000')
    })

    it('GIVEN account WHEN rendering THEN should display unit', () => {
      render(<PointsBalanceCard account={mockPointsAccount} />)

      expect(screen.getByText('points')).toBeInTheDocument()
    })

    it('GIVEN account WHEN rendering THEN should display card title', () => {
      render(<PointsBalanceCard account={mockPointsAccount} />)

      expect(screen.getByText('Points Balance')).toBeInTheDocument()
    })
  })

  describe('status display', () => {
    it('GIVEN account with active status WHEN rendering THEN should show active status', () => {
      render(<PointsBalanceCard account={mockPointsAccount} />)

      const status = screen.getByTestId('points-account-status')
      expect(status).toBeInTheDocument()
      expect(status).toHaveTextContent('Active')
      expect(status).toHaveClass(/text-green-600/)
      expect(status).toHaveClass(/bg-green-50/)
    })

    it('GIVEN account with frozen status WHEN rendering THEN should show frozen status', () => {
      render(<PointsBalanceCard account={mockPointsAccountFrozen} />)

      const status = screen.getByTestId('points-account-status')
      expect(status).toBeInTheDocument()
      expect(status).toHaveTextContent('Frozen')
      expect(status).toHaveClass(/text-red-600/)
      expect(status).toHaveClass(/bg-red-50/)
    })

    it('GIVEN account with closed status WHEN rendering THEN should show closed status', () => {
      render(<PointsBalanceCard account={mockPointsAccountClosed} />)

      const status = screen.getByTestId('points-account-status')
      expect(status).toBeInTheDocument()
      expect(status).toHaveTextContent('Closed')
      expect(status).toHaveClass(/text-gray-600/)
      expect(status).toHaveClass(/bg-gray-50/)
    })

    it('GIVEN account with unknown status WHEN rendering THEN should show unknown status', () => {
      const accountWithUnknownStatus = {
        ...mockPointsAccount,
        status: 'unknown' as any,
      }
      render(<PointsBalanceCard account={accountWithUnknownStatus} />)

      const status = screen.getByTestId('points-account-status')
      expect(status).toBeInTheDocument()
      expect(status).toHaveTextContent('Unknown')
      expect(status).toHaveClass(/text-yellow-600/)
      expect(status).toHaveClass(/bg-yellow-50/)
    })
  })

  describe('loading state', () => {
    it('GIVEN loading is true WHEN rendering THEN should show loading skeleton', () => {
      render(<PointsBalanceCard account={mockPointsAccount} loading={true} />)

      const card = screen.getByTestId('points-balance-card')
      expect(card).toBeInTheDocument()
      expect(card).toHaveTextContent('Points Balance')

      const balance = screen.queryByTestId('points-balance')
      expect(balance).not.toBeInTheDocument()
    })

    it('GIVEN loading is true WHEN rendering THEN should show skeleton elements', () => {
      render(<PointsBalanceCard account={mockPointsAccount} loading={true} />)

      // Check for skeleton classes
      const skeletonElements = document.querySelectorAll('.animate-pulse')
      expect(skeletonElements.length).toBeGreaterThan(0)
    })
  })

  describe('null account', () => {
    it('GIVEN account is null WHEN rendering THEN should return null', () => {
      const { container } = render(<PointsBalanceCard account={null} />)

      expect(container.firstChild).toBeNull()
    })

    it('GIVEN account is null and loading is false WHEN rendering THEN should not render', () => {
      render(<PointsBalanceCard account={null} loading={false} />)

      const card = screen.queryByTestId('points-balance-card')
      expect(card).not.toBeInTheDocument()
    })
  })

  describe('large numbers', () => {
    it('GIVEN account with large balance WHEN rendering THEN should format with commas', () => {
      const largeAccount = {
        ...mockPointsAccount,
        balance: 1000000,
        totalRecharged: 5000000,
        totalConsumed: 4000000,
      }
      render(<PointsBalanceCard account={largeAccount} />)

      const balance = screen.getByTestId('points-balance')
      expect(balance).toHaveTextContent('1,000,000')

      const totalRecharged = screen.getByTestId('points-total-recharged')
      expect(totalRecharged).toHaveTextContent('5,000,000')

      const totalConsumed = screen.getByTestId('points-total-consumed')
      expect(totalConsumed).toHaveTextContent('4,000,000')
    })
  })
})
