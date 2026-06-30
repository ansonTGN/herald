import { ExternalLink } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { CreditNoteRow } from './credit-note-row'
import {
  getCreditNoteSourceColorClass,
  getProviderLabel,
  getViewInProviderUrl,
  shouldRenderRefundDimension,
} from '@/lib/invoice-utils'
import type { InvoiceDetailResponse } from '@/lib/api-generated'
import { m } from '@/paraglide/messages'

interface CreditNoteListProps {
  invoice: InvoiceDetailResponse
  showCreditNotes: boolean
  showRecordRefundEntry?: boolean
  onRecordRefund?: () => void
}

export function CreditNoteList({
  invoice,
  showCreditNotes,
  showRecordRefundEntry,
  onRecordRefund,
}: CreditNoteListProps) {
  if (!showCreditNotes || !shouldRenderRefundDimension(invoice.provider, invoice.amountRefunded)) {
    return null
  }

  const manualNotes = invoice.creditNotes.filter((n) => n.source === 'manual')
  const stripeNotes = invoice.creditNotes.filter((n) => n.source === 'stripe')

  return (
    <div className="space-y-4" data-testid="credit-note-list">
      <h3 className="text-sm font-semibold">{m['billing.credit_note_list_title']()}</h3>

      <TrackCard
        track="manual"
        title={m['billing.credit_note_list_manual_title']()}
        notes={manualNotes}
        invoice={invoice}
        showOperator
      >
        <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground">
          <span>{m['billing.credit_note_list_manual_footer']()}</span>
          {showRecordRefundEntry && onRecordRefund && (
            <button
              type="button"
              onClick={onRecordRefund}
              className="inline-flex items-center gap-1 text-primary hover:underline"
              data-testid="record-refund-button"
            >
              + {m['billing.credit_note_record_refund_button']()}
            </button>
          )}
        </div>
      </TrackCard>

      <TrackCard
        track="stripe"
        title={m['billing.credit_note_list_stripe_title']()}
        notes={stripeNotes}
        invoice={invoice}
        showOperator={false}
      >
        <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground">
          <span>{m['billing.credit_note_list_stripe_footer']()}</span>
          <StripeViewLink invoice={invoice} />
        </div>
      </TrackCard>
    </div>
  )
}

interface TrackCardProps {
  track: 'manual' | 'stripe'
  title: string
  notes: InvoiceDetailResponse['creditNotes']
  invoice: InvoiceDetailResponse
  showOperator: boolean
  children: React.ReactNode
}

function TrackCard({ track, title, notes, invoice, showOperator, children }: TrackCardProps) {
  const color = getCreditNoteSourceColorClass(track)
  const isManual = track === 'manual'
  const testId = isManual ? 'credit-note-list-manual' : 'credit-note-list-stripe'

  return (
    <Card className={`border-t-4 ${color.bar}`} data-testid={testId}>
      <CardHeader>
        <div className="flex items-center gap-2">
          <CardTitle className="text-sm">{title}</CardTitle>
          <Badge variant="secondary" className={color.badge}>
            {notes.length}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{m['billing.credit_note_col_number']()}</TableHead>
              {isManual && <TableHead>{m['billing.credit_note_col_reason']()}</TableHead>}
              {isManual && showOperator && (
                <TableHead>{m['billing.credit_note_col_operator']()}</TableHead>
              )}
              <TableHead>{m['billing.credit_note_col_issued']()}</TableHead>
              <TableHead>{m['billing.credit_note_col_status']()}</TableHead>
              <TableHead className="text-right">{m['billing.credit_note_col_amount']()}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {notes.length === 0 ? (
              <TableRow>
                <TableCell
                  colSpan={isManual ? (showOperator ? 6 : 5) : 4}
                  className="text-center text-muted-foreground"
                >
                  {m['billing.credit_note_list_empty']()}
                </TableCell>
              </TableRow>
            ) : (
              notes.map((note) => (
                <CreditNoteRow
                  key={note.id}
                  note={note}
                  track={track}
                  currency={invoice.currency}
                  showOperator={showOperator}
                />
              ))
            )}
          </TableBody>
        </Table>
        {children}
      </CardContent>
    </Card>
  )
}

function StripeViewLink({ invoice }: { invoice: InvoiceDetailResponse }) {
  const url = getViewInProviderUrl(invoice)
  if (!url) return null

  return (
    <button
      type="button"
      onClick={() => window.open(url, '_blank', 'noopener,noreferrer')}
      className="inline-flex items-center gap-1 text-primary hover:underline"
      data-testid="credit-note-view-in-stripe"
    >
      {m['billing.invoice_view_in_provider']({ provider: getProviderLabel(invoice.provider) })}
      <ExternalLink className="h-3 w-3" />
    </button>
  )
}
