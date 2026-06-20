import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PaymentAttemptStatus } from '../payment-attempt-status'
import { mockPaymentAttempts, makePaymentContext } from '@/test/fixtures/unified-purchase'
import type { PaymentAttemptStatusResponse } from '@/lib/api-generated'

const FUTURE_EXPIRES = new Date(Date.now() + 3600 * 1000).toISOString()
const PAST_EXPIRES = new Date(Date.now() - 3600 * 1000).toISOString()

function makeStatusResponse(
  overrides?: Partial<PaymentAttemptStatusResponse>
): PaymentAttemptStatusResponse {
  return {
    ...mockPaymentAttempts.pending,
    expiresAt: FUTURE_EXPIRES,
    ...overrides,
  }
}

function expectPresent(...testids: string[]) {
  for (const id of testids) {
    expect(screen.getByTestId(id)).toBeInTheDocument()
  }
}

function expectAbsent(...testids: string[]) {
  for (const id of testids) {
    expect(screen.queryByTestId(id)).toBeNull()
  }
}

describe('PaymentAttemptStatus provider-specific conditional branches', () => {
  describe('Stripe redirect branch', () => {
    it('renders redirect prompt when provider=stripe and stripeCheckoutUrl exists', () => {
      render(
        <PaymentAttemptStatus
          status={makeStatusResponse()}
          paymentProvider="stripe"
          paymentContext={makePaymentContext({
            stripeCheckoutUrl: 'https://checkout.stripe.com/test',
          })}
        />
      )

      expectPresent('payment-redirect-prompt', 'payment-redirect-manual-link')
      expectAbsent(
        'payment-context-degraded',
        'payment-status-pending'
      )
    })
  })

  describe('Creem redirect branch', () => {
    it('renders redirect prompt when provider=creem and creemCheckoutUrl exists', () => {
      render(
        <PaymentAttemptStatus
          status={makeStatusResponse()}
          paymentProvider="creem"
          paymentContext={makePaymentContext({
            creemCheckoutUrl: 'https://checkout.creem.io/test',
          })}
        />
      )

      expectPresent('payment-redirect-prompt', 'payment-redirect-manual-link')
      expectAbsent(
        'payment-context-degraded',
        'payment-status-pending'
      )
    })
  })

  describe('Degraded UI', () => {
    it.each([
      {
        label: 'stripe with null context',
        provider: 'stripe',
        context: null,
      },
    ] as const)('renders degraded UI when $label (context is null)', ({ provider, context }) => {
      render(
        <PaymentAttemptStatus
          status={makeStatusResponse()}
          paymentProvider={provider}
          paymentContext={context}
        />
      )

      expectPresent('payment-context-degraded')
      expectAbsent(
        'payment-redirect-prompt',
        'payment-redirect-manual-link',
        'payment-status-pending'
      )
    })

    it.each([
      {
        label: 'stripe with missing URL',
        provider: 'stripe',
        context: makePaymentContext({
          stripeCheckoutUrl: null,
        }),
      },
    ] as const)('renders degraded UI when $label', ({ provider, context }) => {
      render(
        <PaymentAttemptStatus
          status={makeStatusResponse()}
          paymentProvider={provider}
          paymentContext={context}
        />
      )

      expectPresent('payment-context-degraded')
      expectAbsent(
        'payment-redirect-prompt',
        'payment-redirect-manual-link',
        'payment-status-pending'
      )
    })
  })

  describe('Countdown expired', () => {
    it('still renders Stripe redirect prompt when expiresAt is in the past', () => {
      render(
        <PaymentAttemptStatus
          status={makeStatusResponse({
            status: 'Pending',
            expiresAt: PAST_EXPIRES,
          })}
          paymentProvider="stripe"
          paymentContext={makePaymentContext({
            stripeCheckoutUrl: 'https://checkout.stripe.com/test',
          })}
        />
      )

      expectPresent('payment-redirect-prompt')
      expectAbsent('payment-context-degraded', 'payment-status-pending')
    })
  })

  describe('No provider props', () => {
    it('renders generic pending UI when no provider props are provided', () => {
      render(<PaymentAttemptStatus status={makeStatusResponse({ status: 'Pending' })} />)

      expectPresent('payment-status-pending')
      expectAbsent(
        'payment-redirect-prompt',
        'payment-redirect-manual-link',
        'payment-context-degraded',
        'payment-status-requires-action'
      )
    })
  })
})
