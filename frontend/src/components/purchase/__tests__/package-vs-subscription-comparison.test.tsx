import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PackageVsSubscriptionComparison } from '../package-vs-subscription-comparison'
import type { PointsPackageResponse } from '@/lib/api-generated'
import type { SubscriptionPlanResponse } from '@/lib/api-generated'

describe('PackageVsSubscriptionComparison', () => {
  const mockPackages: PointsPackageResponse[] = [
    {
      id: 'pkg-1',
      name: 'starter_pack',
      title: 'Starter Pack',
      points: 1000,
      price: 999, // API price in cents (9.99 USD)
      currency: 'USD',
      enabled: true,
      createdAt: '2025-01-01T00:00:00Z',
      updatedAt: '2025-01-01T00:00:00Z',
      realmId: 'realm-1',
    },
    {
      id: 'pkg-2',
      name: 'premium_pack',
      title: 'Premium Pack',
      points: 5000,
      price: 3999, // API price in cents (39.99 USD)
      currency: 'USD',
      enabled: true,
      createdAt: '2025-01-01T00:00:00Z',
      updatedAt: '2025-01-01T00:00:00Z',
      realmId: 'realm-1',
    },
  ]

  const mockPlans: SubscriptionPlanResponse[] = [
    {
      id: 'plan-1',
      name: 'basic_monthly',
      title: 'Basic Monthly',
      price: 19.99,
      currency: 'USD',
      active: true,
      productId: 'prod-1',
      createdAt: '2025-01-01T00:00:00Z',
      updatedAt: '2025-01-01T00:00:00Z',
      realmId: 'realm-1',
    },
    {
      id: 'plan-2',
      name: 'premium_yearly',
      title: 'Premium Yearly',
      price: 199.99,
      currency: 'USD',
      active: true,
      productId: 'prod-1',
      createdAt: '2025-01-01T00:00:00Z',
      updatedAt: '2025-01-01T00:00:00Z',
      realmId: 'realm-1',
    },
  ]

  describe('rendering', () => {
    it('should render comparison component', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={mockPlans} />)

      expect(screen.getByTestId('package-vs-subscription-comparison')).toBeInTheDocument()
    })

    it('should render recommendation banner', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={mockPlans} />)

      // Check for some recommendation text
      expect(
        screen.getByText(/Compare your options|Better Value|Both Great Options/i)
      ).toBeInTheDocument()
    })

    it('should render comparison table headers', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={mockPlans} />)

      expect(screen.getByText('Feature')).toBeInTheDocument()
      // Use getAllByText since there are multiple "Points Package" and "Subscription" texts
      expect(screen.getAllByText('Points Package').length).toBeGreaterThan(0)
      expect(screen.getAllByText('Subscription').length).toBeGreaterThan(0)
    })

    it('should render cheapest package card', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={mockPlans} />)

      expect(screen.getByTestId('cheapest-package-card')).toBeInTheDocument()
      expect(screen.getByText('1,000')).toBeInTheDocument() // Cheapest package points
      // formatPrice returns "9.99 USD" when API price is 999 cents
      // Use getAllByText since "9.99 USD" appears in both the table and the card
      expect(screen.getAllByText('9.99 USD').length).toBeGreaterThan(0)
    })

    it('should render cheapest plan card', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={mockPlans} />)

      expect(screen.getByTestId('cheapest-plan-card')).toBeInTheDocument()
      expect(screen.getByText('Recurring')).toBeInTheDocument()
      // Plan prices are shown as "$19.99/mo"
      expect(screen.getByText(/\$19\.99\/mo/)).toBeInTheDocument()
    })

    it('should display N/A when no packages available', () => {
      render(<PackageVsSubscriptionComparison packages={[]} plans={mockPlans} />)

      expect(screen.getAllByText('N/A').length).toBeGreaterThan(0)
    })

    it('should display N/A when no plans available', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={[]} />)

      expect(screen.getAllByText('N/A').length).toBeGreaterThan(0)
    })

    it('should show "No packages available" message', () => {
      render(<PackageVsSubscriptionComparison packages={[]} plans={mockPlans} />)

      expect(screen.getByText('No packages available')).toBeInTheDocument()
    })

    it('should show "No plans available" message', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={[]} />)

      expect(screen.getByText('No plans available')).toBeInTheDocument()
    })
  })

  describe('recommendation logic', () => {
    it('should show info recommendation when both are missing', () => {
      render(<PackageVsSubscriptionComparison packages={[]} plans={[]} />)

      expect(screen.getByText('Compare your options')).toBeInTheDocument()
    })

    it('should show some recommendation text when both are available', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={mockPlans} />)

      // Just check that some recommendation is shown
      expect(
        screen.getByText(/Compare your options|Better Value|Both Great Options/i)
      ).toBeInTheDocument()
    })
  })

  describe('value calculations', () => {
    it('should calculate points per dollar correctly', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={mockPlans} />)

      // 1000 points / 9.99 (display price from 999 cents) = 100.1 points/$
      // The component displays: {calculatePointsPerDollar(points, apiPriceToDisplayPrice(price, currency)).toFixed(1)} points/$
      // 1000 / 9.99 = 100.1001001... -> toFixed(1) = "100.1"
      const valueText = screen.getByText((content, element) => {
        return (
          content !== null &&
          element?.textContent !== null &&
          /^100\.1 points\/\$/.test(element.textContent)
        )
      })
      expect(valueText).toBeInTheDocument()
    })

    it('should display value calculation in package card', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={mockPlans} />)

      expect(screen.getByText('Value:')).toBeInTheDocument()
    })
  })

  describe('comparison table content', () => {
    it('should show payment type comparison', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={mockPlans} />)

      expect(screen.getByText('Payment Type')).toBeInTheDocument()
      expect(screen.getByText('One-time Payment')).toBeInTheDocument()
      expect(screen.getByText('Recurring Payment')).toBeInTheDocument()
    })

    it('should show points delivery comparison', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={mockPlans} />)

      expect(screen.getByText('Points Delivery')).toBeInTheDocument()
      expect(screen.getByText('Immediate')).toBeInTheDocument()
      expect(screen.getByText('Periodic (Monthly/Weekly)')).toBeInTheDocument()
    })

    it('should show cost flexibility comparison', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={mockPlans} />)

      expect(screen.getByText('Cost Flexibility')).toBeInTheDocument()
      expect(screen.getByText('Pay as you need')).toBeInTheDocument()
      expect(screen.getByText('Fixed commitment')).toBeInTheDocument()
    })

    it('should show cancellation comparison', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={mockPlans} />)

      expect(screen.getByText('Cancellation')).toBeInTheDocument()
      expect(screen.getAllByText('Not applicable').length).toBeGreaterThan(0)
      expect(screen.getAllByText('Cancel anytime').length).toBeGreaterThan(0)
    })
  })

  describe('interaction handlers', () => {
    it('should call onPackageClick when package card clicked', () => {
      const onPackageClick = vi.fn()

      render(
        <PackageVsSubscriptionComparison
          packages={mockPackages}
          plans={mockPlans}
          onPackageClick={onPackageClick}
        />
      )

      const packageCard = screen.getByTestId('cheapest-package-card')
      packageCard.click()

      expect(onPackageClick).toHaveBeenCalledWith(mockPackages[0]) // Cheapest package
    })

    it('should call onPlanClick when plan card clicked', () => {
      const onPlanClick = vi.fn()

      render(
        <PackageVsSubscriptionComparison
          packages={mockPackages}
          plans={mockPlans}
          onPlanClick={onPlanClick}
        />
      )

      const planCard = screen.getByTestId('cheapest-plan-card')
      planCard.click()

      expect(onPlanClick).toHaveBeenCalledWith(mockPlans[0]) // Cheapest plan
    })

    it('should not call onPackageClick when no packages', () => {
      const onPackageClick = vi.fn()

      render(
        <PackageVsSubscriptionComparison
          packages={[]}
          plans={mockPlans}
          onPackageClick={onPackageClick}
        />
      )

      const packageCard = screen.getByTestId('cheapest-package-card')
      packageCard.click()

      expect(onPackageClick).not.toHaveBeenCalled()
    })

    it('should not call onPlanClick when no plans', () => {
      const onPlanClick = vi.fn()

      render(
        <PackageVsSubscriptionComparison
          packages={mockPackages}
          plans={[]}
          onPlanClick={onPlanClick}
        />
      )

      const planCard = screen.getByTestId('cheapest-plan-card')
      planCard.click()

      expect(onPlanClick).not.toHaveBeenCalled()
    })
  })

  describe('edge cases', () => {
    it('should handle inactive plans correctly', () => {
      const inactivePlans: SubscriptionPlanResponse[] = [
        {
          ...mockPlans[0],
          active: false,
        },
      ]

      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={inactivePlans} />)

      expect(screen.getByText('No plans available')).toBeInTheDocument()
    })

    it('should filter out inactive plans when finding cheapest', () => {
      const mixedPlans: SubscriptionPlanResponse[] = [
        {
          ...mockPlans[0],
          active: false,
          price: 5.99, // Cheaper but inactive
        },
        {
          ...mockPlans[1],
          active: true,
          price: 19.99,
        },
      ]

      render(<PackageVsSubscriptionComparison packages={mockPackages} plans={mixedPlans} />)

      // Should show the active plan, not the inactive cheaper one
      expect(screen.getByText('$19.99')).toBeInTheDocument()
    })

    it('should handle empty arrays for both packages and plans', () => {
      render(<PackageVsSubscriptionComparison packages={[]} plans={[]} />)

      expect(screen.getByTestId('package-vs-subscription-comparison')).toBeInTheDocument()
      expect(screen.getByText('No packages available')).toBeInTheDocument()
      expect(screen.getByText('No plans available')).toBeInTheDocument()
    })

    it('should handle packages with zero price', () => {
      const freePackages: PointsPackageResponse[] = [
        {
          ...mockPackages[0],
          price: 0, // 0 cents = $0.00
          points: 100,
        },
      ]

      render(<PackageVsSubscriptionComparison packages={freePackages} plans={mockPlans} />)

      // formatPrice returns "0.00 USD" when API price is 0 cents
      // Use getAllByText since "0.00 USD" appears in both the table and the card
      expect(screen.getAllByText('0.00 USD').length).toBeGreaterThan(0)
    })
  })
})
