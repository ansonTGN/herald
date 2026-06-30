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
    // Pool-only bucket default: no quota windows (design §4.2.2 — null/undefined
    // for pool-only). `PointsBalanceCard` is pool-only by design (FE-D04): the
    // big number is `spendableFromPool`, and only topup/registration/granted
    // type badges render (subscription/freePeriodic moved to the dashboard).
    quotaWindows: undefined,
    spendableFromQuota: undefined,
    spendableFromPool: 0,
    ...overrides,
  }
}

// ==================== rendering ====================

describe('PointsBalanceCard', () => {
  describe('rendering', () => {
    it('GIVEN a bucket with a pool balance WHEN rendering THEN should display the formatted pool total (spendableFromPool), not bucketTotal', () => {
      // INTENT (FE-D04 pool-only contract): the big number is the pool-side
      // balance only — subscription/free-periodic entitlements now surface via
      // PointsUsageDashboard windows, so this card must NOT show the combined
      // `bucketTotal`. Asserting spendableFromPool (5000) while bucketTotal is
      // a deliberately larger 9999 pins that the card ignores bucketTotal.
      render(<PointsBalanceCard card={makeCard({ bucketTotal: 9999, spendableFromPool: 5000 })} />)

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
    it('GIVEN non-zero pool balances WHEN rendering THEN shows a badge per non-zero pool type (topup/registration/granted) and never renders subscription/freePeriodic', () => {
      // INTENT (FE-D04 pool-only contract): only the pool model types render
      // here. subscription/freePeriodic are quota-window concerns now and MUST
      // NOT appear on this card even when their balances are non-zero — a
      // stale subscription badge would mislead users into thinking the pool
      // card still tracks subscription entitlement. We seed non-zero
      // subscription/freePeriodic to pin that they are intentionally dropped.
      render(
        <PointsBalanceCard
          card={makeCard({
            balancesByType: {
              subscription: 3000,
              topup: 1200,
              registration: 0,
              freePeriodic: 700,
              granted: 50,
            },
          })}
        />
      )

      // Pool types with non-zero balances render.
      expect(screen.getByTestId('points-balance-type-bucket-1-topup')).toBeInTheDocument()
      expect(screen.getByTestId('points-balance-type-bucket-1-granted')).toBeInTheDocument()
      // A zero pool balance type is omitted — indistinguishable from "not present".
      expect(
        screen.queryByTestId('points-balance-type-bucket-1-registration')
      ).not.toBeInTheDocument()
      // subscription/freePeriodic are no longer this card's concern (quota model).
      expect(
        screen.queryByTestId('points-balance-type-bucket-1-subscription')
      ).not.toBeInTheDocument()
      expect(
        screen.queryByTestId('points-balance-type-bucket-1-freePeriodic')
      ).not.toBeInTheDocument()
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
