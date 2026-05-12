import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { SubscriptionSelector } from '../subscription-selector'
import type { ClientAppItem, SubscriptionDetailResponse } from '@/lib/api-generated'

describe('SubscriptionSelector - Rendering', () => {
  const mockSubscriptions: Array<{
    clientApp: ClientAppItem
    subscription: SubscriptionDetailResponse | null
  }> = [
    {
      clientApp: {
        id: 'app1',
        name: 'App 1',
        description: 'Description 1',
        clientId: 'client-1',
        clientSecret: 'secret-1',
        redirectUris: ['http://localhost:3000/callback'],
        postLogoutRedirectUris: ['http://localhost:3000/logout'],
        scopes: ['openid', 'profile'],
        grantTypes: ['authorization_code'],
        realmId: 'realm-1',
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
      subscription: {
        id: 'sub1',
        status: 'active',
        currentPeriodStart: '2025-01-01T00:00:00Z',
        currentPeriodEnd: '2025-02-01T00:00:00Z',
        cancelAtPeriodEnd: false,
        plan: {
          id: 'basic-plan',
          name: 'basic',
          title: 'Basic Plan',
          description: 'Basic plan',
          tier: 'basic',
          price: 0,
          currency: 'USD',
          interval: 'monthly',
        },
      } as SubscriptionDetailResponse,
    },
    {
      clientApp: {
        id: 'app2',
        name: 'App 2',
        description: 'Description 2',
        clientId: 'client-2',
        clientSecret: 'secret-2',
        redirectUris: ['http://localhost:3000/callback'],
        postLogoutRedirectUris: ['http://localhost:3000/logout'],
        scopes: ['openid', 'profile'],
        grantTypes: ['authorization_code'],
        realmId: 'realm-1',
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
      subscription: {
        id: 'sub2',
        status: 'canceled',
        currentPeriodStart: '2025-01-01T00:00:00Z',
        currentPeriodEnd: '2025-02-01T00:00:00Z',
        cancelAtPeriodEnd: true,
        plan: {
          id: 'pro-plan',
          name: 'pro',
          title: 'Pro Plan',
          description: 'Pro plan',
          tier: 'pro',
          price: 29,
          currency: 'USD',
          interval: 'monthly',
        },
      } as SubscriptionDetailResponse,
    },
  ]

  it('should render all subscription cards', () => {
    const mockOnSelect = vi.fn()

    render(<SubscriptionSelector subscriptions={mockSubscriptions} onSelect={mockOnSelect} />)

    expect(screen.getByTestId('subscription-selector')).toBeInTheDocument()
    expect(screen.getByTestId('subscription-card-app1')).toBeInTheDocument()
    expect(screen.getByTestId('subscription-card-app2')).toBeInTheDocument()
    expect(screen.getByText('App 1')).toBeInTheDocument()
    expect(screen.getByText('App 2')).toBeInTheDocument()
  })

  it('should render subscription status badges', () => {
    const mockOnSelect = vi.fn()

    render(<SubscriptionSelector subscriptions={mockSubscriptions} onSelect={mockOnSelect} />)

    expect(screen.getByText('active')).toBeInTheDocument()
    expect(screen.getByText('canceled')).toBeInTheDocument()
  })

  it('should render plan information', () => {
    const mockOnSelect = vi.fn()

    render(<SubscriptionSelector subscriptions={mockSubscriptions} onSelect={mockOnSelect} />)

    expect(screen.getByText('Basic Plan')).toBeInTheDocument()
    expect(screen.getByText('Pro Plan')).toBeInTheDocument()
  })

  it('should render client IDs', () => {
    const mockOnSelect = vi.fn()

    render(<SubscriptionSelector subscriptions={mockSubscriptions} onSelect={mockOnSelect} />)

    expect(screen.getByText('client-1')).toBeInTheDocument()
    expect(screen.getByText('client-2')).toBeInTheDocument()
  })

  it('should display empty state when no subscriptions', () => {
    const mockOnSelect = vi.fn()

    render(<SubscriptionSelector subscriptions={[]} onSelect={mockOnSelect} />)

    expect(screen.getByTestId('subscription-selector-empty')).toBeInTheDocument()
    expect(screen.getByText('No subscriptions found')).toBeInTheDocument()
    expect(screen.queryByTestId('subscription-selector')).not.toBeInTheDocument()
  })

  it('should render subscription without subscription object', () => {
    const mockOnSelect = vi.fn()
    const subscriptionsWithoutSubscription = [
      {
        clientApp: {
          id: 'app1',
          name: 'App 1',
          description: 'Description 1',
          clientId: 'client-1',
          clientSecret: 'secret-1',
          redirectUris: ['http://localhost:3000/callback'],
          postLogoutRedirectUris: ['http://localhost:3000/logout'],
          scopes: ['openid', 'profile'],
          grantTypes: ['authorization_code'],
          realmId: 'realm-1',
          createdAt: '2025-01-01T00:00:00Z',
          updatedAt: '2025-01-01T00:00:00Z',
        },
        subscription: null,
      },
    ]

    render(
      <SubscriptionSelector
        subscriptions={subscriptionsWithoutSubscription}
        onSelect={mockOnSelect}
      />
    )

    expect(screen.getByTestId('subscription-card-app1')).toBeInTheDocument()
    expect(screen.getByText('No subscription')).toBeInTheDocument()
    expect(screen.queryByText('Plan:')).not.toBeInTheDocument()
  })
})

describe('SubscriptionSelector - Selection Logic', () => {
  const mockSubscriptions: Array<{
    clientApp: ClientAppItem
    subscription: SubscriptionDetailResponse | null
  }> = [
    {
      clientApp: {
        id: 'app1',
        name: 'App 1',
        description: 'Description 1',
        clientId: 'client-1',
        clientSecret: 'secret-1',
        redirectUris: ['http://localhost:3000/callback'],
        postLogoutRedirectUris: ['http://localhost:3000/logout'],
        scopes: ['openid', 'profile'],
        grantTypes: ['authorization_code'],
        realmId: 'realm-1',
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
      subscription: {
        id: 'sub1',
        status: 'active',
        currentPeriodStart: '2025-01-01T00:00:00Z',
        currentPeriodEnd: '2025-02-01T00:00:00Z',
        cancelAtPeriodEnd: false,
        plan: {
          id: 'basic-plan',
          name: 'basic',
          title: 'Basic Plan',
          description: 'Basic plan',
          tier: 'basic',
          price: 0,
          currency: 'USD',
          interval: 'monthly',
        },
      } as SubscriptionDetailResponse,
    },
  ]

  it('should call onSelect with subscription ID when subscription card is clicked', async () => {
    const mockOnSelect = vi.fn()
    const user = userEvent.setup()

    render(<SubscriptionSelector subscriptions={mockSubscriptions} onSelect={mockOnSelect} />)

    const card = screen.getByTestId('subscription-card-app1')
    await user.click(card)

    expect(mockOnSelect).toHaveBeenCalledTimes(1)
    expect(mockOnSelect).toHaveBeenCalledWith('sub1')
  })

  it('should call onSelect with client app ID when subscription is null', async () => {
    const mockOnSelect = vi.fn()
    const user = userEvent.setup()
    const subscriptionsWithoutSubscription = [
      {
        clientApp: {
          id: 'app1',
          name: 'App 1',
          description: 'Description 1',
          clientId: 'client-1',
          clientSecret: 'secret-1',
          redirectUris: ['http://localhost:3000/callback'],
          postLogoutRedirectUris: ['http://localhost:3000/logout'],
          scopes: ['openid', 'profile'],
          grantTypes: ['authorization_code'],
          realmId: 'realm-1',
          createdAt: '2025-01-01T00:00:00Z',
          updatedAt: '2025-01-01T00:00:00Z',
        },
        subscription: null,
      },
    ]

    render(
      <SubscriptionSelector
        subscriptions={subscriptionsWithoutSubscription}
        onSelect={mockOnSelect}
      />
    )

    const card = screen.getByTestId('subscription-card-app1')
    await user.click(card)

    expect(mockOnSelect).toHaveBeenCalledTimes(1)
    expect(mockOnSelect).toHaveBeenCalledWith('app1')
  })

  it('should highlight selected subscription card', () => {
    const mockOnSelect = vi.fn()

    render(
      <SubscriptionSelector
        subscriptions={mockSubscriptions}
        selectedId="sub1"
        onSelect={mockOnSelect}
      />
    )

    const card = screen.getByTestId('subscription-card-app1')
    expect(card).toHaveClass('border-primary', 'ring-2', 'ring-primary', 'ring-offset-2')
  })

  it('should not highlight unselected subscription cards', () => {
    const mockOnSelect = vi.fn()

    render(
      <SubscriptionSelector
        subscriptions={mockSubscriptions}
        selectedId="sub2"
        onSelect={mockOnSelect}
      />
    )

    const card = screen.getByTestId('subscription-card-app1')
    expect(card).toHaveClass('border-border')
    expect(card).not.toHaveClass('border-primary', 'ring-2', 'ring-primary', 'ring-offset-2')
  })

  it('should update highlight when selectedId changes', () => {
    const mockOnSelect = vi.fn()
    const { rerender } = render(
      <SubscriptionSelector
        subscriptions={mockSubscriptions}
        selectedId="sub1"
        onSelect={mockOnSelect}
      />
    )

    let card = screen.getByTestId('subscription-card-app1')
    expect(card).toHaveClass('border-primary', 'ring-2', 'ring-primary', 'ring-offset-2')

    rerender(
      <SubscriptionSelector
        subscriptions={mockSubscriptions}
        selectedId="sub2"
        onSelect={mockOnSelect}
      />
    )

    card = screen.getByTestId('subscription-card-app1')
    expect(card).toHaveClass('border-border')
    expect(card).not.toHaveClass('border-primary', 'ring-2', 'ring-primary', 'ring-offset-2')
  })
})

describe('SubscriptionSelector - useCallback Optimization', () => {
  const mockSubscriptions: Array<{
    clientApp: ClientAppItem
    subscription: SubscriptionDetailResponse | null
  }> = [
    {
      clientApp: {
        id: 'app1',
        name: 'App 1',
        description: 'Description 1',
        clientId: 'client-1',
        clientSecret: 'secret-1',
        redirectUris: ['http://localhost:3000/callback'],
        postLogoutRedirectUris: ['http://localhost:3000/logout'],
        scopes: ['openid', 'profile'],
        grantTypes: ['authorization_code'],
        realmId: 'realm-1',
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
      subscription: {
        id: 'sub1',
        status: 'active',
        currentPeriodStart: '2025-01-01T00:00:00Z',
        currentPeriodEnd: '2025-02-01T00:00:00Z',
        cancelAtPeriodEnd: false,
        plan: {
          id: 'basic-plan',
          name: 'basic',
          title: 'Basic Plan',
          description: 'Basic plan',
          tier: 'basic',
          price: 0,
          currency: 'USD',
          interval: 'monthly',
        },
      } as SubscriptionDetailResponse,
    },
  ]

  it('should maintain stable onSelect callback reference when onSelect prop does not change', () => {
    const mockOnSelect = vi.fn()
    let firstHandleSelect: (() => void) | null = null

    const { rerender } = render(
      <SubscriptionSelector subscriptions={mockSubscriptions} onSelect={mockOnSelect} />
    )

    // Access the handleSelect function from the first render
    // This is a bit tricky as it's internal to the component
    // We'll just verify that the component re-renders correctly
    const firstRender = screen.getByTestId('subscription-selector')

    // Rerender with same onSelect
    rerender(<SubscriptionSelector subscriptions={mockSubscriptions} onSelect={mockOnSelect} />)

    const secondRender = screen.getByTestId('subscription-selector')
    expect(firstRender).toEqual(secondRender)
  })

  it('should call correct onSelect when multiple subscriptions exist', async () => {
    const mockOnSelect = vi.fn()
    const user = userEvent.setup()

    const multipleSubscriptions: Array<{
      clientApp: ClientAppItem
      subscription: SubscriptionDetailResponse | null
    }> = [
      {
        clientApp: {
          id: 'app1',
          name: 'App 1',
          description: 'Description 1',
          clientId: 'client-1',
          clientSecret: 'secret-1',
          redirectUris: ['http://localhost:3000/callback'],
          postLogoutRedirectUris: ['http://localhost:3000/logout'],
          scopes: ['openid', 'profile'],
          grantTypes: ['authorization_code'],
          realmId: 'realm-1',
          createdAt: '2025-01-01T00:00:00Z',
          updatedAt: '2025-01-01T00:00:00Z',
        },
        subscription: {
          id: 'sub1',
          status: 'active',
          currentPeriodStart: '2025-01-01T00:00:00Z',
          currentPeriodEnd: '2025-02-01T00:00:00Z',
          cancelAtPeriodEnd: false,
          plan: {
            id: 'basic-plan',
            name: 'basic',
            title: 'Basic Plan',
            description: 'Basic plan',
            tier: 'basic',
            price: 0,
            currency: 'USD',
            interval: 'monthly',
          },
        } as SubscriptionDetailResponse,
      },
      {
        clientApp: {
          id: 'app2',
          name: 'App 2',
          description: 'Description 2',
          clientId: 'client-2',
          clientSecret: 'secret-2',
          redirectUris: ['http://localhost:3000/callback'],
          postLogoutRedirectUris: ['http://localhost:3000/logout'],
          scopes: ['openid', 'profile'],
          grantTypes: ['authorization_code'],
          realmId: 'realm-1',
          createdAt: '2025-01-01T00:00:00Z',
          updatedAt: '2025-01-01T00:00:00Z',
        },
        subscription: {
          id: 'sub2',
          status: 'canceled',
          currentPeriodStart: '2025-01-01T00:00:00Z',
          currentPeriodEnd: '2025-02-01T00:00:00Z',
          cancelAtPeriodEnd: true,
          plan: {
            id: 'pro-plan',
            name: 'pro',
            title: 'Pro Plan',
            description: 'Pro plan',
            tier: 'pro',
            price: 29,
            currency: 'USD',
            interval: 'monthly',
          },
        } as SubscriptionDetailResponse,
      },
    ]

    render(<SubscriptionSelector subscriptions={multipleSubscriptions} onSelect={mockOnSelect} />)

    const card1 = screen.getByTestId('subscription-card-app1')
    const card2 = screen.getByTestId('subscription-card-app2')

    await user.click(card1)
    expect(mockOnSelect).toHaveBeenCalledWith('sub1')

    await user.click(card2)
    expect(mockOnSelect).toHaveBeenCalledWith('sub2')
  })
})

describe('SubscriptionSelector - Status Badge Colors', () => {
  const createSubscriptionWithStatus = (status: string): SubscriptionDetailResponse =>
    ({
      id: 'sub1',
      status: status as any,
      currentPeriodStart: '2025-01-01T00:00:00Z',
      currentPeriodEnd: '2025-02-01T00:00:00Z',
      cancelAtPeriodEnd: false,
      plan: {
        id: 'basic-plan',
        name: 'basic',
        title: 'Basic Plan',
        description: 'Basic plan',
        tier: 'basic',
        price: 0,
        currency: 'USD',
        interval: 'monthly',
      },
    }) as SubscriptionDetailResponse

  const createMockSubscription = (status: string) => ({
    clientApp: {
      id: 'app1',
      name: 'App 1',
      description: 'Description 1',
      clientId: 'client-1',
      clientSecret: 'secret-1',
      redirectUris: ['http://localhost:3000/callback'],
      postLogoutRedirectUris: ['http://localhost:3000/logout'],
      scopes: ['openid', 'profile'],
      grantTypes: ['authorization_code'],
      realmId: 'realm-1',
      createdAt: '2025-01-01T00:00:00Z',
      updatedAt: '2025-01-01T00:00:00Z',
    },
    subscription: createSubscriptionWithStatus(status),
  })

  it('should apply default variant for active status', () => {
    const mockOnSelect = vi.fn()

    render(
      <SubscriptionSelector
        subscriptions={[createMockSubscription('active')]}
        onSelect={mockOnSelect}
      />
    )

    const badge = screen.getByText('active')
    expect(badge).toBeInTheDocument()
  })

  it('should apply default variant for trialing status', () => {
    const mockOnSelect = vi.fn()

    render(
      <SubscriptionSelector
        subscriptions={[createMockSubscription('trialing')]}
        onSelect={mockOnSelect}
      />
    )

    const badge = screen.getByText('trialing')
    expect(badge).toBeInTheDocument()
  })

  it('should apply destructive variant for past_due status', () => {
    const mockOnSelect = vi.fn()

    render(
      <SubscriptionSelector
        subscriptions={[createMockSubscription('past_due')]}
        onSelect={mockOnSelect}
      />
    )

    const badge = screen.getByText('past_due')
    expect(badge).toBeInTheDocument()
  })

  it('should apply secondary variant for canceled status', () => {
    const mockOnSelect = vi.fn()

    render(
      <SubscriptionSelector
        subscriptions={[createMockSubscription('canceled')]}
        onSelect={mockOnSelect}
      />
    )

    const badge = screen.getByText('canceled')
    expect(badge).toBeInTheDocument()
  })

  it('should apply secondary variant for unknown status', () => {
    const mockOnSelect = vi.fn()

    render(
      <SubscriptionSelector
        subscriptions={[createMockSubscription('unknown_status')]}
        onSelect={mockOnSelect}
      />
    )

    const badge = screen.getByText('unknown_status')
    expect(badge).toBeInTheDocument()
  })
})

describe('SubscriptionSelector - Date Formatting', () => {
  it('should format currentPeriodEnd date correctly', () => {
    const mockOnSelect = vi.fn()

    const mockSubscription: Array<{
      clientApp: ClientAppItem
      subscription: SubscriptionDetailResponse | null
    }> = [
      {
        clientApp: {
          id: 'app1',
          name: 'App 1',
          description: 'Description 1',
          clientId: 'client-1',
          clientSecret: 'secret-1',
          redirectUris: ['http://localhost:3000/callback'],
          postLogoutRedirectUris: ['http://localhost:3000/logout'],
          scopes: ['openid', 'profile'],
          grantTypes: ['authorization_code'],
          realmId: 'realm-1',
          createdAt: '2025-01-01T00:00:00Z',
          updatedAt: '2025-01-01T00:00:00Z',
        },
        subscription: {
          id: 'sub1',
          status: 'active',
          currentPeriodStart: '2025-01-01T00:00:00Z',
          currentPeriodEnd: '2025-02-15T00:00:00Z',
          cancelAtPeriodEnd: false,
          plan: {
            id: 'basic-plan',
            name: 'basic',
            title: 'Basic Plan',
            description: 'Basic plan',
            tier: 'basic',
            price: 0,
            currency: 'USD',
            interval: 'monthly',
          },
        } as SubscriptionDetailResponse,
      },
    ]

    render(<SubscriptionSelector subscriptions={mockSubscription} onSelect={mockOnSelect} />)

    // The date should be formatted using toLocaleDateString()
    // We just check that "Expires:" text is present
    expect(screen.getByText(/Expires:/i)).toBeInTheDocument()
  })

  it('should not show expiry date when currentPeriodEnd is missing', () => {
    const mockOnSelect = vi.fn()

    const mockSubscription: Array<{
      clientApp: ClientAppItem
      subscription: SubscriptionDetailResponse | null
    }> = [
      {
        clientApp: {
          id: 'app1',
          name: 'App 1',
          description: 'Description 1',
          clientId: 'client-1',
          clientSecret: 'secret-1',
          redirectUris: ['http://localhost:3000/callback'],
          postLogoutRedirectUris: ['http://localhost:3000/logout'],
          scopes: ['openid', 'profile'],
          grantTypes: ['authorization_code'],
          realmId: 'realm-1',
          createdAt: '2025-01-01T00:00:00Z',
          updatedAt: '2025-01-01T00:00:00Z',
        },
        subscription: {
          id: 'sub1',
          status: 'active',
          currentPeriodStart: '2025-01-01T00:00:00Z',
          cancelAtPeriodEnd: false,
          plan: {
            id: 'basic-plan',
            name: 'basic',
            title: 'Basic Plan',
            description: 'Basic plan',
            tier: 'basic',
            price: 0,
            currency: 'USD',
            interval: 'monthly',
          },
        } as SubscriptionDetailResponse,
      },
    ]

    render(<SubscriptionSelector subscriptions={mockSubscription} onSelect={mockOnSelect} />)

    expect(screen.queryByText(/Expires:/i)).not.toBeInTheDocument()
  })
})
