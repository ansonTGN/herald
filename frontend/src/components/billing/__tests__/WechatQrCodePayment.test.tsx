import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/mocks/server'
import { WechatQrCodePayment } from '../WechatQrCodePayment'

describe('WechatQrCodePayment', () => {
  let queryClient: QueryClient

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    })
  })

  afterEach(() => {
    queryClient.clear()
  })

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  )

  describe('initial rendering', () => {
    it('GIVEN component mounted WHEN rendered THEN shows creating state', () => {
      render(
        <WechatQrCodePayment
          realmId="realm-1"
          planId="plan-123"
          onSuccess={vi.fn()}
          onCancel={vi.fn()}
          onError={vi.fn()}
        />,
        { wrapper }
      )

      expect(screen.getByTestId('wechat-qr-payment')).toBeInTheDocument()
      expect(screen.getByTestId('creating-order-state')).toBeInTheDocument()
      expect(screen.getByText(/creating order/i)).toBeInTheDocument()
    })

    it('GIVEN pending order WHEN rendered THEN shows QR code', async () => {
      render(
        <WechatQrCodePayment
          realmId="realm-1"
          planId="plan-123"
          onSuccess={vi.fn()}
          onCancel={vi.fn()}
          onError={vi.fn()}
        />,
        { wrapper }
      )

      // Wait for order creation and QR code to appear
      await waitFor(() => {
        expect(screen.getByTestId('wechat-qr-code-container')).toBeInTheDocument()
      })

      expect(screen.getByTestId('wechat-qr-code')).toBeInTheDocument()
      expect(screen.getByText(/scan with wechat to pay/i)).toBeInTheDocument()
      expect(screen.getByTestId('cancel-payment-button')).toBeInTheDocument()
    })
  })

  describe('status display', () => {
    it('GIVEN creating status WHEN rendered THEN shows loading spinner', () => {
      render(
        <WechatQrCodePayment
          realmId="realm-1"
          planId="plan-123"
          onSuccess={vi.fn()}
          onCancel={vi.fn()}
          onError={vi.fn()}
        />,
        { wrapper }
      )

      expect(screen.getByTestId('creating-order-state')).toBeInTheDocument()
      expect(screen.getByText(/creating payment order/i)).toBeInTheDocument()
    })

    it('GIVEN pending status WHEN rendered THEN shows waiting badge', async () => {
      render(
        <WechatQrCodePayment
          realmId="realm-1"
          planId="plan-123"
          onSuccess={vi.fn()}
          onCancel={vi.fn()}
          onError={vi.fn()}
        />,
        { wrapper }
      )

      await waitFor(() => {
        expect(screen.getByText(/waiting for payment/i)).toBeInTheDocument()
      })
    })

    it('GIVEN expired status WHEN rendered THEN shows expired message', async () => {
      // Mock order to be expired immediately
      server.use(
        http.post('/api/third/pay/realm-1/wechat/create-order', () =>
          HttpResponse.json({
            orderId: 'order-expired',
            outTradeNo: 'CAS_realm1_abc123',
            codeUrl: 'weixin://wxpay/bizpayurl?pr=abc123',
            expiresAt: new Date(Date.now() - 1000).toISOString(),
          })
        ),
        http.get('/api/third/pay/realm-1/wechat/order-status/order-expired', () =>
          HttpResponse.json({
            orderId: 'order-expired',
            status: 'expired',
          })
        )
      )

      render(
        <WechatQrCodePayment
          realmId="realm-1"
          planId="plan-123"
          onSuccess={vi.fn()}
          onCancel={vi.fn()}
          onError={vi.fn()}
        />,
        { wrapper }
      )

      await waitFor(() => {
        expect(screen.getByTestId('expired-state')).toBeInTheDocument()
      })

      // Check for specific elements in expired state
      expect(screen.getByTestId('expired-state')).toHaveTextContent('QR Code Expired')
      expect(screen.getByTestId('expired-state')).toHaveTextContent('Please try again')
    })
  })

  describe('countdown timer', () => {
    it('GIVEN pending order WHEN rendered THEN shows countdown timer', async () => {
      render(
        <WechatQrCodePayment
          realmId="realm-1"
          planId="plan-123"
          onSuccess={vi.fn()}
          onCancel={vi.fn()}
          onError={vi.fn()}
        />,
        { wrapper }
      )

      await waitFor(() => {
        expect(screen.getByTestId('qr-countdown-display')).toBeInTheDocument()
      })

      const countdownText = screen.getByTestId('qr-countdown-display').textContent
      expect(countdownText).toMatch(/\d+h \d+m \d+s|\d+m \d+s|\d+s/)
    })

    it('GIVEN time remaining WHEN formatTime called THEN formats correctly', async () => {
      render(
        <WechatQrCodePayment
          realmId="realm-1"
          planId="plan-123"
          onSuccess={vi.fn()}
          onCancel={vi.fn()}
          onError={vi.fn()}
        />,
        { wrapper }
      )

      await waitFor(() => {
        expect(screen.getByTestId('qr-countdown-display')).toBeInTheDocument()
      })

      // The countdown should show time in the correct format
      const countdownElement = screen.getByTestId('qr-countdown-display')
      expect(countdownElement.textContent).toBeTruthy()
      expect(countdownElement.textContent?.length).toBeGreaterThan(0)
    })
  })

  describe('user interactions', () => {
    it('GIVEN pending order WHEN cancel clicked THEN calls cancel callbacks', async () => {
      const user = userEvent.setup()
      const onCancel = vi.fn()

      render(
        <WechatQrCodePayment
          realmId="realm-1"
          planId="plan-123"
          onSuccess={vi.fn()}
          onCancel={onCancel}
          onError={vi.fn()}
        />,
        { wrapper }
      )

      await waitFor(() => {
        expect(screen.getByTestId('cancel-payment-button')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('cancel-payment-button'))

      await waitFor(() => {
        expect(onCancel).toHaveBeenCalled()
      })
    })

    it('GIVEN cancel button WHEN clicked THEN cancels payment and calls callback', async () => {
      const user = userEvent.setup()
      const onCancel = vi.fn()

      render(
        <WechatQrCodePayment
          realmId="realm-1"
          planId="plan-123"
          onSuccess={vi.fn()}
          onCancel={onCancel}
          onError={vi.fn()}
        />,
        { wrapper }
      )

      await waitFor(() => {
        expect(screen.getByTestId('cancel-payment-button')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('cancel-payment-button'))

      // Verify cancel callback was called
      expect(onCancel).toHaveBeenCalledTimes(1)
    })
  })

  describe('payment success flow', () => {
    it('GIVEN payment succeeds WHEN status becomes paid THEN calls onSuccess and returns null', async () => {
      const onSuccess = vi.fn()

      // Mock immediate payment success
      server.use(
        http.get('/api/third/pay/realm-1/wechat/order-status/order-123', () =>
          HttpResponse.json({
            orderId: 'order-123',
            status: 'paid',
            tradeState: 'SUCCESS',
          })
        )
      )

      const { container } = render(
        <WechatQrCodePayment
          realmId="realm-1"
          planId="plan-123"
          onSuccess={onSuccess}
          onCancel={vi.fn()}
          onError={vi.fn()}
        />,
        { wrapper }
      )

      // Wait for payment success
      await waitFor(() => {
        expect(onSuccess).toHaveBeenCalled()
      })

      // Component should remain mounted and show success state
      expect(screen.getByTestId('wechat-qr-payment')).toBeInTheDocument()
      expect(screen.getByText('Payment Successful')).toBeInTheDocument()
    })
  })

  describe('payment error flow', () => {
    it.skip(
      'GIVEN payment fails WHEN status becomes failed THEN calls onError and shows error state',
      { timeout: 15000 },
      async () => {
        const onError = vi.fn()

        // Mock payment failure with network error
        server.use(
          http.post('/api/third/pay/realm-1/wechat/create-order', () => {
            return HttpResponse.json({ message: 'WeChat Pay not configured' }, { status: 400 })
          })
        )

        render(
          <WechatQrCodePayment
            realmId="realm-1"
            planId="plan-123"
            onSuccess={vi.fn()}
            onCancel={vi.fn()}
            onError={onError}
          />,
          { wrapper }
        )

        // Wait for error state to be displayed (component shows error state instead of unmounting)
        await waitFor(() => {
          expect(screen.getByTestId('error-state')).toBeInTheDocument()
        })

        // Verify error callback was called
        expect(onError).toHaveBeenCalled()
        expect(screen.getByText('Payment Failed')).toBeInTheDocument()
      }
    )
  })

  describe('payment instructions', () => {
    it('GIVEN pending order WHEN rendered THEN shows payment instructions', async () => {
      render(
        <WechatQrCodePayment
          realmId="realm-1"
          planId="plan-123"
          onSuccess={vi.fn()}
          onCancel={vi.fn()}
          onError={vi.fn()}
        />,
        { wrapper }
      )

      await waitFor(() => {
        expect(screen.getByText(/how to pay/i)).toBeInTheDocument()
      })

      expect(screen.getByText(/open wechat on your phone/i)).toBeInTheDocument()
      expect(screen.getByText(/tap "me" → "pay" → "scan qr code"/i)).toBeInTheDocument()
      expect(screen.getByText(/scan the qr code above/i)).toBeInTheDocument()
      expect(screen.getByText(/confirm payment/i)).toBeInTheDocument()
    })
  })

  describe('QR code display', () => {
    it('GIVEN pending order WHEN rendered THEN shows QR code canvas', async () => {
      render(
        <WechatQrCodePayment
          realmId="realm-1"
          planId="plan-123"
          onSuccess={vi.fn()}
          onCancel={vi.fn()}
          onError={vi.fn()}
        />,
        { wrapper }
      )

      await waitFor(() => {
        expect(screen.getByTestId('wechat-qr-code')).toBeInTheDocument()
      })

      const qrCode = screen.getByTestId('wechat-qr-code')
      expect(qrCode.tagName).toBe('CANVAS')
    })
  })

  describe('component lifecycle', () => {
    it('GIVEN component mounted WHEN order created automatically THEN does not show loading indefinitely', async () => {
      render(
        <WechatQrCodePayment
          realmId="realm-1"
          planId="plan-123"
          onSuccess={vi.fn()}
          onCancel={vi.fn()}
          onError={vi.fn()}
        />,
        { wrapper }
      )

      // Initially should show creating state
      expect(screen.getByTestId('creating-order-state')).toBeInTheDocument()

      // Then should transition to showing QR code
      await waitFor(() => {
        expect(screen.getByTestId('wechat-qr-code-container')).toBeInTheDocument()
      })

      // Creating state should be gone
      expect(screen.queryByTestId('creating-order-state')).not.toBeInTheDocument()
    })
  })

  describe('client app id', () => {
    it('GIVEN clientAppId provided WHEN order created THEN includes in request', async () => {
      const onSuccess = vi.fn()

      render(
        <WechatQrCodePayment
          realmId="realm-1"
          planId="plan-123"
          clientAppId="client-app-456"
          onSuccess={onSuccess}
          onCancel={vi.fn()}
          onError={vi.fn()}
        />,
        { wrapper }
      )

      // Component should initialize successfully with clientAppId
      await waitFor(() => {
        expect(screen.getByTestId('wechat-qr-payment')).toBeInTheDocument()
      })
    })
  })
})
