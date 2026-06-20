import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PointsBalanceCard } from '../PointsBalanceCard'
import type { DerivedBucketCard } from '../user-points-view'

// ---------- Factory helpers ----------

function makeCard(overrides: Partial<DerivedBucketCard> = {}): DerivedBucketCard {
  return {
    bucketId: 'bucket-1',
    name: 'Default Bucket',
    enabled: true,
    bucketTotal: 5000,
    balancesByType: {
      subscription: 0,
      topup: 0,
      registration: 0,
      freePeriodic: 0,
      granted: 0,
    },
    ...overrides,
  }
}

// ==================== rendering ====================

describe('PointsBalanceCard', () => {
  describe('rendering', () => {
    it('GIVEN a bucket with a balance WHEN rendering THEN should display the formatted total', () => {
      render(<PointsBalanceCard card={makeCard({ bucketTotal: 5000 })} />)

      const total = screen.getByTestId('points-balance-total-bucket-1')
      expect(total).toBeInTheDocument()
      expect(total).toHaveTextContent('5,000')
    })

    it('GIVEN a bucket without a bucketId WHEN rendering THEN should use the generic card testid', () => {
      render(<PointsBalanceCard card={makeCard({ bucketId: '' })} />)

      expect(screen.getByTestId('points-balance-card')).toBeInTheDocument()
    })
  })

  describe('per-type balances', () => {
    it('GIVEN non-zero balances WHEN rendering THEN should show a badge per non-zero type and hide zero types', () => {
      render(
        <PointsBalanceCard
          card={makeCard({
            balancesByType: {
              subscription: 3000,
              topup: 0,
              registration: 0,
              freePeriodic: 0,
              granted: 50,
            },
          })}
        />
      )

      expect(screen.getByTestId('points-balance-type-bucket-1-subscription')).toBeInTheDocument()
      expect(screen.getByTestId('points-balance-type-bucket-1-granted')).toBeInTheDocument()
      // A zero balance type is omitted — indistinguishable from "not present"
      expect(screen.queryByTestId('points-balance-type-bucket-1-topup')).not.toBeInTheDocument()
    })
  })

  describe('disabled state', () => {
    it('GIVEN an enabled bucket WHEN rendering THEN should not show the disabled badge', () => {
      render(<PointsBalanceCard card={makeCard({ enabled: true })} />)

      expect(screen.queryByTestId('points-balance-card-disabled-bucket-1')).not.toBeInTheDocument()
    })

    it('GIVEN a disabled bucket WHEN rendering THEN should show the disabled badge', () => {
      render(<PointsBalanceCard card={makeCard({ enabled: false })} />)

      expect(screen.getByTestId('points-balance-card-disabled-bucket-1')).toBeInTheDocument()
    })
  })

  describe('loading state', () => {
    it('GIVEN loading is true WHEN rendering THEN should show the skeleton and no balance', () => {
      render(<PointsBalanceCard card={makeCard()} loading />)

      expect(screen.getByTestId('points-balance-card')).toBeInTheDocument()
      expect(screen.queryByTestId('points-balance-total-bucket-1')).not.toBeInTheDocument()
    })
  })
})
