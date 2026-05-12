import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from '@/components/ui/pagination'

const DEFAULT_MAX_VISIBLE = 7

export function InvoicePagination({
  page,
  pageSize,
  total,
  onPageChange,
  testIdPrefix = 'invoice',
}: {
  page: number
  pageSize: number
  total: number
  onPageChange: (page: number) => void
  testIdPrefix?: string
}) {
  const totalPages = pageSize > 0 ? Math.ceil(total / pageSize) : 0

  const getPageNumbers = () => {
    const start = Math.max(0, page - Math.floor(DEFAULT_MAX_VISIBLE / 2))
    const end = Math.min(totalPages, start + DEFAULT_MAX_VISIBLE)
    return Array.from({ length: end - start }, (_, i) => start + i)
  }

  const pageNumbers = getPageNumbers()

  return (
    <div className="flex items-center justify-between" data-testid={`${testIdPrefix}-pagination`}>
      <div className="text-sm text-muted-foreground">
        Showing {Math.min(page * pageSize + 1, total)} to {Math.min((page + 1) * pageSize, total)}{' '}
        of {total} results
      </div>

      <Pagination>
        <PaginationContent>
          <PaginationItem>
            <PaginationPrevious
              onClick={() => page > 0 && onPageChange(page - 1)}
              className={page === 0 ? 'opacity-50 pointer-events-none' : undefined}
              data-testid={`${testIdPrefix}-pagination-previous`}
            />
          </PaginationItem>

          {pageNumbers.map((pageNum) => (
            <PaginationItem key={pageNum}>
              <PaginationLink
                onClick={() => onPageChange(pageNum)}
                isActive={page === pageNum}
                data-testid={`${testIdPrefix}-pagination-page-${pageNum}`}
              >
                {pageNum + 1}
              </PaginationLink>
            </PaginationItem>
          ))}

          <PaginationItem>
            <PaginationNext
              onClick={() => page < totalPages - 1 && onPageChange(page + 1)}
              className={page >= totalPages - 1 ? 'opacity-50 pointer-events-none' : undefined}
              data-testid={`${testIdPrefix}-pagination-next`}
            />
          </PaginationItem>
        </PaginationContent>
      </Pagination>
    </div>
  )
}
