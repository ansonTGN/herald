import { m } from '@/paraglide/messages'
import { useEffect, useState, useMemo } from 'react'
import {
  type PaymentAttemptStatusResponse,
  type PaymentContextResponse,
  type WechatJsapiParams,
} from '@/lib/api-generated'
import { Button } from '@/components/ui/button'
import { AlertCircle, CheckCircle2, Clock, XCircle, RefreshCw, X, ExternalLink } from 'lucide-react'
import { WechatQrCodePayment } from '@/components/purchase/WechatQrCodePayment'
import { invokeWechatJsapiPay, type WechatJsapiInvokeResult } from '@/lib/wechat-pay-utils'

interface PaymentAttemptStatusProps {
  status: PaymentAttemptStatusResponse
  onRetry?: () => void
  onCancel?: () => void
  isRetrying?: boolean
  isCancelling?: boolean
  paymentProvider?: string | null
  paymentContext?: PaymentContextResponse | null
}

function CancelButton({
  onCancel,
  isCancelling,
}: {
  onCancel?: () => void
  isCancelling?: boolean
}) {
  if (!onCancel) return null
  return (
    <Button
      variant="outline"
      onClick={onCancel}
      disabled={isCancelling}
      data-testid="payment-cancel-button"
    >
      <X className="mr-2 h-4 w-4" />
      {m['points.payment_cancel_button']()}
    </Button>
  )
}

function CountdownTimer({
  formattedTime,
  variant,
}: {
  formattedTime: string
  variant: 'primary' | 'destructive'
}) {
  const colorClass = variant === 'primary' ? 'text-primary' : 'text-destructive'
  return (
    <span className={`font-mono font-bold ${colorClass}`} data-testid="payment-countdown-timer">
      {formattedTime}
    </span>
  )
}

/**
 * JSAPI (in-WeChat) pending state: invokes WeChat's built-in payment sheet
 * via WeixinJSBridge. The bridge result is user feedback only — the attempt
 * stays Pending until the server-side callback confirms the outcome, so
 * polling continues regardless of the bridge result.
 */
function WechatJsapiPayment({ params }: { params: WechatJsapiParams }) {
  const [invokeState, setInvokeState] = useState<'idle' | 'invoking' | WechatJsapiInvokeResult>(
    'idle'
  )

  const handleInvoke = async () => {
    setInvokeState('invoking')
    const result = await invokeWechatJsapiPay(params)
    setInvokeState(result)
  }

  return (
    <div data-testid="wechat-jsapi-payment" className="space-y-4">
      <div className="text-lg font-semibold">{m['points.payment_wechat_jsapi_title']()}</div>
      <p className="text-sm text-muted-foreground">
        {m['points.payment_wechat_jsapi_description']()}
      </p>
      <Button
        onClick={handleInvoke}
        disabled={invokeState === 'invoking'}
        data-testid="wechat-jsapi-invoke-button"
      >
        <RefreshCw className="mr-2 h-4 w-4" />
        {invokeState === 'invoking'
          ? m['points.payment_wechat_jsapi_invoking']()
          : m['points.payment_wechat_jsapi_invoke']()}
      </Button>
      {invokeState === 'ok' && (
        <p className="text-sm text-muted-foreground" data-testid="wechat-jsapi-result-ok">
          {m['points.payment_wechat_jsapi_ok']()}
        </p>
      )}
      {invokeState === 'cancel' && (
        <p className="text-sm text-muted-foreground" data-testid="wechat-jsapi-result-cancel">
          {m['points.payment_wechat_jsapi_cancelled']()}
        </p>
      )}
      {(invokeState === 'fail' || invokeState === 'bridge_unavailable') && (
        <p className="text-sm text-destructive" data-testid="wechat-jsapi-result-fail">
          {invokeState === 'bridge_unavailable'
            ? m['points.payment_wechat_jsapi_bridge_unavailable']()
            : m['points.payment_wechat_jsapi_failed']()}
        </p>
      )}
    </div>
  )
}

export function PaymentAttemptStatus({
  status,
  onRetry,
  onCancel,
  isRetrying = false,
  isCancelling = false,
  paymentProvider,
  paymentContext,
}: PaymentAttemptStatusProps) {
  const initialTimeRemaining = useMemo(() => {
    if (!status.expiresAt || (status.status !== 'Pending' && status.status !== 'RequiresAction')) {
      return 0
    }
    // eslint-disable-next-line react-hooks/purity -- Date.now() is safe here for initial calculation in useMemo
    const now = Date.now()
    const expires = new Date(status.expiresAt).getTime()
    return Math.max(0, Math.ceil((expires - now) / 1000))
  }, [status.expiresAt, status.status])

  const [timeRemaining, setTimeRemaining] = useState(initialTimeRemaining)

  useEffect(() => {
    if (initialTimeRemaining === 0) return

    const timer = setInterval(() => {
      setTimeRemaining((prev) => Math.max(0, prev - 1))
    }, 1000)

    return () => clearInterval(timer)
  }, [initialTimeRemaining])

  const formattedTime = useMemo(() => {
    const minutes = Math.floor(timeRemaining / 60)
    const seconds = timeRemaining % 60
    return `${minutes}:${seconds.toString().padStart(2, '0')}`
  }, [timeRemaining])

  const renderDegradedUI = () => (
    <div data-testid="payment-context-degraded" className="space-y-4">
      <div className="flex items-center gap-3">
        <AlertCircle className="h-8 w-8 text-warning" />
        <div>
          <h3 className="font-semibold text-lg">{m['points.payment_degraded_title']()}</h3>
          <p className="text-sm text-muted-foreground">
            {m['points.payment_degraded_description']()}
          </p>
        </div>
      </div>
      <CancelButton onCancel={onCancel} isCancelling={isCancelling} />
    </div>
  )

  const renderRedirectPrompt = (providerName: string, checkoutUrl: string) => (
    <div data-testid="payment-redirect-prompt" className="space-y-4">
      <div className="flex items-center gap-3">
        <Clock className="h-8 w-8 text-warning animate-pulse" />
        <div>
          <h3 className="font-semibold text-lg">{m['points.payment_redirect_title']()}</h3>
          <p className="text-sm text-muted-foreground">
            {m['points.payment_redirect_description']({ provider: providerName })}
          </p>
        </div>
      </div>
      <div className="rounded-md bg-muted p-4 space-y-2">
        <p className="text-sm text-muted-foreground">
          {m['points.payment_redirect_manual_prompt']()}
        </p>
        <a
          href={checkoutUrl}
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-1.5 text-sm font-medium text-primary hover:underline"
          data-testid="payment-redirect-manual-link"
        >
          {m['points.payment_redirect_manual_link']()}
          <ExternalLink className="h-4 w-4" />
        </a>
      </div>
      {timeRemaining > 0 && (
        <div className="text-sm text-muted-foreground">
          {m['points.payment_expires_in']()}{' '}
          <CountdownTimer formattedTime={formattedTime} variant="destructive" />
        </div>
      )}
      <CancelButton onCancel={onCancel} isCancelling={isCancelling} />
    </div>
  )

  const renderProviderPendingContent = () => {
    if (!paymentContext) {
      return renderDegradedUI()
    }

    if (paymentProvider === 'stripe' && paymentContext.stripeCheckoutUrl) {
      return renderRedirectPrompt('Stripe', paymentContext.stripeCheckoutUrl)
    }

    if (paymentProvider === 'creem' && paymentContext.creemCheckoutUrl) {
      return renderRedirectPrompt('Creem', paymentContext.creemCheckoutUrl)
    }

    if (paymentProvider === 'wechat' && paymentContext.wechatCodeUrl) {
      return (
        <div className="space-y-4" data-testid="wechat-native-pending">
          <WechatQrCodePayment
            codeUrl={paymentContext.wechatCodeUrl}
            timeRemaining={timeRemaining}
            formattedTime={formattedTime}
            onRegenerate={onRetry}
          />
          <CancelButton onCancel={onCancel} isCancelling={isCancelling} />
        </div>
      )
    }

    if (paymentProvider === 'wechat' && paymentContext.wechatJsapiParams) {
      return (
        <div className="space-y-4" data-testid="wechat-jsapi-pending">
          <WechatJsapiPayment params={paymentContext.wechatJsapiParams} />
          <CancelButton onCancel={onCancel} isCancelling={isCancelling} />
        </div>
      )
    }

    return renderDegradedUI()
  }

  const renderStatusContent = () => {
    switch (status.status) {
      case 'Pending': {
        if (paymentProvider) {
          return renderProviderPendingContent()
        }
        return (
          <div data-testid="payment-status-pending" className="space-y-4">
            <div className="flex items-center gap-3">
              <Clock className="h-8 w-8 text-warning animate-pulse" />
              <div>
                <h3 className="font-semibold text-lg">{m['points.payment_pending_title']()}</h3>
                <p className="text-sm text-muted-foreground">
                  {m['points.payment_pending_description']()}
                </p>
              </div>
            </div>
            {timeRemaining > 0 && (
              <div className="rounded-md bg-muted p-4">
                <div className="text-sm">
                  <div className="font-medium">{m['points.payment_time_remaining']()}</div>
                  <div className="text-2xl font-mono font-bold">
                    <CountdownTimer formattedTime={formattedTime} variant="primary" />
                  </div>
                </div>
              </div>
            )}
            <CancelButton onCancel={onCancel} isCancelling={isCancelling} />
          </div>
        )
      }

      case 'RequiresAction': {
        if (paymentProvider) {
          return renderProviderPendingContent()
        }
        return (
          <div data-testid="payment-status-requires-action" className="space-y-4">
            <div className="flex items-center gap-3">
              <AlertCircle className="h-8 w-8 text-primary" />
              <div>
                <h3 className="font-semibold text-lg">{m['points.payment_action_required']()}</h3>
                <p className="text-sm text-muted-foreground">
                  {m['points.payment_action_required_description']()}
                </p>
              </div>
            </div>
            {timeRemaining > 0 && (
              <div className="text-center text-sm rounded-md bg-muted p-4">
                <div className="font-medium">{m['points.payment_expires_in']()}</div>
                <div className="text-xl font-mono font-bold">
                  <CountdownTimer formattedTime={formattedTime} variant="destructive" />
                </div>
              </div>
            )}
            <CancelButton onCancel={onCancel} isCancelling={isCancelling} />
          </div>
        )
      }

      case 'Succeeded':
        return (
          <div data-testid="payment-status-succeeded" className="space-y-4">
            <div className="flex items-center gap-3">
              <CheckCircle2 className="h-8 w-8 text-success" />
              <div>
                <h3 className="font-semibold text-lg">{m['points.payment_succeeded_title']()}</h3>
                <p className="text-sm text-muted-foreground">
                  {m['points.payment_succeeded_description']()}
                </p>
              </div>
            </div>
            {status.fulfillment && status.fulfillment.pointGrants.length > 0 && (
              <div className="space-y-2 rounded-md bg-success/10 p-4">
                <div className="font-medium text-success">
                  {m['points.payment_points_granted']()}
                </div>
                {status.fulfillment.pointGrants.map((grant) => (
                  <div
                    key={grant.resultId}
                    className="text-sm text-success"
                    data-testid={`payment-point-grant-${grant.resultId}`}
                  >
                    <span className="font-mono text-xs">{grant.bucketId}</span>
                    <span className="ml-2">
                      {grant.points == null
                        ? grant.description
                        : `+${grant.points.toLocaleString()} ${grant.pointsType}`}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )

      case 'Failed':
        return (
          <div data-testid="payment-status-failed" className="space-y-4">
            <div className="flex items-center gap-3">
              <XCircle className="h-8 w-8 text-destructive" />
              <div>
                <h3 className="font-semibold text-lg">{m['points.payment_failed_title']()}</h3>
                <p className="text-sm text-muted-foreground">
                  {m['points.payment_failed_description']()}
                </p>
              </div>
            </div>
            {onRetry && (
              <Button onClick={onRetry} disabled={isRetrying} data-testid="payment-retry-button">
                <RefreshCw className="mr-2 h-4 w-4" />
                {m['points.payment_try_again']()}
              </Button>
            )}
          </div>
        )

      case 'Cancelled':
        return (
          <div data-testid="payment-status-cancelled" className="space-y-4">
            <div className="flex items-center gap-3">
              <XCircle className="h-8 w-8 text-muted-foreground" />
              <div>
                <h3 className="font-semibold text-lg">{m['points.payment_cancelled_title']()}</h3>
                <p className="text-sm text-muted-foreground">
                  {m['points.payment_cancelled_description']()}
                </p>
              </div>
            </div>
          </div>
        )

      case 'Expired':
        return (
          <div data-testid="payment-status-expired" className="space-y-4">
            <div className="flex items-center gap-3">
              <AlertCircle className="h-8 w-8 text-warning" />
              <div>
                <h3 className="font-semibold text-lg">{m['points.payment_expired_title']()}</h3>
                <p className="text-sm text-muted-foreground">
                  {m['points.payment_expired_description']()}
                </p>
              </div>
            </div>
            {onRetry && (
              <Button onClick={onRetry} disabled={isRetrying} data-testid="payment-retry-button">
                <RefreshCw className="mr-2 h-4 w-4" />
                {m['points.payment_try_again']()}
              </Button>
            )}
          </div>
        )

      default:
        return (
          <div className="flex items-center gap-3">
            <AlertCircle className="h-8 w-8 text-muted-foreground" />
            <div>
              <h3 className="font-semibold text-lg">{m['points.payment_unknown_title']()}</h3>
              <p className="text-sm text-muted-foreground">
                {m['points.payment_unknown_description']({ status: status.status })}
              </p>
            </div>
          </div>
        )
    }
  }

  return <div data-testid="payment-status-display">{renderStatusContent()}</div>
}
