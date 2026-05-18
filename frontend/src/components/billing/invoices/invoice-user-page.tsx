import { useState, useEffect, useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { type ColumnDef } from '@tanstack/react-table'
import { Download, Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { DataTable } from '@/components/shared/data-table'
import { PageHeader } from '@/components/shared'
import { BaseFormDialog } from '@/components/shared/form-dialog'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { TextField, TextareaField } from '@/components/shared/form-fields'
import { InvoiceStatusBadge } from '@/components/billing/invoices/invoice-status-badge'
import { ListPagination } from '@/components/shared'
import type { InvoiceResponse } from '@/lib/api-generated'
import { myInvoiceListQueryOptions } from '@/data/invoice-query-options'
import { useApplyInvoice } from '@/data/invoice-mutations'
import { applyInvoiceSchema, getApplyFormDefaults } from '@/lib/schemas/invoice-forms'
import {
  formatInvoiceAmount,
  downloadInvoicePdf,
  INVOICE_PAGE_SIZE,
  PDF_DOWNLOADABLE_STATUSES,
} from '@/lib/invoice-utils'

function createInvoiceColumns(
  realmId: string,
  page: number,
  pageSize: number
): ColumnDef<InvoiceResponse>[] {
  return [
    {
      id: 'rowNumber',
      header: '#',
      cell: ({ row }) => (
        <span className="text-muted-foreground text-sm">{page * pageSize + row.index + 1}</span>
      ),
    },
    {
      accessorKey: 'invoiceNumber',
      header: 'Invoice Number',
      cell: ({ row }) => (
        <span className="font-mono text-sm font-medium">{row.getValue('invoiceNumber')}</span>
      ),
    },
    {
      accessorKey: 'total',
      header: 'Amount',
      cell: ({ row }) => {
        const total = row.getValue('total') as number
        const currency = row.original.currency
        return <span className="font-mono text-sm">{formatInvoiceAmount(total, currency)}</span>
      },
    },
    {
      accessorKey: 'status',
      header: 'Status',
      cell: ({ row }) => <InvoiceStatusBadge status={row.getValue('status') as string} />,
    },
    {
      accessorKey: 'dueDate',
      header: 'Due Date',
      cell: ({ row }) => {
        const dueDate = row.getValue('dueDate') as string
        return <span className="text-sm">{new Date(dueDate).toLocaleDateString()}</span>
      },
    },
    {
      id: 'actions',
      header: 'Actions',
      cell: ({ row }) => {
        const invoice = row.original
        const canDownload = PDF_DOWNLOADABLE_STATUSES.has(invoice.status)
        if (!canDownload) return null
        return (
          <Button
            variant="ghost"
            size="sm"
            onClick={() =>
              downloadInvoicePdf(
                `/api/bill/${realmId}/my/invoices/${invoice.id}/pdf`,
                `${invoice.invoiceNumber}.pdf`
              )
            }
            data-testid={`invoice-download-pdf-${invoice.id}`}
          >
            <Download className="mr-1 h-4 w-4" />
            PDF
          </Button>
        )
      },
    },
  ]
}

function ApplyInvoiceDialog({
  open,
  onOpenChange,
  realmId,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  realmId: string
}) {
  const { mutate: apply, isPending: isSubmitting } = useApplyInvoice(realmId)
  const defaultValues = useMemo(() => getApplyFormDefaults(), [])

  const form = useAppForm({
    schema: applyInvoiceSchema,
    defaultValues,
    onSubmit: async ({ value }) => {
      apply(value, {
        onSuccess: () => {
          onOpenChange(false)
        },
      })
    },
  })

  useEffect(() => {
    if (open) {
      form.reset(defaultValues)
    }
  }, [defaultValues, form, open])

  return (
    <BaseFormDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Apply for Invoice"
      description="Submit a request for an invoice. Seller info will be auto-filled from Realm config."
      data-testid="apply-invoice-dialog"
      footer={
        <>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
            data-testid="apply-invoice-cancel-button"
          >
            Cancel
          </Button>
          <Button
            type="submit"
            form="apply-invoice-form"
            disabled={isSubmitting}
            data-testid="apply-invoice-submit-button"
          >
            {isSubmitting ? 'Submitting...' : 'Submit'}
          </Button>
        </>
      }
    >
      <form
        id="apply-invoice-form"
        onSubmit={(e) => {
          e.preventDefault()
          e.stopPropagation()
          form.handleSubmit()
        }}
      >
        <AppForm>
          <div className="space-y-4">
            <TextField
              form={form}
              name="paymentAttemptId"
              label="Payment Attempt ID"
              dataTestId="apply-payment-attempt-id-input"
              placeholder="Enter payment attempt ID"
            />
            <TextField
              form={form}
              name="subscriptionId"
              label="Subscription ID"
              dataTestId="apply-subscription-id-input"
              placeholder="Enter subscription ID"
            />
            <div className="rounded-md border border-muted bg-muted/40 px-3 py-2">
              <p className="text-xs text-muted-foreground">
                At least one of Payment Attempt ID or Subscription ID is required.
              </p>
            </div>
            <TextField
              form={form}
              name="billingName"
              label="Billing Name"
              dataTestId="apply-billing-name-input"
              placeholder="Name on invoice"
              required
            />
            <TextField
              form={form}
              name="billingEmail"
              label="Billing Email"
              dataTestId="apply-billing-email-input"
              type="email"
              placeholder="billing@example.com"
            />
            <TextField
              form={form}
              name="billingAddress"
              label="Billing Address"
              dataTestId="apply-billing-address-input"
              placeholder="Billing address"
              required
            />
            <TextField
              form={form}
              name="billingPhone"
              label="Billing Phone"
              dataTestId="apply-billing-phone-input"
              placeholder="+1 234 567 8900"
            />
            <TextField
              form={form}
              name="dueDate"
              label="Due Date"
              dataTestId="apply-due-date-input"
              type="date"
              required
            />
            <TextareaField
              form={form}
              name="notes"
              label="Notes"
              dataTestId="apply-notes-input"
              placeholder="Additional notes for this invoice request"
              rows={3}
            />
            <div className="rounded-md border border-muted bg-muted/40 px-3 py-2">
              <p className="text-xs text-muted-foreground">
                Seller info will be auto-filled from Realm config.
              </p>
            </div>
          </div>
        </AppForm>
      </form>
    </BaseFormDialog>
  )
}

export function InvoiceUserPage({ realmId }: { realmId: string }) {
  const [page, setPage] = useState(0)
  const [applyDialogOpen, setApplyDialogOpen] = useState(false)

  const { data, isLoading, error } = useQuery(
    myInvoiceListQueryOptions(realmId, {
      page,
      pageSize: INVOICE_PAGE_SIZE,
    })
  )

  const invoices = data?.data ?? []
  const total = data?.total ?? 0

  const columns = useMemo(
    () => createInvoiceColumns(realmId, page, INVOICE_PAGE_SIZE),
    [realmId, page]
  )

  return (
    <div className="space-y-6" data-testid="invoice-user-page">
      <PageHeader
        title="My Invoices"
        headingTestId="invoice-user-heading"
        action={{
          label: 'Apply for Invoice',
          onClick: () => setApplyDialogOpen(true),
          testId: 'apply-invoice-button',
          icon: <Plus className="mr-2 h-4 w-4" />,
        }}
      />

      <Card>
        <CardHeader>
          <CardTitle>Invoices</CardTitle>
        </CardHeader>
        <CardContent>
          <DataTable
            columns={columns}
            data={invoices}
            isLoading={isLoading}
            error={error instanceof Error ? error : error ? new Error(String(error)) : undefined}
            loadingMessage="Loading invoices..."
            emptyMessage="No invoices yet."
            data-testid="invoice-user-table"
          />
        </CardContent>
      </Card>

      {total > 0 && (
        <ListPagination
          page={page}
          pageSize={INVOICE_PAGE_SIZE}
          total={total}
          onPageChange={setPage}
          testIdPrefix="invoice-user-pagination"
        />
      )}

      <ApplyInvoiceDialog
        open={applyDialogOpen}
        onOpenChange={setApplyDialogOpen}
        realmId={realmId}
      />
    </div>
  )
}
