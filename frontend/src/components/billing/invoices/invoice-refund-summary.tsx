import { Alert, AlertDescription } from '@/components/ui/alert'
import {
  formatInvoiceAmount,
  isRefundOverTotal,
  shouldRenderRefundDimension,
} from '@/lib/invoice-utils'
import type { InvoiceDetailResponse } from '@/lib/api-generated'
import { m } from '@/paraglide/messages'

interface InvoiceRefundSummaryProps {
  invoice: InvoiceDetailResponse
}

export function InvoiceRefundSummary({ invoice }: InvoiceRefundSummaryProps) {
  if (!shouldRenderRefundDimension(invoice.provider, invoice.amountRefunded)) {
    return null
  }

  const overTotal = isRefundOverTotal(invoice.amountRefunded, invoice.total)

  return (
    <div data-testid="invoice-refund-summary">
      <div className="rounded-md border bg-muted/30 p-4 space-y-2">
        <div className="flex justify-between text-sm">
          <span className="text-muted-foreground">{m['billing.invoice_total_label']()}</span>
          <span className="font-mono">{formatInvoiceAmount(invoice.total, invoice.currency)}</span>
        </div>
        <div className="flex justify-between text-sm">
          <span className="text-muted-foreground">{m['billing.credit_note_refunded_label']()}</span>
          <span className="font-mono text-red-600" data-testid="invoice-refunded-amount">
            -{formatInvoiceAmount(invoice.amountRefunded, invoice.currency)}
          </span>
        </div>
        <div className="border-t pt-2 flex justify-between font-semibold">
          <span>{m['billing.credit_note_remaining_label']()}</span>
          <span className="font-mono text-primary" data-testid="invoice-remaining-amount">
            {formatInvoiceAmount(invoice.amountRemaining, invoice.currency)}
          </span>
        </div>
      </div>

      {overTotal && (
        <Alert variant="destructive" className="mt-3" data-testid="invoice-refund-over-total-alert">
          <AlertDescription>{m['billing.credit_note_over_total_alert']()}</AlertDescription>
        </Alert>
      )}
    </div>
  )
}
