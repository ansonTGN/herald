import { useEffect, useState, useMemo } from 'react'
import { type PaymentAttemptStatusResponse } from '@/lib/api-generated'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { AlertCircle, CheckCircle2, Clock, XCircle, RefreshCw, X } from 'lucide-react'

interface PaymentAttemptStatusProps {
  status: PaymentAttemptStatusResponse
  onRetry?: () => void
  onCancel?: () => void
  isRetrying?: boolean
  isCancelling?: boolean
}

export function PaymentAttemptStatus({
  status,
  onRetry,
  onCancel,
  isRetrying = false,
  isCancelling = false,
}: PaymentAttemptStatusProps) {
  // Calculate initial time remaining using useMemo
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

  // Update time remaining every second
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

  const renderStatusContent = () => {
    switch (status.status) {
      case 'Pending':
        return (
          <div data-testid="payment-status-pending" className="space-y-4">
            <div className="flex items-center gap-3">
              <Clock className="h-8 w-8 text-yellow-600 animate-pulse" />
              <div>
                <h3 className="font-semibold text-lg">Payment Pending</h3>
                <p className="text-sm text-muted-foreground">Waiting for payment completion</p>
              </div>
            </div>
            {timeRemaining > 0 && (
              <div className="rounded-md bg-muted p-4">
                <div className="text-sm">
                  <div className="font-medium">Time Remaining</div>
                  <div
                    className="text-2xl font-mono font-bold text-primary"
                    data-testid="payment-countdown-timer"
                  >
                    {formattedTime}
                  </div>
                </div>
              </div>
            )}
            {onCancel && (
              <Button
                variant="outline"
                onClick={onCancel}
                disabled={isCancelling}
                data-testid="payment-cancel-button"
              >
                <X className="mr-2 h-4 w-4" />
                Cancel Payment
              </Button>
            )}
          </div>
        )

      case 'RequiresAction':
        return (
          <div data-testid="payment-status-requires-action" className="space-y-4">
            <div className="flex items-center gap-3">
              <AlertCircle className="h-8 w-8 text-blue-600" />
              <div>
                <h3 className="font-semibold text-lg">Action Required</h3>
                <p className="text-sm text-muted-foreground">Complete payment to continue</p>
              </div>
            </div>
            {timeRemaining > 0 && (
              <div className="text-center text-sm rounded-md bg-muted p-4">
                <div className="font-medium">Expires in</div>
                <div className="text-xl font-mono font-bold text-destructive">{formattedTime}</div>
              </div>
            )}
            {onCancel && (
              <Button
                variant="outline"
                onClick={onCancel}
                disabled={isCancelling}
                data-testid="payment-cancel-button"
              >
                <X className="mr-2 h-4 w-4" />
                Cancel Payment
              </Button>
            )}
          </div>
        )

      case 'Succeeded':
        return (
          <div data-testid="payment-status-succeeded" className="space-y-4">
            <div className="flex items-center gap-3">
              <CheckCircle2 className="h-8 w-8 text-green-600" />
              <div>
                <h3 className="font-semibold text-lg">Payment Successful!</h3>
                <p className="text-sm text-muted-foreground">
                  Your points have been added to your account
                </p>
              </div>
            </div>
            {status.fulfillment && (
              <div className="rounded-md bg-green-50 p-4">
                <div className="text-sm">
                  <div className="font-medium text-green-900">Points Granted</div>
                  <div className="text-2xl font-bold text-green-700">
                    +{status.fulfillment.points?.toLocaleString() || '0'}
                  </div>
                  {status.fulfillment.transactionId && (
                    <div className="mt-2 text-xs text-green-600">
                      Transaction ID: {status.fulfillment.transactionId}
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
                <h3 className="font-semibold text-lg">Payment Failed</h3>
                <p className="text-sm text-muted-foreground">Payment could not be completed</p>
              </div>
            </div>
            {onRetry && (
              <Button onClick={onRetry} disabled={isRetrying} data-testid="payment-retry-button">
                <RefreshCw className="mr-2 h-4 w-4" />
                Try Again
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
                <h3 className="font-semibold text-lg">Payment Cancelled</h3>
                <p className="text-sm text-muted-foreground">The payment was cancelled</p>
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
                <h3 className="font-semibold text-lg">Payment Expired</h3>
                <p className="text-sm text-muted-foreground">
                  The payment time limit has been reached
                </p>
              </div>
            </div>
            {onRetry && (
              <Button onClick={onRetry} disabled={isRetrying} data-testid="payment-retry-button">
                <RefreshCw className="mr-2 h-4 w-4" />
                Try Again
              </Button>
            )}
          </div>
        )

      default:
        return (
          <div className="flex items-center gap-3">
            <AlertCircle className="h-8 w-8 text-muted-foreground" />
            <div>
              <h3 className="font-semibold text-lg">Unknown Status</h3>
              <p className="text-sm text-muted-foreground">Payment status: {status.status}</p>
            </div>
          </div>
        )
    }
  }

  return (
    <div data-testid="payment-status-display">
      <Card>
        <CardHeader>
          <CardTitle>Payment Status</CardTitle>
        </CardHeader>
        <CardContent>{renderStatusContent()}</CardContent>
      </Card>
    </div>
  )
}
