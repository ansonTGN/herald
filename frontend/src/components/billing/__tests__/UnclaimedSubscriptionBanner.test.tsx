import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { UnclaimedSubscriptionBanner } from '../UnclaimedSubscriptionBanner'

describe('UnclaimedSubscriptionBanner', () => {
  const mockOnClaimClick = vi.fn()
  const mockOnClose = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('initial rendering', () => {
    it('GIVEN count is 0 WHEN rendered THEN should not display banner', () => {
      render(
        <UnclaimedSubscriptionBanner
          count={0}
          onClaimClick={mockOnClaimClick}
          onClose={mockOnClose}
        />
      )

      expect(screen.queryByTestId('unclaimed-subscription-banner')).not.toBeInTheDocument()
    })

    it('GIVEN count is 1 WHEN rendered THEN should display banner with singular text', () => {
      render(
        <UnclaimedSubscriptionBanner
          count={1}
          onClaimClick={mockOnClaimClick}
          onClose={mockOnClose}
        />
      )

      expect(screen.getByTestId('unclaimed-subscription-banner')).toBeInTheDocument()
      expect(screen.getByText('Unclaimed Shopify Subscription Found')).toBeInTheDocument()
      expect(screen.getByTestId('unclaimed-count-display')).toBeInTheDocument()
      expect(
        screen.getByText(/Found 1 Shopify subscription waiting to be claimed/)
      ).toBeInTheDocument()
    })

    it('GIVEN count is greater than 1 WHEN rendered THEN should display banner with plural text', () => {
      render(
        <UnclaimedSubscriptionBanner
          count={3}
          onClaimClick={mockOnClaimClick}
          onClose={mockOnClose}
        />
      )

      expect(screen.getByTestId('unclaimed-subscription-banner')).toBeInTheDocument()
      expect(screen.getByText('Unclaimed Shopify Subscriptions Found')).toBeInTheDocument()
      expect(screen.getByTestId('unclaimed-count-display')).toBeInTheDocument()
      expect(
        screen.getByText(/Found 3 Shopify subscriptions waiting to be claimed/)
      ).toBeInTheDocument()
      expect(screen.getByText(/Claim them to start receiving/)).toBeInTheDocument()
    })
  })

  describe('claim button', () => {
    it('GIVEN count is 1 WHEN rendered THEN should show singular Claim Subscription button', () => {
      render(
        <UnclaimedSubscriptionBanner
          count={1}
          onClaimClick={mockOnClaimClick}
          onClose={mockOnClose}
        />
      )

      const claimButton = screen.getByTestId('claim-subscription-button')
      expect(claimButton).toBeInTheDocument()
      expect(claimButton).toHaveTextContent('Claim Subscription')
    })

    it('GIVEN count is greater than 1 WHEN rendered THEN should show plural Claim Subscriptions button', () => {
      render(
        <UnclaimedSubscriptionBanner
          count={3}
          onClaimClick={mockOnClaimClick}
          onClose={mockOnClose}
        />
      )

      const claimButton = screen.getByTestId('claim-subscription-button')
      expect(claimButton).toBeInTheDocument()
      expect(claimButton).toHaveTextContent('Claim Subscriptions')
    })

    it('GIVEN banner is displayed WHEN Claim Subscription button clicked THEN should call onClaimClick', async () => {
      const user = userEvent.setup()
      render(
        <UnclaimedSubscriptionBanner
          count={1}
          onClaimClick={mockOnClaimClick}
          onClose={mockOnClose}
        />
      )

      await user.click(screen.getByTestId('claim-subscription-button'))
      expect(mockOnClaimClick).toHaveBeenCalledTimes(1)
    })
  })

  describe('close button', () => {
    it('GIVEN banner is displayed WHEN close button clicked THEN should call onClose', async () => {
      const user = userEvent.setup()
      render(
        <UnclaimedSubscriptionBanner
          count={1}
          onClaimClick={mockOnClaimClick}
          onClose={mockOnClose}
        />
      )

      await user.click(screen.getByTestId('unclaimed-banner-close-button'))
      expect(mockOnClose).toHaveBeenCalledTimes(1)
    })

    it('GIVEN banner is displayed WHEN close button clicked THEN should not call onClaimClick', async () => {
      const user = userEvent.setup()
      render(
        <UnclaimedSubscriptionBanner
          count={1}
          onClaimClick={mockOnClaimClick}
          onClose={mockOnClose}
        />
      )

      await user.click(screen.getByTestId('unclaimed-banner-close-button'))
      expect(mockOnClaimClick).not.toHaveBeenCalled()
    })
  })

  describe('styling and layout', () => {
    it('GIVEN banner WHEN rendered THEN should have correct styling classes', () => {
      render(
        <UnclaimedSubscriptionBanner
          count={1}
          onClaimClick={mockOnClaimClick}
          onClose={mockOnClose}
        />
      )

      const banner = screen.getByTestId('unclaimed-subscription-banner')
      expect(banner).toHaveClass('border-yellow-200', 'bg-yellow-50', 'relative')
    })

    it('GIVEN banner WHEN rendered THEN should display alert icon', () => {
      render(
        <UnclaimedSubscriptionBanner
          count={1}
          onClaimClick={mockOnClaimClick}
          onClose={mockOnClose}
        />
      )

      // Check for the alert circle icon (Lucide icon component)
      const banner = screen.getByTestId('unclaimed-subscription-banner')
      expect(banner.querySelector('svg')).toBeInTheDocument()
    })
  })

  describe('accessibility', () => {
    it('GIVEN close button WHEN rendered THEN should have aria-label', () => {
      render(
        <UnclaimedSubscriptionBanner
          count={1}
          onClaimClick={mockOnClaimClick}
          onClose={mockOnClose}
        />
      )

      const closeButton = screen.getByTestId('unclaimed-banner-close-button')
      expect(closeButton).toHaveAttribute('aria-label', 'Close')
    })

    it('GIVEN banner WHEN rendered THEN should be keyboard accessible for close button', async () => {
      const user = userEvent.setup()
      render(
        <UnclaimedSubscriptionBanner
          count={1}
          onClaimClick={mockOnClaimClick}
          onClose={mockOnClose}
        />
      )

      const closeButton = screen.getByTestId('unclaimed-banner-close-button')
      closeButton.focus()
      expect(closeButton).toHaveFocus()

      await user.keyboard('{Enter}')
      expect(mockOnClose).toHaveBeenCalledTimes(1)
    })
  })
})
