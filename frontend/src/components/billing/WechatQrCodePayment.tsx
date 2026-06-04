import { QRCodeCanvas } from 'qrcode.react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { AlertCircle, Clock, CheckCircle2, XCircle, Loader2 } from 'lucide-react'
import { useEffect } from 'react'
import { useWechatPay } from '@/hooks/use-wechat-pay'
import { formatDuration } from '@/lib/time-utils'
import { m } from '@/paraglide/messages'

interface WechatQrCodePaymentProps {
  realmId: string
  planId: string
  clientAppId?: string
  onSuccess: () => void
  onCancel: () => void
  onError: (error: Error) => void
}

export function WechatQrCodePayment({
  realmId,
  planId,
  clientAppId,
  onSuccess,
  onCancel,
  onError,
}: WechatQrCodePaymentProps) {
  const {
    status,
    codeUrl,
    timeRemaining,
    error,
    createOrder,
    cancelPayment,
    isCreating,
    isPaid,
    isExpired,
    isFailed,
  } = useWechatPay({
    realmId,
    planId,
    clientAppId,
  })

  // Create order on mount - fixed to use useEffect instead of direct call during render
  useEffect(() => {
    if (status === 'creating' && !codeUrl) {
      createOrder()
    }
  }, [status, codeUrl, createOrder])

  // Handle payment success - call callback but don't unmount
  useEffect(() => {
    if (isPaid && status === 'paid') {
      onSuccess()
    }
  }, [isPaid, status, onSuccess])

  // Handle payment error - call callback but don't unmount
  useEffect(() => {
    if (error && status === 'failed') {
      onError(error)
    }
  }, [error, status, onError])

  const getStatusBadge = () => {
    switch (status) {
      case 'creating':
        return (
          <Badge variant="secondary" className="gap-1">
            <Loader2 className="h-3 w-3 animate-spin" />
            {m['billing.wechat_pay_creating_order']()}
          </Badge>
        )
      case 'pending':
        return (
          <Badge variant="default" className="gap-1">
            <Clock className="h-3 w-3" />
            {m['billing.wechat_pay_waiting']()}
          </Badge>
        )
      case 'paid':
        return (
          <Badge variant="default" className="gap-1">
            <CheckCircle2 className="h-3 w-3" />
            {m['billing.wechat_pay_success']()}
          </Badge>
        )
      case 'expired':
        return (
          <Badge variant="destructive" className="gap-1">
            <XCircle className="h-3 w-3" />
            {m['billing.wechat_pay_expired_badge']()}
          </Badge>
        )
      case 'failed':
        return (
          <Badge variant="destructive" className="gap-1">
            <AlertCircle className="h-3 w-3" />
            {m['billing.wechat_pay_failed_badge']()}
          </Badge>
        )
      default:
        return null
    }
  }

  return (
    <Card className="max-w-md mx-auto" data-testid="wechat-qr-payment">
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>{m['billing.wechat_pay_title']()}</CardTitle>
            <CardDescription>{m['billing.wechat_pay_scan_qr']()}</CardDescription>
          </div>
          {getStatusBadge()}
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Loading state */}
        {isCreating && (
          <div
            className="flex flex-col items-center justify-center py-8"
            data-testid="creating-order-state"
          >
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground mb-4" />
            <p className="text-sm text-muted-foreground">
              {m['billing.wechat_pay_creating_payment']()}
            </p>
          </div>
        )}

        {/* QR Code display */}
        {!isCreating && codeUrl && !isExpired && (
          <div
            className="flex flex-col items-center space-y-4"
            data-testid="wechat-qr-code-container"
          >
            <div className="bg-white p-4 rounded-lg border">
              <QRCodeCanvas
                value={codeUrl}
                size={200}
                level="M"
                includeMargin={false}
                data-testid="wechat-qr-code"
              />
            </div>
            <p className="text-sm font-medium">{m['billing.wechat_pay_scan_to_pay']()}</p>
          </div>
        )}

        {/* Countdown timer */}
        {!isCreating && codeUrl && !isExpired && timeRemaining > 0 && (
          <div className="text-center">
            <div className="text-sm text-muted-foreground">
              {m['billing.wechat_pay_time_remaining']()}
            </div>
            <div className="text-lg font-semibold" data-testid="qr-countdown-display">
              {formatDuration(timeRemaining)}
            </div>
          </div>
        )}

        {/* Expired state */}
        {isExpired && (
          <div
            className="flex flex-col items-center justify-center py-4"
            data-testid="expired-state"
          >
            <XCircle className="h-12 w-12 text-destructive mb-4" />
            <p className="text-sm font-medium">{m['billing.wechat_pay_qr_expired']()}</p>
            <p className="text-xs text-muted-foreground">{m['billing.wechat_pay_try_again']()}</p>
          </div>
        )}

        {/* Error state */}
        {isFailed && error && (
          <div className="flex flex-col items-center justify-center py-4" data-testid="error-state">
            <AlertCircle className="h-12 w-12 text-destructive mb-4" />
            <p className="text-sm font-medium">{m['billing.wechat_pay_payment_failed']()}</p>
            <p className="text-xs text-muted-foreground">{error.message}</p>
          </div>
        )}

        {/* Payment instructions */}
        {!isCreating && codeUrl && !isExpired && (
          <div className="bg-muted rounded-lg p-4 space-y-2">
            <h4 className="font-medium text-sm">{m['billing.wechat_pay_how_to_pay']()}</h4>
            <ol className="text-xs text-muted-foreground space-y-1 list-decimal list-inside">
              <li>{m['billing.wechat_pay_step_open']()}</li>
              <li>{m['billing.wechat_pay_step_scan']()}</li>
              <li>{m['billing.wechat_pay_step_scan_code']()}</li>
              <li>{m['billing.wechat_pay_step_confirm']()}</li>
            </ol>
          </div>
        )}

        {/* Action buttons */}
        <div className="flex gap-2">
          {!isCreating && !isPaid && (
            <Button
              variant="outline"
              onClick={() => {
                cancelPayment()
                onCancel()
              }}
              className="flex-1"
              data-testid="cancel-payment-button"
            >
              {m['billing.wechat_pay_cancel_payment']()}
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  )
}
