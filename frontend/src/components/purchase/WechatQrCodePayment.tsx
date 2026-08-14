import { m } from '@/paraglide/messages'
import { QRCodeCanvas } from 'qrcode.react'
import { Button } from '@/components/ui/button'
import { RefreshCw } from 'lucide-react'

interface WechatQrCodePaymentProps {
  codeUrl: string
  /** Seconds left before the attempt expires; 0 means the QR is expired. */
  timeRemaining: number
  formattedTime: string
  onRegenerate?: () => void
}

/**
 * Native (PC scan-to-pay) pending state for WeChat: renders the `code_url` as
 * a QR code with the shared attempt countdown. When the countdown reaches
 * zero the QR is replaced by the expired state and a regenerate entry — the
 * order itself is only closed server-side, so regenerating creates a fresh
 * attempt. The cancel button is composed by the caller (shared `CancelButton`).
 */
export function WechatQrCodePayment({
  codeUrl,
  timeRemaining,
  formattedTime,
  onRegenerate,
}: WechatQrCodePaymentProps) {
  const isExpired = timeRemaining <= 0

  return (
    <div data-testid="wechat-qr-payment" className="space-y-4">
      <div className="flex items-center gap-3">
        <div className="text-lg font-semibold">{m['points.payment_wechat_scan_title']()}</div>
      </div>
      <p className="text-sm text-muted-foreground">
        {m['points.payment_wechat_scan_description']()}
      </p>

      {isExpired ? (
        <div
          data-testid="wechat-qr-expired"
          className="rounded-md border border-dashed p-8 text-center"
        >
          <div className="font-medium">{m['points.payment_wechat_expired_title']()}</div>
          <p className="mt-1 text-sm text-muted-foreground">
            {m['points.payment_wechat_expired_description']()}
          </p>
          {onRegenerate && (
            <Button onClick={onRegenerate} className="mt-4" data-testid="wechat-regenerate-button">
              <RefreshCw className="mr-2 h-4 w-4" />
              {m['points.payment_wechat_regenerate']()}
            </Button>
          )}
        </div>
      ) : (
        <>
          {/* White panel: the canvas QR is black-on-transparent and becomes
              unscannable on dark backgrounds. */}
          <div className="flex justify-center rounded-md border bg-white p-6">
            <QRCodeCanvas value={codeUrl} size={220} level="M" data-testid="wechat-qr-code" />
          </div>
          <div className="text-sm text-muted-foreground" data-testid="wechat-qr-countdown">
            {m['points.payment_expires_in']()}{' '}
            <span className="font-mono font-bold text-primary">{formattedTime}</span>
          </div>
        </>
      )}
    </div>
  )
}
