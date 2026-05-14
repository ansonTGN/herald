import type { ColumnDef } from '@tanstack/react-table'
import { DataTable } from '@/components/shared'
import { Badge } from '@/components/ui/badge'
import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from '@/components/ui/pagination'
import { ChevronLeftIcon, ChevronRightIcon } from '@radix-ui/react-icons'
import type { AuditEventResponse } from '@/lib/api-generated'
import { formatDateTime } from '@/lib/date-utils'

interface AuditEventTableProps {
  data: AuditEventResponse[]
  total: number
  page: number
  pageSize: number
  onRowClick: (event: AuditEventResponse) => void
  onPageChange: (page: number) => void
  emptyMessage?: string
}

function getPageNumbers(currentPage: number, totalPages: number): number[] {
  const maxVisible = 5
  let startPage = Math.max(0, currentPage - 2)
  let endPage = Math.min(totalPages - 1, startPage + maxVisible - 1)

  if (endPage - startPage < maxVisible - 1) {
    startPage = Math.max(0, endPage - maxVisible + 1)
  }

  const pages: number[] = []
  for (let i = startPage; i <= endPage; i++) {
    pages.push(i)
  }
  return pages
}

const columns: ColumnDef<AuditEventResponse>[] = [
  {
    accessorKey: 'createdAt',
    header: 'Time',
    cell: ({ getValue }) => (
      <span className="whitespace-nowrap text-sm">{formatDateTime(getValue() as string)}</span>
    ),
  },
  {
    accessorKey: 'actorName',
    header: 'Actor',
    cell: ({ row }) => {
      const name = row.original.actorName
      const id = row.original.actorId
      return (
        <div className="max-w-[200px]">
          <div className="truncate text-sm font-medium">{name || 'Unknown'}</div>
          <div className="truncate text-xs text-muted-foreground">{id}</div>
        </div>
      )
    },
  },
  {
    accessorKey: 'category',
    header: 'Category',
    cell: ({ getValue }) => (
      <span className="text-sm">{(getValue() as string).replace(/_/g, ' ')}</span>
    ),
  },
  {
    accessorKey: 'action',
    header: 'Action',
    cell: ({ getValue }) => <span className="font-mono text-sm">{getValue() as string}</span>,
  },
  {
    id: 'target',
    header: 'Target',
    cell: ({ row }) => {
      const name = row.original.targetName
      const id = row.original.targetId
      const type = row.original.targetType
      return (
        <div className="max-w-[200px]">
          <div className="truncate text-sm font-medium">{name || 'Unknown'}</div>
          <div className="truncate text-xs text-muted-foreground">
            {type}: {id}
          </div>
        </div>
      )
    },
  },
  {
    accessorKey: 'result',
    header: 'Result',
    cell: ({ getValue }) => {
      const result = getValue() as string
      const isSuccess = result === 'success'
      return (
        <Badge
          variant={isSuccess ? 'default' : 'destructive'}
          className="text-xs"
          data-testid={`audit-result-${result}`}
        >
          {result}
        </Badge>
      )
    },
  },
  {
    accessorKey: 'ipAddress',
    header: 'IP Address',
    cell: ({ getValue }) => (
      <span className="font-mono text-sm">{(getValue() as string) || '-'}</span>
    ),
  },
]

export function AuditEventTable({
  data,
  total,
  page,
  pageSize,
  onRowClick,
  onPageChange,
  emptyMessage = 'No audit logs yet.',
}: AuditEventTableProps) {
  const totalPages = Math.ceil(total / pageSize)
  const pageNumbers = getPageNumbers(page, totalPages)

  return (
    <div className="space-y-4">
      <DataTable
        columns={columns}
        data={data}
        onRowClick={onRowClick}
        emptyMessage={emptyMessage}
        data-testid="audit-table"
      />

      {totalPages > 1 && (
        <Pagination data-testid="audit-pagination">
          <PaginationContent>
            <PaginationItem>
              {page === 0 ? (
                <span
                  className="inline-flex cursor-not-allowed items-center gap-1 pl-2.5 opacity-50"
                  data-testid="audit-pagination-previous"
                >
                  <ChevronLeftIcon className="h-4 w-4" />
                  <span>Previous</span>
                </span>
              ) : (
                <PaginationPrevious
                  onClick={() => onPageChange(page - 1)}
                  data-testid="audit-pagination-previous"
                />
              )}
            </PaginationItem>

            {pageNumbers.map((pageNum) => (
              <PaginationItem key={pageNum}>
                <PaginationLink
                  onClick={() => onPageChange(pageNum)}
                  isActive={page === pageNum}
                  data-testid={`audit-pagination-page-${pageNum}`}
                >
                  {pageNum + 1}
                </PaginationLink>
              </PaginationItem>
            ))}

            <PaginationItem>
              {page >= totalPages - 1 ? (
                <span
                  className="inline-flex cursor-not-allowed items-center gap-1 pr-2.5 opacity-50"
                  data-testid="audit-pagination-next"
                >
                  <span>Next</span>
                  <ChevronRightIcon className="h-4 w-4" />
                </span>
              ) : (
                <PaginationNext
                  onClick={() => onPageChange(page + 1)}
                  data-testid="audit-pagination-next"
                />
              )}
            </PaginationItem>
          </PaginationContent>
        </Pagination>
      )}
    </div>
  )
}
