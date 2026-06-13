import { Badge } from '@/components/ui/badge'
import {
  getInvoiceStatusLabel,
  INVOICE_STATUS_COLORS,
  isExternalInvoice,
  getProviderLabel,
} from '@/lib/invoice-utils'

interface InvoiceStatusBadgeProps {
  status: string
  provider?: string
}

export function InvoiceStatusBadge({ status, provider }: InvoiceStatusBadgeProps) {
  const label = getInvoiceStatusLabel(status)

  const statusBadgeEl = getStatusBadge(status, label)

  if (provider && isExternalInvoice(provider)) {
    return (
      <div className="flex items-center gap-2">
        {statusBadgeEl}
        <Badge variant="secondary" className="bg-teal-50 text-teal-700 border-teal-200 text-xs">
          {getProviderLabel(provider)}
        </Badge>
      </div>
    )
  }

  return statusBadgeEl
}

function getStatusBadge(status: string, label: string) {
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
