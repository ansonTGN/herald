import { describe, it, expect, vi, afterEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { PaymentAttemptStatus } from '../payment-attempt-status'
import { mockPaymentAttempts, makePaymentContext } from '@/test/fixtures/unified-purchase'
import type { PaymentAttemptStatusResponse, WechatJsapiParams } from '@/lib/api-generated'
import { m } from '@/paraglide/messages'

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
  it('shows every fulfilled wallet grant as a separate result', () => {
    render(
      <PaymentAttemptStatus
        status={makeStatusResponse({
          status: 'Succeeded',
          fulfillment: {
            type: 'entitlement_mapping',
            grantedAt: '2026-07-30T00:00:00Z',
            pointGrants: [
              {
                resultId: 'fixed-result',
                ruleId: 'fixed-rule',
                bucketId: 'wallet-fixed',
                pointsType: 'fixed',
                points: 1000,
                description: 'Fixed grant',
              },
              {
                resultId: 'quota-result',
                ruleId: 'quota-rule',
                bucketId: 'wallet-quota',
                pointsType: 'quota',
                points: null,
                description: '25 / 3600s quota',
              },
            ],
          },
        })}
      />
    )

    expect(screen.getByTestId('payment-point-grant-fixed-result')).toHaveTextContent(
      'wallet-fixed+1,000 fixed'
    )
    expect(screen.getByTestId('payment-point-grant-quota-result')).toHaveTextContent(
      'wallet-quota25 / 3600s quota'
    )
  })

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
      expectAbsent('payment-context-degraded', 'payment-status-pending')
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
      expectAbsent('payment-context-degraded', 'payment-status-pending')
    })
  })

  describe('WeChat Native QR branch', () => {
    it('renders the QR payment block when provider=wechat and wechatCodeUrl exists', () => {
      render(
        <PaymentAttemptStatus
          status={makeStatusResponse()}
          paymentProvider="wechat"
          paymentContext={makePaymentContext({
            wechatCodeUrl: 'weixin://wxpay/bizpayurl?pr=test',
          })}
          onCancel={() => {}}
        />
      )

      expectPresent('wechat-qr-payment', 'wechat-qr-code', 'payment-cancel-button')
      expectAbsent('payment-context-degraded', 'payment-redirect-prompt', 'wechat-qr-expired')
    })

    it('replaces the QR with the expired state and regenerate entry once the countdown ends', () => {
      render(
        <PaymentAttemptStatus
          status={makeStatusResponse({
            status: 'Pending',
            expiresAt: PAST_EXPIRES,
          })}
          paymentProvider="wechat"
          paymentContext={makePaymentContext({
            wechatCodeUrl: 'weixin://wxpay/bizpayurl?pr=test',
          })}
          onRetry={() => {}}
        />
      )

      expectPresent('wechat-qr-expired', 'wechat-regenerate-button')
      expectAbsent('wechat-qr-code', 'wechat-qr-countdown')
    })
  })

  describe('WeChat JSAPI branch', () => {
    const jsapiParams: WechatJsapiParams = {
      appId: 'wx1234',
      timeStamp: '1723600000',
      nonceStr: 'nonce',
      package: 'prepay_id=wx123',
      signType: 'RSA',
      paySign: 'signature',
    }

    afterEach(() => {
      delete window.WeixinJSBridge
      vi.restoreAllMocks()
    })

    it('renders the invoke UI when provider=wechat and jsapi params exist', () => {
      render(
        <PaymentAttemptStatus
          status={makeStatusResponse()}
          paymentProvider="wechat"
          paymentContext={makePaymentContext({ wechatJsapiParams: jsapiParams })}
        />
      )

      expectPresent('wechat-jsapi-pending', 'wechat-jsapi-invoke-button')
      expectAbsent('payment-context-degraded', 'wechat-qr-payment')
    })

    it('surfaces bridge feedback per invoke outcome instead of leaving a dead button', async () => {
      const user = userEvent.setup()
      window.WeixinJSBridge = {
        invoke: (_api, _params, callback) => callback({ err_msg: 'get_brand_wcpay_request:ok' }),
      }
      render(
        <PaymentAttemptStatus
          status={makeStatusResponse()}
          paymentProvider="wechat"
          paymentContext={makePaymentContext({ wechatJsapiParams: jsapiParams })}
        />
      )

      await user.click(screen.getByTestId('wechat-jsapi-invoke-button'))
      expect(screen.getByTestId('wechat-jsapi-result-ok')).toBeInTheDocument()

      // The bridge result is feedback only — the attempt itself stays Pending
      // until the server-side callback confirms, so no status change is forced.
      expect(screen.queryByTestId('payment-status-succeeded')).toBeNull()
    })

    it('reports bridge unavailability outside a real WeChat webview', async () => {
      const user = userEvent.setup()
      delete window.WeixinJSBridge
      render(
        <PaymentAttemptStatus
          status={makeStatusResponse()}
          paymentProvider="wechat"
          paymentContext={makePaymentContext({ wechatJsapiParams: jsapiParams })}
        />
      )

      await user.click(screen.getByTestId('wechat-jsapi-invoke-button'))
      expect(screen.getByTestId('wechat-jsapi-result-fail')).toHaveTextContent(
        m['points.payment_wechat_jsapi_bridge_unavailable']()
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
      {
        label: 'wechat with null context',
        provider: 'wechat',
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
      {
        label: 'wechat with neither code URL nor jsapi params',
        provider: 'wechat',
        context: makePaymentContext({}),
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
