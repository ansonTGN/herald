import { useState, useCallback } from 'react'
import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { InvoiceAdminPage } from '@/components/billing/invoices/invoice-admin-page'
import { InvoiceDetailDialog } from '@/components/billing/invoices/invoice-detail-dialog'
import { InvoiceSellerConfigForm } from '@/components/billing/invoices/invoice-seller-config-form'
import { useIssueInvoice, useVoidInvoice, useMarkPaid } from '@/data/invoice-mutations'
import type { InvoiceResponse } from '@/lib/api-generated'

export const Route = createFileRoute('/$realmId/manage/billing/invoices/')({
  component: InvoiceAdminRoute,
})

function InvoiceAdminRoute() {
  const { realmId } = Route.useParams()
  const navigate = useNavigate()

  const [detailInvoiceId, setDetailInvoiceId] = useState<string | null>(null)

  const [sellerConfigOpen, setSellerConfigOpen] = useState(false)

  const [issueTarget, setIssueTarget] = useState<InvoiceResponse | null>(null)
  const [issueDate, setIssueDate] = useState('')

  const [voidTarget, setVoidTarget] = useState<InvoiceResponse | null>(null)
  const [voidReason, setVoidReason] = useState('')

  const [markPaidTarget, setMarkPaidTarget] = useState<InvoiceResponse | null>(null)

  const issueMutation = useIssueInvoice(realmId)
  const voidMutation = useVoidInvoice(realmId)
  const markPaidMutation = useMarkPaid(realmId)

  const handleCreateInvoice = useCallback(() => {
    navigate({
      to: '/$realmId/manage/billing/invoices/new',
      params: { realmId },
    })
  }, [navigate, realmId])

  const handleEditInvoice = useCallback(
    (invoice: InvoiceResponse) => {
      navigate({
        to: '/$realmId/manage/billing/invoices/$invoiceId/edit',
        params: { realmId, invoiceId: invoice.id },
      })
    },
    [navigate, realmId]
  )

  const handleViewInvoice = useCallback((invoice: InvoiceResponse) => {
    setDetailInvoiceId(invoice.id)
  }, [])

  const handleOpenSellerConfig = useCallback(() => {
    setSellerConfigOpen(true)
  }, [])

  const handleIssueInvoice = useCallback((invoice: InvoiceResponse) => {
    setIssueTarget(invoice)
    setIssueDate('')
  }, [])

  const handleVoidInvoice = useCallback((invoice: InvoiceResponse) => {
    setVoidTarget(invoice)
    setVoidReason('')
  }, [])

  const handleMarkPaidInvoice = useCallback((invoice: InvoiceResponse) => {
    setMarkPaidTarget(invoice)
  }, [])

  return (
    <>
      <InvoiceAdminPage
        realmId={realmId}
        onCreateInvoice={handleCreateInvoice}
        onEditInvoice={handleEditInvoice}
        onViewInvoice={handleViewInvoice}
        onOpenSellerConfig={handleOpenSellerConfig}
        onIssueInvoice={handleIssueInvoice}
        onVoidInvoice={handleVoidInvoice}
        onMarkPaidInvoice={handleMarkPaidInvoice}
      />

      <InvoiceDetailDialog
        open={!!detailInvoiceId}
        onOpenChange={(open) => {
          if (!open) setDetailInvoiceId(null)
        }}
        realmId={realmId}
        invoiceId={detailInvoiceId}
      />

      <InvoiceSellerConfigForm
        open={sellerConfigOpen}
        onOpenChange={setSellerConfigOpen}
        realmId={realmId}
      />

      <AlertDialog
        open={!!issueTarget}
        onOpenChange={(open) => {
          if (!open) setIssueTarget(null)
        }}
      >
        <AlertDialogContent data-testid="issue-confirm-dialog">
          <AlertDialogHeader>
            <AlertDialogTitle>Issue Invoice</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to issue invoice{' '}
              <span className="font-mono font-semibold">{issueTarget?.invoiceNumber}</span>? This
              action will lock the invoice and send it to the buyer.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <div className="space-y-2">
            <Label htmlFor="issue-date" className="text-sm">
              Issue Date (optional, defaults to today)
            </Label>
            <Input
              id="issue-date"
              type="date"
              value={issueDate}
              onChange={(e) => setIssueDate(e.target.value)}
              data-testid="issue-date-input"
            />
          </div>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={issueMutation.isPending} data-testid="issue-cancel-button">
              Cancel
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                issueMutation.mutate(
                  { invoiceId: issueTarget!.id, issueDate: issueDate || undefined },
                  { onSettled: () => setIssueTarget(null) }
                )
              }}
              disabled={issueMutation.isPending}
              data-testid="issue-confirm-button"
            >
              {issueMutation.isPending ? 'Issuing...' : 'Issue'}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      <AlertDialog
        open={!!voidTarget}
        onOpenChange={(open) => {
          if (!open) setVoidTarget(null)
        }}
      >
        <AlertDialogContent data-testid="void-confirm-dialog">
          <AlertDialogHeader>
            <AlertDialogTitle>Void Invoice</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to void invoice{' '}
              <span className="font-mono font-semibold">{voidTarget?.invoiceNumber}</span>? This
              action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <div className="space-y-2">
            <Label htmlFor="void-reason" className="text-sm">
              Reason (optional)
            </Label>
            <Input
              id="void-reason"
              value={voidReason}
              onChange={(e) => setVoidReason(e.target.value)}
              placeholder="Reason for voiding this invoice"
              data-testid="void-reason-input"
            />
          </div>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={voidMutation.isPending} data-testid="void-cancel-button">
              Cancel
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                voidMutation.mutate(
                  { invoiceId: voidTarget!.id, voidReason: voidReason || undefined },
                  { onSettled: () => setVoidTarget(null) }
                )
              }}
              disabled={voidMutation.isPending}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              data-testid="void-confirm-button"
            >
              {voidMutation.isPending ? 'Voiding...' : 'Void'}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      <AlertDialog
        open={!!markPaidTarget}
        onOpenChange={(open) => {
          if (!open) setMarkPaidTarget(null)
        }}
      >
        <AlertDialogContent data-testid="mark-paid-confirm-dialog">
          <AlertDialogHeader>
            <AlertDialogTitle>Mark Invoice as Paid</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to mark invoice{' '}
              <span className="font-mono font-semibold">{markPaidTarget?.invoiceNumber}</span> as
              paid?
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel
              disabled={markPaidMutation.isPending}
              data-testid="mark-paid-cancel-button"
            >
              Cancel
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                markPaidMutation.mutate(
                  { invoiceId: markPaidTarget!.id },
                  { onSettled: () => setMarkPaidTarget(null) }
                )
              }}
              disabled={markPaidMutation.isPending}
              data-testid="mark-paid-confirm-button"
            >
              {markPaidMutation.isPending ? 'Marking...' : 'Mark as Paid'}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  )
}
