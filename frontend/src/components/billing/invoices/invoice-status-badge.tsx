import { Badge } from '@/components/ui/badge'
import { INVOICE_STATUS_LABELS, INVOICE_STATUS_COLORS } from '@/lib/invoice-utils'

export function InvoiceStatusBadge({ status }: { status: string }) {
  const label = INVOICE_STATUS_LABELS[status] ?? status

  if (status === 'paid') {
    return (
      <Badge className="border-transparent bg-green-600 text-white shadow hover:bg-green-600/80">
        {label}
      </Badge>
    )
  }
  if (status === 'overdue') {
    return (
      <Badge className="border-transparent bg-amber-500 text-white shadow hover:bg-amber-500/80">
        {label}
      </Badge>
    )
  }

  const variant = INVOICE_STATUS_COLORS[status] as
    | 'default'
    | 'secondary'
    | 'destructive'
    | 'outline'
    | undefined

  return <Badge variant={variant ?? 'secondary'}>{label}</Badge>
}
