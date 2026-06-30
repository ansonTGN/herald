import { Badge } from '@/components/ui/badge'
import type { InvoiceResponse } from '@/lib/api-generated'
import {
  formatInvoiceAmount,
  getCreditNoteSourceColorClass,
  getProviderLabel,
  shouldRenderRefundDimension,
} from '@/lib/invoice-utils'
import { m } from '@/paraglide/messages'

interface RefundAmountChipProps {
  invoice: InvoiceResponse
}

function getRefundSourceLabel(provider: string): string {
  if (provider === 'stripe') return m['billing.refund_source_stripe']()
  if (provider === 'manual') return m['billing.refund_source_manual']()
  return getProviderLabel(provider)
}

export function RefundAmountChip({ invoice }: RefundAmountChipProps) {
  if (!shouldRenderRefundDimension(invoice.provider, invoice.amountRefunded)) {
    return <span className="text-muted-foreground">—</span>
  }

  const colorClass = getCreditNoteSourceColorClass(invoice.provider)
  const sourceLabel = getRefundSourceLabel(invoice.provider)

  return (
    <div
      className="flex flex-col items-start gap-1"
      data-testid={`invoice-refund-chip-${invoice.id}`}
    >
      <span className="font-mono text-sm">
        {formatInvoiceAmount(invoice.amountRefunded, invoice.currency)}
      </span>
      <Badge variant="secondary" className={colorClass.badge}>
        {sourceLabel}
      </Badge>
    </div>
  )
}
