import { m } from '@/paraglide/messages'
import { useEffect, useState, useMemo } from 'react'
import { type PaymentAttemptStatusResponse, type PaymentContextDto } from '@/lib/api-generated'
import { Button } from '@/components/ui/button'
import { AlertCircle, CheckCircle2, Clock, XCircle, RefreshCw, X, ExternalLink } from 'lucide-react'

interface PaymentAttemptStatusProps {
  status: PaymentAttemptStatusResponse
  onRetry?: () => void
  onCancel?: () => void
  isRetrying?: boolean
  isCancelling?: boolean
  paymentProvider?: string | null
  paymentContext?: PaymentContextDto | null
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
        <AlertCircle className="h-8 w-8 text-orange-600" />
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
        <Clock className="h-8 w-8 text-yellow-600 animate-pulse" />
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
              <Clock className="h-8 w-8 text-yellow-600 animate-pulse" />
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
              <CheckCircle2 className="h-8 w-8 text-green-600" />
              <div>
                <h3 className="font-semibold text-lg">{m['points.payment_succeeded_title']()}</h3>
                <p className="text-sm text-muted-foreground">
                  {m['points.payment_succeeded_description']()}
                </p>
              </div>
            </div>
            {status.fulfillment && (
              <div className="rounded-md bg-green-50 p-4">
                <div className="text-sm">
                  <div className="font-medium text-green-900">
                    {m['points.payment_points_granted']()}
                  </div>
                  <div className="text-2xl font-bold text-green-700">
                    +{status.fulfillment.points?.toLocaleString() || '0'}
                  </div>
                  {status.fulfillment.transactionId && (
                    <div className="mt-2 text-xs text-green-600">
                      {m['points.payment_transaction_id']({ id: status.fulfillment.transactionId })}
                    </div>
                  )}
                </div>
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
              <AlertCircle className="h-8 w-8 text-orange-600" />
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
