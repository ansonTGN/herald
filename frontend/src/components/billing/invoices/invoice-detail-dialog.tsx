import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { format } from 'date-fns'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Download, Clock, User, ExternalLink } from 'lucide-react'
import { InvoiceStatusBadge } from '@/components/billing/invoices/invoice-status-badge'
import { invoiceDetailQueryOptions } from '@/data/invoice-query-options'
import {
  formatInvoiceAmount,
  PDF_DOWNLOADABLE_STATUSES,
  isExternalInvoice,
  getProviderLabel,
  getViewInProviderUrl,
} from '@/lib/invoice-utils'
import type { InvoiceDetailResponse } from '@/lib/api-generated'
import { m } from '@/paraglide/messages'

interface InvoiceDetailDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  realmId: string
  invoiceId: string | null
}

function getEventTypeLabels(): Record<string, string> {
  return {
    created: m['billing.invoice_event_created'](),
    updated: m['billing.invoice_event_updated'](),
    issued: m['billing.invoice_event_issued'](),
    paid: m['billing.invoice_event_paid'](),
    void: m['billing.invoice_event_void'](),
    overdue: m['billing.invoice_event_overdue'](),
  }
}

const EVENT_TYPE_COLORS: Record<string, string> = {
  created: 'bg-green-100 text-green-800 border-green-200',
  updated: 'bg-blue-100 text-blue-800 border-blue-200',
  issued: 'bg-blue-100 text-blue-800 border-blue-200',
  paid: 'bg-green-100 text-green-800 border-green-200',
  void: 'bg-red-100 text-red-800 border-red-200',
  overdue: 'bg-amber-100 text-amber-800 border-amber-200',
}

export function InvoiceDetailDialog({
  open,
  onOpenChange,
  realmId,
  invoiceId,
}: InvoiceDetailDialogProps) {
  const { data: invoice, isLoading } = useQuery({
    ...invoiceDetailQueryOptions(realmId, invoiceId ?? ''),
    enabled: open && !!invoiceId,
  })

  const pdfUrl = invoiceId ? `/api/bill/${realmId}/invoices/${invoiceId}/pdf` : null
  const provider = invoice?.provider
  const isExternal = invoice ? isExternalInvoice(provider!) : false
  const externalPdfUrl = invoice?.externalPdfUrl ?? null
  const externalHostedUrl = invoice?.externalHostedUrl ?? null
  const canDownloadPdf = !isExternal && !!invoice && PDF_DOWNLOADABLE_STATUSES.has(invoice.status)

  if (!open) return null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="max-w-3xl max-h-[90vh] overflow-y-auto"
        data-testid="invoice-detail-dialog"
      >
        <DialogHeader>
          {isLoading ? (
            <>
              <Skeleton className="h-6 w-48" />
              <Skeleton className="h-4 w-32" />
            </>
          ) : invoice ? (
            <>
              <DialogTitle className="flex items-center gap-3">
                {m['billing.invoice_detail_title']({ number: invoice.invoiceNumber })}
                <InvoiceStatusBadge status={invoice.status} provider={invoice.provider} />
              </DialogTitle>
              <DialogDescription>{m['billing.invoice_detail_description']()}</DialogDescription>
            </>
          ) : (
            <>
              <DialogTitle>{m['billing.invoice_not_found']()}</DialogTitle>
              <DialogDescription>{m['billing.invoice_not_found_description']()}</DialogDescription>
            </>
          )}
        </DialogHeader>

        {isLoading ? <LoadingSkeleton /> : invoice ? <InvoiceContent invoice={invoice} /> : null}

        <DialogFooter showCloseButton>
          {isExternal && externalPdfUrl && (
            <Button variant="outline" asChild>
              <a
                href={externalPdfUrl}
                target="_blank"
                rel="noopener noreferrer"
                data-testid="invoice-download-pdf-button"
              >
                <Download className="mr-2 h-4 w-4" />
                {m['billing.invoice_download_pdf']()}
              </a>
            </Button>
          )}
          {isExternal && !externalPdfUrl && externalHostedUrl && (
            <Button variant="outline" asChild>
              <a
                href={externalHostedUrl}
                target="_blank"
                rel="noopener noreferrer"
                data-testid="invoice-view-pdf-button"
              >
                <ExternalLink className="mr-2 h-4 w-4" />
                {m['billing.invoice_view_in_provider']({ provider: getProviderLabel(provider!) })}
              </a>
            </Button>
          )}
          {!isExternal && canDownloadPdf && pdfUrl && (
            <Button variant="outline" asChild>
              <a href={pdfUrl} download data-testid="invoice-download-pdf-button">
                <Download className="mr-2 h-4 w-4" />
                {m['billing.invoice_download_pdf']()}
              </a>
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function InvoiceContent({ invoice }: { invoice: InvoiceDetailResponse }) {
  const fmt = (amount: number) => formatInvoiceAmount(amount, invoice.currency)
  const provider = invoice.provider
  const isExt = isExternalInvoice(provider)
  const externalUrl = getViewInProviderUrl(invoice)

  return (
    <div className="space-y-6">
      {isExt && (
        <div
          className="rounded-md border border-blue-200 bg-blue-50 p-3 flex items-center justify-between"
          data-testid="invoice-external-provider-banner"
        >
          <p className="text-sm text-blue-800">
            {m['billing.invoice_external_managed']({ provider: getProviderLabel(provider) })}
          </p>
          {externalUrl && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => window.open(externalUrl, '_blank', 'noopener,noreferrer')}
              data-testid="invoice-view-in-provider-button"
            >
              <ExternalLink className="mr-2 h-4 w-4" />
              {m['billing.invoice_view_in_provider']({ provider: getProviderLabel(provider) })}
            </Button>
          )}
        </div>
      )}
      <div className="grid grid-cols-1 gap-6 md:grid-cols-2">
        <ContactCard
          title={m['billing.invoice_seller']()}
          name={invoice.sellerName ?? '—'}
          email={invoice.sellerEmail}
          address={invoice.sellerAddress ?? '—'}
          phone={invoice.sellerPhone}
          taxId={invoice.sellerTaxId}
          dataTestId="invoice-seller-info"
        />
        <ContactCard
          title={m['billing.invoice_buyer_label']()}
          name={invoice.billingName ?? '—'}
          email={invoice.billingEmail}
          address={invoice.billingAddress ?? '—'}
          phone={invoice.billingPhone}
          taxId={invoice.billingTaxId}
          dataTestId="invoice-buyer-info"
        />
      </div>

      <div data-testid="invoice-line-items-section">
        <h3 className="mb-2 text-sm font-semibold">{m['billing.invoice_line_items']()}</h3>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{m['billing.invoice_line_item_name']()}</TableHead>
              <TableHead className="text-right">{m['billing.invoice_line_item_qty']()}</TableHead>
              <TableHead className="text-right">
                {m['billing.invoice_line_item_unit_price']()}
              </TableHead>
              <TableHead className="text-right">
                {m['billing.invoice_line_item_subtotal']()}
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {invoice.lineItems.map((item) => (
              <TableRow key={item.id}>
                <TableCell>
                  <div>
                    <span className="font-medium">{item.name}</span>
                    {item.description && (
                      <p className="text-xs text-muted-foreground">{item.description}</p>
                    )}
                  </div>
                </TableCell>
                <TableCell className="text-right">{item.quantity}</TableCell>
                <TableCell className="text-right font-mono">{fmt(item.unitPrice)}</TableCell>
                <TableCell className="text-right font-mono">{fmt(item.subtotal)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      <AmountBreakdown invoice={invoice} fmt={fmt} />
      <AdditionalInfo invoice={invoice} />
      <StatusHistory events={invoice.history} />
    </div>
  )
}

function ContactCard({
  title,
  name,
  email,
  address,
  phone,
  taxId,
  dataTestId,
}: {
  title: string
  name: string
  email?: string | null
  address: string
  phone?: string | null
  taxId?: string | null
  dataTestId: string
}) {
  return (
    <div className="rounded-lg border p-4" data-testid={dataTestId}>
      <h3 className="mb-2 text-sm font-semibold text-muted-foreground">{title}</h3>
      <div className="space-y-1 text-sm">
        <p className="font-medium">{name}</p>
        {email && <p className="text-muted-foreground">{email}</p>}
        <p className="text-muted-foreground">{address}</p>
        {phone && <p className="text-muted-foreground">{phone}</p>}
        {taxId && <p className="text-muted-foreground">{m['billing.invoice_tax_id']({ taxId })}</p>}
      </div>
    </div>
  )
}

function AmountBreakdown({
  invoice,
  fmt,
}: {
  invoice: InvoiceDetailResponse
  fmt: (amount: number) => string
}) {
  return (
    <div
      className="rounded-md border bg-muted/30 p-4 space-y-2"
      data-testid="invoice-amount-breakdown"
    >
      <div className="flex justify-between text-sm">
        <span className="text-muted-foreground">{m['billing.invoice_subtotal']()}</span>
        <span className="font-mono">{fmt(invoice.subtotal)}</span>
      </div>
      <div className="flex justify-between text-sm">
        <span className="text-muted-foreground">{m['billing.invoice_discount']()}</span>
        <span className="font-mono text-red-600">-{fmt(invoice.discountAmount)}</span>
      </div>
      <div className="flex justify-between text-sm">
        <span className="text-muted-foreground">{m['billing.invoice_tax']()}</span>
        <span className="font-mono">+{fmt(invoice.taxAmount)}</span>
      </div>
      <div className="flex justify-between text-sm">
        <span className="text-muted-foreground">{m['billing.invoice_shipping']()}</span>
        <span className="font-mono">+{fmt(invoice.shippingAmount)}</span>
      </div>
      <div className="border-t pt-2 flex justify-between font-semibold">
        <span>{m['billing.invoice_total_label']()}</span>
        <span className="font-mono" data-testid="invoice-total-amount">
          {fmt(invoice.total)}
        </span>
      </div>
    </div>
  )
}

function AdditionalInfo({ invoice }: { invoice: InvoiceDetailResponse }) {
  return (
    <div className="grid grid-cols-2 gap-4 md:grid-cols-4" data-testid="invoice-additional-info">
      {invoice.issueDate && (
        <InfoField label={m['billing.invoice_issue_date']()}>
          {format(new Date(invoice.issueDate), 'PPP')}
        </InfoField>
      )}
      <InfoField label={m['billing.invoice_due_date_label']()}>
        {format(new Date(invoice.dueDate ?? ''), 'PPP')}
      </InfoField>
      {invoice.paymentTerms && (
        <InfoField label={m['billing.invoice_payment_terms']()}>{invoice.paymentTerms}</InfoField>
      )}
      {invoice.notes && (
        <InfoField label={m['billing.invoice_additional_info']()}>{invoice.notes}</InfoField>
      )}
    </div>
  )
}

function InfoField({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <p className="text-xs font-medium text-muted-foreground">{label}</p>
      <p className="text-sm">{children}</p>
    </div>
  )
}

function StatusHistory({ events }: { events: InvoiceDetailResponse['history'] }) {
  const sorted = useMemo(
    () =>
      [...events].sort((a, b) => new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime()),
    [events]
  )

  return (
    <div data-testid="invoice-status-history">
      <h3 className="mb-3 text-sm font-semibold">{m['billing.invoice_status_history']()}</h3>
      {sorted.length === 0 ? (
        <p className="py-4 text-center text-sm text-muted-foreground">
          {m['billing.invoice_no_history']()}
        </p>
      ) : (
        <div className="relative space-y-4 before:absolute before:left-[7px] before:top-2 before:h-[calc(100%-1rem)] before:w-px before:bg-border">
          {sorted.map((event) => (
            <HistoryEvent key={event.id} event={event} />
          ))}
        </div>
      )}
    </div>
  )
}

function HistoryEvent({ event }: { event: InvoiceDetailResponse['history'][number] }) {
  const timestamp = format(new Date(event.createdAt), 'PPp')
  const labels = getEventTypeLabels()
  const label = labels[event.eventType] ?? event.eventType
  const colorClass =
    EVENT_TYPE_COLORS[event.eventType] ?? 'bg-gray-100 text-gray-800 border-gray-200'
  const actorLabel =
    event.actorType === 'system'
      ? m['billing.invoice_actor_system']()
      : m['billing.invoice_actor_user']()

  let changeDescription = ''
  if (event.changes && typeof event.changes === 'object' && event.changes !== null) {
    const changes = event.changes as Record<string, unknown>
    if ('field' in changes && 'from' in changes && 'to' in changes) {
      changeDescription = `${String(changes.field)}: ${String(changes.from)} -> ${String(changes.to)}`
    }
  }

  return (
    <div className="relative flex gap-3">
      <div className="relative flex size-4 shrink-0 items-center justify-center">
        <div className="absolute h-2 w-2 rounded-full bg-primary ring-4 ring-background" />
      </div>
      <div className="flex-1 space-y-1">
        <div className="flex items-center gap-2">
          <span
            className={`inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-medium ${colorClass}`}
          >
            {label}
          </span>
        </div>
        <div className="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
          <span className="flex items-center gap-1">
            <Clock className="h-3 w-3" />
            {timestamp}
          </span>
          <span className="flex items-center gap-1">
            <User className="h-3 w-3" />
            {actorLabel}
          </span>
        </div>
        {changeDescription && <p className="text-xs text-muted-foreground">{changeDescription}</p>}
      </div>
    </div>
  )
}

function LoadingSkeleton() {
  return (
    <div className="space-y-6">
      <div className="grid grid-cols-2 gap-4">
        <Skeleton className="h-24 w-full" />
        <Skeleton className="h-24 w-full" />
      </div>
      <Skeleton className="h-40 w-full" />
      <Skeleton className="h-32 w-full" />
      <Skeleton className="h-20 w-full" />
    </div>
  )
}
