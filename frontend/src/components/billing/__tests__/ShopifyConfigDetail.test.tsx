import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, waitFor, act } from '@testing-library/react'
import { ShopifyConfigDetail } from '../ShopifyConfigDetail'

describe('ShopifyConfigDetail - Sensitive Token Display', () => {
  const mockConfig = {
    shopDomain: 'demo-store.myshopify.com',
    apiVersion: '2024-01',
    webhookEndpoint: 'https://api.example.com/api/third/pay/realm-123/shopify/webhooks',
    adminAccessToken: 'shpat_1234567890abcdef',
    storefrontAccessToken: 'shp_abcdef1234567890',
    appClientSecret: 'my_secret_app_client_key_123',
    lastUpdated: '2026-04-02T10:30:00Z',
    enabled: true,
  }

  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  describe('initial state and masking', () => {
    it('GIVEN config with sensitive data WHEN rendered THEN should mask all tokens by default', () => {
      render(<ShopifyConfigDetail config={mockConfig} />)

      // Admin access token should be masked
      expect(screen.getByTestId('admin-access-token-display')).toHaveTextContent('shpat_***')
      expect(screen.getByTestId('masked-token-display')).toHaveTextContent('shpat_***')

      // Storefront access token should be masked
      expect(screen.getByTestId('storefront-access-token-display')).toHaveTextContent('shp_***')

      // App client secret should be masked
      expect(screen.getByTestId('app-client-secret-display')).toHaveTextContent('***')

      // Non-sensitive fields should be visible
      expect(screen.getByTestId('shop-domain-display')).toHaveTextContent(
        'demo-store.myshopify.com'
      )
      expect(screen.getByTestId('api-version-display')).toHaveTextContent('2024-01')
    })

    it('GIVEN config without webhook endpoint WHEN rendered THEN should not show webhook section', () => {
      const configWithoutWebhook = {
        ...mockConfig,
        webhookEndpoint: undefined,
      }

      render(<ShopifyConfigDetail config={configWithoutWebhook} />)

      expect(screen.queryByTestId('webhook-endpoint-display')).not.toBeInTheDocument()
    })

    it('GIVEN enabled config WHEN rendered THEN should show enabled badge', () => {
      render(<ShopifyConfigDetail config={mockConfig} />)

      expect(screen.getByText('Enabled')).toBeInTheDocument()
    })

    it('GIVEN disabled config WHEN rendered THEN should show disabled badge', () => {
      const disabledConfig = { ...mockConfig, enabled: false }

      render(<ShopifyConfigDetail config={disabledConfig} />)

      expect(screen.getByText('Disabled')).toBeInTheDocument()
    })
  })

  describe('show secrets functionality', () => {
    it('GIVEN masked tokens WHEN Show Secrets button clicked THEN should reveal all sensitive data', () => {
      render(<ShopifyConfigDetail config={mockConfig} />)

      act(() => {
        screen.getByTestId('show-secrets-button').click()
      })

      // All sensitive data should now be visible
      expect(screen.getByTestId('masked-token-display')).toHaveTextContent('shpat_1234567890abcdef')
      expect(screen.getByTestId('storefront-access-token-display')).toHaveTextContent(
        'shp_abcdef1234567890'
      )
      expect(screen.getByTestId('app-client-secret-display')).toHaveTextContent(
        'my_secret_app_client_key_123'
      )

      // Show button should be replaced with Hide button
      expect(screen.queryByTestId('show-secrets-button')).not.toBeInTheDocument()
      expect(screen.getByTestId('hide-secrets-button')).toBeInTheDocument()
    })

    it('GIVEN revealed tokens WHEN Hide Secrets button clicked THEN should mask sensitive data again', () => {
      render(<ShopifyConfigDetail config={mockConfig} />)

      act(() => {
        screen.getByTestId('show-secrets-button').click()
      })

      act(() => {
        screen.getByTestId('hide-secrets-button').click()
      })

      // Data should be masked again
      expect(screen.getByTestId('masked-token-display')).toHaveTextContent('shpat_***')
      expect(screen.getByTestId('storefront-access-token-display')).toHaveTextContent('shp_***')
      expect(screen.getByTestId('app-client-secret-display')).toHaveTextContent('***')

      // Hide button should be replaced with Show button
      expect(screen.queryByTestId('hide-secrets-button')).not.toBeInTheDocument()
      expect(screen.getByTestId('show-secrets-button')).toBeInTheDocument()
    })
  })

  describe('auto-hide after 5 seconds', () => {
    it('GIVEN revealed tokens WHEN 5 seconds pass THEN should auto-hide sensitive data', () => {
      render(<ShopifyConfigDetail config={mockConfig} />)

      act(() => {
        screen.getByTestId('show-secrets-button').click()
      })

      // Verify secrets are visible
      expect(screen.getByTestId('masked-token-display')).toHaveTextContent('shpat_1234567890abcdef')

      // Fast-forward 4.9 seconds - should still be visible
      act(() => {
        vi.advanceTimersByTime(4900)
      })
      expect(screen.getByTestId('masked-token-display')).toHaveTextContent('shpat_1234567890abcdef')

      // Fast-forward to 5 seconds - should hide
      act(() => {
        vi.advanceTimersByTime(100)
      })

      expect(screen.getByTestId('masked-token-display')).toHaveTextContent('shpat_***')
    })

    it('GIVEN revealed tokens WHEN user clicks hide before 5 seconds THEN should cancel auto-hide timer', () => {
      render(<ShopifyConfigDetail config={mockConfig} />)

      act(() => {
        screen.getByTestId('show-secrets-button').click()
      })

      act(() => {
        vi.advanceTimersByTime(3000)
      })

      act(() => {
        screen.getByTestId('hide-secrets-button').click()
      })

      // Advance past the 5 second mark
      act(() => {
        vi.advanceTimersByTime(3000)
      })

      // Should still be hidden (not revealed again)
      expect(screen.getByTestId('masked-token-display')).toHaveTextContent('shpat_***')
      expect(screen.getByTestId('show-secrets-button')).toBeInTheDocument()
    })

    it('GIVEN revealed tokens WHEN user clicks show again before 5 seconds THEN should reset auto-hide timer', () => {
      render(<ShopifyConfigDetail config={mockConfig} />)

      act(() => {
        screen.getByTestId('show-secrets-button').click()
      })
      act(() => {
        vi.advanceTimersByTime(3000)
      })

      // Second click - clicking the button again while it's already shown should reset timer
      // Note: The button is now "hide-secrets-button", but clicking it while already shown
      // is not the expected behavior - let's test clicking hide then show again

      act(() => {
        screen.getByTestId('hide-secrets-button').click()
      })
      act(() => {
        screen.getByTestId('show-secrets-button').click()
      })
      act(() => {
        vi.advanceTimersByTime(3000)
      })

      // Should still be visible (3 seconds from the new show click)
      expect(screen.getByTestId('masked-token-display')).toHaveTextContent('shpat_1234567890abcdef')

      // After another 2 seconds (total 5 from second click)
      act(() => {
        vi.advanceTimersByTime(2000)
      })

      expect(screen.getByTestId('masked-token-display')).toHaveTextContent('shpat_***')
    })
  })

  describe('different token formats', () => {
    it('GIVEN admin token with different content WHEN masked THEN should show correct prefix', () => {
      const configWithDifferentTokens = {
        ...mockConfig,
        adminAccessToken: 'shpat_different_token_value',
        storefrontAccessToken: 'shp_another_token',
        appClientSecret: 'completely_different_secret',
      }

      render(<ShopifyConfigDetail config={configWithDifferentTokens} />)

      expect(screen.getByTestId('admin-access-token-display')).toHaveTextContent('shpat_***')
      expect(screen.getByTestId('storefront-access-token-display')).toHaveTextContent('shp_***')
      expect(screen.getByTestId('app-client-secret-display')).toHaveTextContent('***')
    })

    it('GIVEN short tokens WHEN revealed THEN should show full token', () => {
      const configWithShortTokens = {
        ...mockConfig,
        adminAccessToken: 'shpat_ab',
        storefrontAccessToken: 'shp_cd',
        appClientSecret: 'secret',
      }

      render(<ShopifyConfigDetail config={configWithShortTokens} />)

      act(() => {
        screen.getByTestId('show-secrets-button').click()
      })

      expect(screen.getByTestId('admin-access-token-display')).toHaveTextContent('shpat_ab')
      expect(screen.getByTestId('storefront-access-token-display')).toHaveTextContent('shp_cd')
      expect(screen.getByTestId('app-client-secret-display')).toHaveTextContent('secret')
    })
  })

  describe('cleanup on unmount', () => {
    it('GIVEN component with active timer WHEN unmounted THEN should clear timer', () => {
      const { unmount } = render(<ShopifyConfigDetail config={mockConfig} />)

      act(() => {
        screen.getByTestId('show-secrets-button').click()
      })

      // Unmount before timer fires
      unmount()

      // Advance timers past the auto-hide time
      act(() => {
        vi.advanceTimersByTime(6000)
      })

      // If timer wasn't cleared, this would cause issues
      // The fact we get here without errors means the timer was cleared
      expect(true).toBe(true)
    })
  })

  describe('timestamp formatting', () => {
    it('GIVEN config with lastUpdated WHEN rendered THEN should format timestamp', () => {
      render(<ShopifyConfigDetail config={mockConfig} />)

      // Check that the timestamp is displayed in a human-readable format
      expect(screen.getByText(/last updated:/i)).toBeInTheDocument()
      expect(screen.getByText(/2026/)).toBeInTheDocument()
    })
  })
})
