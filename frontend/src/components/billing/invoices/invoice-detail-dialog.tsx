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
import { Download, Clock, User } from 'lucide-react'
import { InvoiceStatusBadge } from '@/components/billing/invoices/invoice-status-badge'
import { invoiceDetailQueryOptions } from '@/data/invoice-query-options'
import { formatInvoiceAmount, PDF_DOWNLOADABLE_STATUSES } from '@/lib/invoice-utils'
import type { InvoiceDetailResponse } from '@/lib/api-generated'

interface InvoiceDetailDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  realmId: string
  invoiceId: string | null
}

const EVENT_TYPE_LABELS: Record<string, string> = {
  created: 'Created',
  updated: 'Updated',
  issued: 'Issued',
  paid: 'Paid',
  void: 'Voided',
  overdue: 'Overdue',
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
  const canDownloadPdf = !!invoice && PDF_DOWNLOADABLE_STATUSES.has(invoice.status)

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
                Invoice {invoice.invoiceNumber}
                <InvoiceStatusBadge status={invoice.status} />
              </DialogTitle>
              <DialogDescription>Invoice details and status history</DialogDescription>
            </>
          ) : (
            <>
              <DialogTitle>Invoice Not Found</DialogTitle>
              <DialogDescription>The requested invoice could not be loaded.</DialogDescription>
            </>
          )}
        </DialogHeader>

        {isLoading ? <LoadingSkeleton /> : invoice ? <InvoiceContent invoice={invoice} /> : null}

        <DialogFooter showCloseButton>
          {canDownloadPdf && pdfUrl && (
            <Button variant="outline" asChild>
              <a href={pdfUrl} download data-testid="invoice-download-pdf-button">
                <Download className="mr-2 h-4 w-4" />
                Download PDF
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

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 gap-6 md:grid-cols-2">
        <ContactCard
          title="Seller"
          name={invoice.sellerName}
          email={invoice.sellerEmail}
          address={invoice.sellerAddress}
          phone={invoice.sellerPhone}
          taxId={invoice.sellerTaxId}
          dataTestId="invoice-seller-info"
        />
        <ContactCard
          title="Buyer"
          name={invoice.billingName}
          email={invoice.billingEmail}
          address={invoice.billingAddress}
          phone={invoice.billingPhone}
          taxId={invoice.billingTaxId}
          dataTestId="invoice-buyer-info"
        />
      </div>

      <div data-testid="invoice-line-items-section">
        <h3 className="mb-2 text-sm font-semibold">Line Items</h3>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead className="text-right">Qty</TableHead>
              <TableHead className="text-right">Unit Price</TableHead>
              <TableHead className="text-right">Subtotal</TableHead>
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
        {taxId && <p className="text-muted-foreground">Tax ID: {taxId}</p>}
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
        <span className="text-muted-foreground">Subtotal</span>
        <span className="font-mono">{fmt(invoice.subtotal)}</span>
      </div>
      <div className="flex justify-between text-sm">
        <span className="text-muted-foreground">Discount (-)</span>
        <span className="font-mono text-red-600">-{fmt(invoice.discountAmount)}</span>
      </div>
      <div className="flex justify-between text-sm">
        <span className="text-muted-foreground">Tax (+)</span>
        <span className="font-mono">+{fmt(invoice.taxAmount)}</span>
      </div>
      <div className="flex justify-between text-sm">
        <span className="text-muted-foreground">Shipping (+)</span>
        <span className="font-mono">+{fmt(invoice.shippingAmount)}</span>
      </div>
      <div className="border-t pt-2 flex justify-between font-semibold">
        <span>Total</span>
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
        <InfoField label="Issue Date">{format(new Date(invoice.issueDate), 'PPP')}</InfoField>
      )}
      <InfoField label="Due Date">{format(new Date(invoice.dueDate), 'PPP')}</InfoField>
      {invoice.paymentTerms && <InfoField label="Payment Terms">{invoice.paymentTerms}</InfoField>}
      {invoice.notes && <InfoField label="Additional Information">{invoice.notes}</InfoField>}
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
      <h3 className="mb-3 text-sm font-semibold">Status History</h3>
      {sorted.length === 0 ? (
        <p className="py-4 text-center text-sm text-muted-foreground">No history events</p>
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
  const label = EVENT_TYPE_LABELS[event.eventType] ?? event.eventType
  const colorClass =
    EVENT_TYPE_COLORS[event.eventType] ?? 'bg-gray-100 text-gray-800 border-gray-200'
  const actorLabel = event.actorType === 'system' ? 'System' : 'User'

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
