import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from '@/components/ui/pagination'
import { ChevronLeftIcon, ChevronRightIcon } from '@radix-ui/react-icons'

interface PaginationInfo {
  page: number
  pageSize: number
  total: number
}

interface UserPaginationProps {
  pagination: PaginationInfo
  onPageChange: (page: number) => void
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

export function UserPagination({ pagination, onPageChange }: UserPaginationProps) {
  const { page, pageSize, total } = pagination
  const totalPages = Math.ceil(total / pageSize)

  const pageNumbers = getPageNumbers(page, totalPages)

  return (
    <Pagination data-testid="user-pagination">
      <PaginationContent>
        <PaginationItem>
          {page === 0 ? (
            <span
              className={`
                ${page === 0 ? 'opacity-50 cursor-not-allowed' : ''}
                inline-flex items-center gap-1 pl-2.5
              `}
              data-testid="user-pagination-previous"
            >
              <ChevronLeftIcon className="h-4 w-4" />
              <span>Previous</span>
            </span>
          ) : (
            <PaginationPrevious
              onClick={() => onPageChange(page - 1)}
              data-testid="user-pagination-previous"
            />
          )}
        </PaginationItem>

        {pageNumbers.map((pageNum) => (
          <PaginationItem key={pageNum}>
            <PaginationLink
              onClick={() => onPageChange(pageNum)}
              isActive={page === pageNum}
              data-testid={`user-pagination-page-${pageNum}`}
            >
              {pageNum + 1}
            </PaginationLink>
          </PaginationItem>
        ))}

        <PaginationItem>
          {page >= totalPages - 1 ? (
            <span
              className={`
                ${page >= totalPages - 1 ? 'opacity-50 cursor-not-allowed' : ''}
                inline-flex items-center gap-1 pr-2.5
              `}
              data-testid="user-pagination-next"
            >
              <span>Next</span>
              <ChevronRightIcon className="h-4 w-4" />
            </span>
          ) : (
            <PaginationNext
              onClick={() => onPageChange(page + 1)}
              data-testid="user-pagination-next"
            />
          )}
        </PaginationItem>
      </PaginationContent>
    </Pagination>
  )
}
