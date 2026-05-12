import { useState, useCallback, useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { type ColumnDef } from '@tanstack/react-table'
import {
  MoreHorizontal,
  Eye,
  Pencil,
  Send,
  Ban,
  CheckCircle,
  Download,
  Settings,
  Plus,
  Search,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { DataTable } from '@/components/shared/data-table'
import { InvoiceStatusBadge } from '@/components/billing/invoices/invoice-status-badge'
import { InvoicePagination } from '@/components/billing/invoices/invoice-pagination'
import type { InvoiceResponse } from '@/lib/api-generated'
import { invoiceListQueryOptions } from '@/data/invoice-query-options'
import {
  INVOICE_PAGE_SIZE,
  INVOICE_SOURCE_LABELS,
  formatInvoiceAmount,
  downloadInvoicePdf,
  getAvailableActions,
} from '@/lib/invoice-utils'

interface InvoiceFilters {
  status?: string
  source?: string
  dateFrom?: string
  dateTo?: string
  search?: string
}

interface InvoiceAdminPageProps {
  realmId: string
  onViewInvoice?: (invoice: InvoiceResponse) => void
  onEditInvoice?: (invoice: InvoiceResponse) => void
  onCreateInvoice?: () => void
  onOpenSellerConfig?: () => void
  onIssueInvoice?: (invoice: InvoiceResponse) => void
  onVoidInvoice?: (invoice: InvoiceResponse) => void
  onMarkPaidInvoice?: (invoice: InvoiceResponse) => void
}

function createInvoiceColumns(
  realmId: string,
  handlers: {
    onView?: (invoice: InvoiceResponse) => void
    onEdit?: (invoice: InvoiceResponse) => void
    onIssue?: (invoice: InvoiceResponse) => void
    onVoid?: (invoice: InvoiceResponse) => void
    onMarkPaid?: (invoice: InvoiceResponse) => void
  },
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
      accessorKey: 'billingName',
      header: 'Buyer',
      cell: ({ row }) => <span className="text-sm">{row.getValue('billingName')}</span>,
    },
    {
      accessorKey: 'source',
      header: 'Source',
      cell: ({ row }) => {
        const source = row.getValue('source') as string
        return <Badge variant="outline">{INVOICE_SOURCE_LABELS[source] ?? source}</Badge>
      },
    },
    {
      accessorKey: 'status',
      header: 'Status',
      cell: ({ row }) => <InvoiceStatusBadge status={row.getValue('status') as string} />,
    },
    {
      accessorKey: 'total',
      header: 'Total',
      cell: ({ row }) => {
        const total = row.getValue('total') as number
        const currency = row.original.currency
        return <span className="font-mono text-sm">{formatInvoiceAmount(total, currency)}</span>
      },
    },
    {
      accessorKey: 'dueDate',
      header: 'Due Date',
      cell: ({ row }) => {
        const dueDate = row.getValue('dueDate') as string | null | undefined
        if (!dueDate) return <span className="text-muted-foreground">-</span>
        return <span className="text-sm">{new Date(dueDate).toLocaleDateString()}</span>
      },
    },
    {
      accessorKey: 'createdAt',
      header: 'Created At',
      cell: ({ row }) => {
        const createdAt = row.getValue('createdAt') as string
        return <span className="text-sm">{new Date(createdAt).toLocaleDateString()}</span>
      },
    },
    {
      id: 'actions',
      header: 'Actions',
      cell: ({ row }) => {
        const invoice = row.original
        const availableActions = getAvailableActions(invoice.status)
        return (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                className="h-8 w-8 p-0"
                data-testid={`invoice-actions-menu-${invoice.id}`}
              >
                <span className="sr-only">Open menu</span>
                <MoreHorizontal className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              {availableActions.includes('view') && (
                <DropdownMenuItem
                  onClick={() => handlers.onView?.(invoice)}
                  data-testid={`invoice-view-${invoice.id}`}
                >
                  <Eye className="mr-2 h-4 w-4" />
                  View
                </DropdownMenuItem>
              )}
              {availableActions.includes('edit') && (
                <DropdownMenuItem
                  onClick={() => handlers.onEdit?.(invoice)}
                  data-testid={`invoice-edit-${invoice.id}`}
                >
                  <Pencil className="mr-2 h-4 w-4" />
                  Edit
                </DropdownMenuItem>
              )}
              {availableActions.includes('issue') && (
                <DropdownMenuItem
                  onClick={() => handlers.onIssue?.(invoice)}
                  data-testid={`invoice-issue-${invoice.id}`}
                >
                  <Send className="mr-2 h-4 w-4" />
                  Issue
                </DropdownMenuItem>
              )}
              {availableActions.includes('void') && (
                <>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    onClick={() => handlers.onVoid?.(invoice)}
                    className="text-destructive"
                    data-testid={`invoice-void-${invoice.id}`}
                  >
                    <Ban className="mr-2 h-4 w-4" />
                    Void
                  </DropdownMenuItem>
                </>
              )}
              {availableActions.includes('markPaid') && (
                <DropdownMenuItem
                  onClick={() => handlers.onMarkPaid?.(invoice)}
                  data-testid={`invoice-mark-paid-${invoice.id}`}
                >
                  <CheckCircle className="mr-2 h-4 w-4" />
                  Mark Paid
                </DropdownMenuItem>
              )}
              {availableActions.includes('downloadPdf') && (
                <>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    onClick={() =>
                      downloadInvoicePdf(
                        `/api/bill/${realmId}/invoices/${invoice.id}/pdf`,
                        `${invoice.invoiceNumber}.pdf`
                      )
                    }
                    data-testid={`invoice-download-pdf-${invoice.id}`}
                  >
                    <Download className="mr-2 h-4 w-4" />
                    Download PDF
                  </DropdownMenuItem>
                </>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        )
      },
    },
  ]
}

const STATUS_OPTIONS = [
  { value: 'all', label: 'All Statuses' },
  { value: 'draft', label: 'Draft' },
  { value: 'issued', label: 'Issued' },
  { value: 'paid', label: 'Paid' },
  { value: 'overdue', label: 'Overdue' },
  { value: 'void', label: 'Void' },
]

const SOURCE_OPTIONS = [
  { value: 'all', label: 'All Sources' },
  { value: 'admin_manual', label: 'Manual' },
  { value: 'user_application', label: 'Application' },
]

function FilterBar({
  filters,
  onFiltersChange,
}: {
  filters: InvoiceFilters
  onFiltersChange: (filters: InvoiceFilters) => void
}) {
  return (
    <div className="flex flex-wrap items-center gap-3" data-testid="invoice-filter-bar">
      <Select
        value={filters.status ?? 'all'}
        onValueChange={(value) =>
          onFiltersChange({ ...filters, status: value === 'all' ? undefined : value })
        }
      >
        <SelectTrigger className="w-[160px]" data-testid="invoice-status-filter">
          <SelectValue placeholder="All Statuses" />
        </SelectTrigger>
        <SelectContent>
          {STATUS_OPTIONS.map((opt) => (
            <SelectItem key={opt.value} value={opt.value}>
              {opt.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      <Select
        value={filters.source ?? 'all'}
        onValueChange={(value) =>
          onFiltersChange({ ...filters, source: value === 'all' ? undefined : value })
        }
      >
        <SelectTrigger className="w-[160px]" data-testid="invoice-source-filter">
          <SelectValue placeholder="All Sources" />
        </SelectTrigger>
        <SelectContent>
          {SOURCE_OPTIONS.map((opt) => (
            <SelectItem key={opt.value} value={opt.value}>
              {opt.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      <Input
        type="date"
        className="w-[150px]"
        value={filters.dateFrom ?? ''}
        onChange={(e) => onFiltersChange({ ...filters, dateFrom: e.target.value || undefined })}
        data-testid="invoice-date-from-filter"
      />
      <Input
        type="date"
        className="w-[150px]"
        value={filters.dateTo ?? ''}
        onChange={(e) => onFiltersChange({ ...filters, dateTo: e.target.value || undefined })}
        data-testid="invoice-date-to-filter"
      />

      <div className="relative">
        <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
        <Input
          placeholder="Search invoices..."
          className="w-[220px] pl-9"
          value={filters.search ?? ''}
          onChange={(e) => onFiltersChange({ ...filters, search: e.target.value || undefined })}
          data-testid="invoice-search-input"
        />
      </div>
    </div>
  )
}

export function InvoiceAdminPage({
  realmId,
  onViewInvoice,
  onEditInvoice,
  onCreateInvoice,
  onOpenSellerConfig,
  onIssueInvoice,
  onVoidInvoice,
  onMarkPaidInvoice,
}: InvoiceAdminPageProps) {
  const [filters, setFilters] = useState<InvoiceFilters>({})
  const [page, setPage] = useState(0)

  const queryFilters = {
    ...filters,
    page,
    pageSize: INVOICE_PAGE_SIZE,
  }

  const { data, isLoading, error } = useQuery(invoiceListQueryOptions(realmId, queryFilters))

  const invoices = data?.data ?? []
  const total = data?.total ?? 0

  const handleFiltersChange = useCallback((newFilters: InvoiceFilters) => {
    setFilters(newFilters)
    setPage(0)
  }, [])

  const actionHandlers = useMemo(
    () => ({
      onView: onViewInvoice,
      onEdit: onEditInvoice,
      onIssue: onIssueInvoice,
      onVoid: onVoidInvoice,
      onMarkPaid: onMarkPaidInvoice,
    }),
    [onViewInvoice, onEditInvoice, onIssueInvoice, onVoidInvoice, onMarkPaidInvoice]
  )

  const columns = useMemo(
    () => createInvoiceColumns(realmId, actionHandlers, page, INVOICE_PAGE_SIZE),
    [realmId, actionHandlers, page]
  )

  return (
    <div className="space-y-6" data-testid="invoice-admin-page">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold" data-testid="invoice-heading">
            Invoices
          </h1>
          <p className="text-muted-foreground mt-1">
            Manage invoices, review applications, and track payments
          </p>
        </div>
        <div className="flex gap-2">
          {onOpenSellerConfig && (
            <Button
              variant="outline"
              onClick={onOpenSellerConfig}
              data-testid="seller-config-button"
            >
              <Settings className="mr-2 h-4 w-4" />
              Seller Config
            </Button>
          )}
          {onCreateInvoice && (
            <Button onClick={onCreateInvoice} data-testid="create-invoice-button">
              <Plus className="mr-2 h-4 w-4" />
              Create Invoice
            </Button>
          )}
        </div>
      </div>

      <FilterBar filters={filters} onFiltersChange={handleFiltersChange} />

      <Card>
        <CardHeader>
          <CardTitle>Invoice List</CardTitle>
        </CardHeader>
        <CardContent>
          <DataTable
            columns={columns}
            data={invoices}
            isLoading={isLoading}
            error={error instanceof Error ? error : error ? new Error(String(error)) : undefined}
            loadingMessage="Loading invoices..."
            emptyMessage="No invoices yet."
            data-testid="invoice-table"
          />
        </CardContent>
      </Card>

      {total > 0 && (
        <InvoicePagination
          page={page}
          pageSize={INVOICE_PAGE_SIZE}
          total={total}
          onPageChange={setPage}
        />
      )}
    </div>
  )
}
