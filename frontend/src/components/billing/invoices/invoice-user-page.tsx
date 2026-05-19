import { useState, useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { type ColumnDef } from '@tanstack/react-table'
import { Download, Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { DataTable } from '@/components/shared/data-table'
import { PageHeader } from '@/components/shared'
import { InvoiceStatusBadge } from '@/components/billing/invoices/invoice-status-badge'
import { ListPagination } from '@/components/shared'
import type { InvoiceResponse } from '@/lib/api-generated'
import { myInvoiceListQueryOptions } from '@/data/invoice-query-options'
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

export function InvoiceUserPage({
  realmId,
  onApplyInvoice,
}: {
  realmId: string
  onApplyInvoice: () => void
}) {
  const [page, setPage] = useState(0)

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
          onClick: onApplyInvoice,
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
    </div>
  )
}
