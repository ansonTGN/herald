import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from '@/components/ui/pagination'

interface PlanPaginationProps {
  pagination: {
    page: number
    pageSize: number
    total: number
  }
  onPageChange: (page: number) => void
}

export function PlanPagination({ pagination, onPageChange }: PlanPaginationProps) {
  const { page, pageSize, total } = pagination
  const totalPages = pageSize > 0 ? Math.ceil(total / pageSize) : 0

  const handlePageChange = (newPage: number) => {
    if (newPage >= 0 && newPage < totalPages) {
      onPageChange(newPage)
    }
  }

  // Performance optimization: only render page numbers near current page
  const getPageNumbers = () => {
    const maxVisiblePages = 7
    const startPage = Math.max(0, page - Math.floor(maxVisiblePages / 2))
    const endPage = Math.min(totalPages, startPage + maxVisiblePages)

    return Array.from({ length: endPage - startPage }, (_, i) => startPage + i)
  }

  const pageNumbers = getPageNumbers()

  return (
    <div className="flex items-center justify-between">
      <div className="text-sm text-muted-foreground">
        Showing {Math.min(page * pageSize + 1, total)} to {Math.min((page + 1) * pageSize, total)}{' '}
        of {total} results
      </div>

      <Pagination data-testid="plan-pagination">
        <PaginationContent>
          <PaginationItem>
            <PaginationPrevious
              onClick={() => page > 0 && handlePageChange(page - 1)}
              className={page === 0 ? 'opacity-50 pointer-events-none' : undefined}
              data-testid="plan-pagination-previous"
            />
          </PaginationItem>

          {pageNumbers.map((pageNum) => (
            <PaginationItem key={pageNum}>
              <PaginationLink
                onClick={() => handlePageChange(pageNum)}
                isActive={page === pageNum}
                data-testid={`plan-pagination-page-${pageNum}`}
              >
                {pageNum + 1}
              </PaginationLink>
            </PaginationItem>
          ))}

          <PaginationItem>
            <PaginationNext
              onClick={() => page < totalPages - 1 && handlePageChange(page + 1)}
              className={page >= totalPages - 1 ? 'opacity-50 pointer-events-none' : undefined}
              data-testid="plan-pagination-next"
            />
          </PaginationItem>
        </PaginationContent>
      </Pagination>
    </div>
  )
}
