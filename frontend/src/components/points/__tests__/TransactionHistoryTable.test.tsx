import { describe, it, expect, afterEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { TransactionHistoryTable } from '../TransactionHistoryTable'
import {
  mockRechargeTransaction,
  mockConsumeTransaction,
} from '@/fixtures/points-transaction.fixture'
import type { TransactionFilters } from '@/lib/schemas/points-forms'

describe('TransactionHistoryTable', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  const mockOnFiltersChange = vi.fn()
  const mockOnPaginationChange = vi.fn()
  const mockFilters: TransactionFilters = {
    transaction_type: undefined,
    start_time: undefined,
    end_time: undefined,
    client_app_id: undefined,
  }
  const mockPagination = {
    page: 1,
    pageSize: 10,
    total: 2,
  }
  const mockClientApps = [{ id: 'app-123', name: 'Test App' }]

  describe('empty state', () => {
    it('GIVEN no transactions WHEN rendering THEN should show empty state', () => {
      render(
        <TransactionHistoryTable
          transactions={[]}
          filters={mockFilters}
          onFiltersChange={mockOnFiltersChange}
          pagination={{ page: 1, pageSize: 10, total: 0 }}
          onPaginationChange={mockOnPaginationChange}
        />
      )

      expect(screen.getByTestId('no-transactions')).toBeInTheDocument()
      expect(screen.getByText('没有找到符合条件的交易记录')).toBeInTheDocument()
    })

    it('GIVEN no transactions with active filters WHEN rendering THEN should show filter hint', () => {
      const activeFilters: TransactionFilters = {
        transactionType: 'recharge',
      }
      render(
        <TransactionHistoryTable
          transactions={[]}
          filters={activeFilters}
          onFiltersChange={mockOnFiltersChange}
          pagination={{ page: 1, pageSize: 10, total: 0 }}
          onPaginationChange={mockOnPaginationChange}
        />
      )

      expect(screen.getByText('尝试调整筛选条件')).toBeInTheDocument()
    })
  })

  describe('rendering transactions', () => {
    it('GIVEN recharge transaction WHEN rendering THEN should display with green color and plus sign', () => {
      render(
        <TransactionHistoryTable
          transactions={[mockRechargeTransaction]}
          filters={mockFilters}
          onFiltersChange={mockOnFiltersChange}
          pagination={{ page: 1, pageSize: 10, total: 1 }}
          onPaginationChange={mockOnPaginationChange}
        />
      )

      const amount = screen.getByTestId('transaction-amount-0')
      expect(amount).toBeInTheDocument()
      expect(amount).toHaveTextContent('+1,000')
      expect(amount).toHaveClass(/text-green-600/)
    })

    it('GIVEN consume transaction WHEN rendering THEN should display with red color and minus sign', () => {
      render(
        <TransactionHistoryTable
          transactions={[mockConsumeTransaction]}
          filters={mockFilters}
          onFiltersChange={mockOnFiltersChange}
          pagination={{ page: 1, pageSize: 10, total: 1 }}
          onPaginationChange={mockOnPaginationChange}
        />
      )

      const amount = screen.getByTestId('transaction-amount-0')
      expect(amount).toBeInTheDocument()
      expect(amount).toHaveTextContent('-500')
      expect(amount).toHaveClass(/text-red-600/)
    })

    it('GIVEN transaction without description WHEN rendering THEN should display dash', () => {
      const transactionWithoutDesc = {
        ...mockRechargeTransaction,
        description: null,
      }
      render(
        <TransactionHistoryTable
          transactions={[transactionWithoutDesc]}
          filters={mockFilters}
          onFiltersChange={mockOnFiltersChange}
          pagination={{ page: 1, pageSize: 10, total: 1 }}
          onPaginationChange={mockOnPaginationChange}
        />
      )

      const description = screen.getByTestId('transaction-description-0')
      expect(description).toHaveTextContent('-')
    })
  })

  describe('admin view', () => {
    it('GIVEN admin view WHEN rendering THEN should display client app column', () => {
      render(
        <TransactionHistoryTable
          transactions={[mockRechargeTransaction]}
          filters={mockFilters}
          onFiltersChange={mockOnFiltersChange}
          pagination={{ page: 1, pageSize: 10, total: 1 }}
          onPaginationChange={mockOnPaginationChange}
          admin={true}
          clientApps={mockClientApps}
        />
      )

      const client = screen.getByTestId('transaction-client-0')
      expect(client).toBeInTheDocument()
      expect(client).toHaveTextContent('Test App')
    })

    it('GIVEN admin view with unknown client app WHEN rendering THEN should display client app ID', () => {
      render(
        <TransactionHistoryTable
          transactions={[mockRechargeTransaction]}
          filters={mockFilters}
          onFiltersChange={mockOnFiltersChange}
          pagination={{ page: 1, pageSize: 10, total: 1 }}
          onPaginationChange={mockOnPaginationChange}
          admin={true}
          clientApps={[]}
        />
      )

      const client = screen.getByTestId('transaction-client-0')
      expect(client).toBeInTheDocument()
      expect(client).toHaveTextContent('app-123')
    })

    it('GIVEN admin view with no client app WHEN rendering THEN should display dash', () => {
      const transactionWithoutClient = {
        ...mockRechargeTransaction,
        clientAppId: null,
      }
      render(
        <TransactionHistoryTable
          transactions={[transactionWithoutClient]}
          filters={mockFilters}
          onFiltersChange={mockOnFiltersChange}
          pagination={{ page: 1, pageSize: 10, total: 1 }}
          onPaginationChange={mockOnPaginationChange}
          admin={true}
          clientApps={mockClientApps}
        />
      )

      const client = screen.getByTestId('transaction-client-0')
      expect(client).toHaveTextContent('-')
    })

    it('GIVEN non-admin view WHEN rendering THEN should not display client app column', () => {
      render(
        <TransactionHistoryTable
          transactions={[mockRechargeTransaction]}
          filters={mockFilters}
          onFiltersChange={mockOnFiltersChange}
          pagination={{ page: 1, pageSize: 10, total: 1 }}
          onPaginationChange={mockOnPaginationChange}
          admin={false}
          clientApps={mockClientApps}
        />
      )

      expect(screen.queryByTestId('transaction-client-0')).not.toBeInTheDocument()
    })
  })

  describe('loading state', () => {
    it('GIVEN loading is true WHEN rendering THEN should show loading skeleton', () => {
      render(
        <TransactionHistoryTable
          transactions={[]}
          loading={true}
          filters={mockFilters}
          onFiltersChange={mockOnFiltersChange}
          pagination={{ page: 1, pageSize: 10, total: 0 }}
          onPaginationChange={mockOnPaginationChange}
        />
      )

      expect(screen.queryByTestId('no-transactions')).not.toBeInTheDocument()
      expect(screen.queryByTestId('transaction-row-0')).not.toBeInTheDocument()
    })
  })

  describe('pagination', () => {
    it('GIVEN user clicks previous button WHEN on page 2 THEN should call onPaginationChange with page 1', async () => {
      const user = userEvent.setup()
      render(
        <TransactionHistoryTable
          transactions={[mockRechargeTransaction]}
          filters={mockFilters}
          onFiltersChange={mockOnFiltersChange}
          pagination={{ page: 2, pageSize: 10, total: 15 }}
          onPaginationChange={mockOnPaginationChange}
        />
      )

      const prevButton = screen.getByTestId('prev-page-button')
      expect(prevButton).not.toBeDisabled()

      await user.click(prevButton)

      // The function is called with the existing pagination object, just page changed
      expect(mockOnPaginationChange).toHaveBeenCalledWith(
        expect.objectContaining({
          page: 1,
          pageSize: 10,
        })
      )
    })

    it('GIVEN user clicks next button WHEN on page 1 THEN should call onPaginationChange with page 2', async () => {
      const user = userEvent.setup()
      render(
        <TransactionHistoryTable
          transactions={[mockRechargeTransaction]}
          filters={mockFilters}
          onFiltersChange={mockOnFiltersChange}
          pagination={{ page: 1, pageSize: 10, total: 15 }}
          onPaginationChange={mockOnPaginationChange}
        />
      )

      const nextButton = screen.getByTestId('next-page-button')
      expect(nextButton).not.toBeDisabled()

      await user.click(nextButton)

      // The function is called with the existing pagination object, just page changed
      expect(mockOnPaginationChange).toHaveBeenCalledWith(
        expect.objectContaining({
          page: 2,
          pageSize: 10,
        })
      )
    })

    it('GIVEN first page WHEN rendering THEN should disable previous button', () => {
      render(
        <TransactionHistoryTable
          transactions={[mockRechargeTransaction]}
          filters={mockFilters}
          onFiltersChange={mockOnFiltersChange}
          pagination={{ page: 1, pageSize: 10, total: 15 }}
          onPaginationChange={mockOnPaginationChange}
        />
      )

      const prevButton = screen.getByTestId('prev-page-button')
      expect(prevButton).toBeDisabled()
    })

    it('GIVEN last page WHEN rendering THEN should disable next button', () => {
      render(
        <TransactionHistoryTable
          transactions={[mockRechargeTransaction]}
          filters={mockFilters}
          onFiltersChange={mockOnFiltersChange}
          pagination={{ page: 2, pageSize: 10, total: 15 }}
          onPaginationChange={mockOnPaginationChange}
        />
      )

      const nextButton = screen.getByTestId('next-page-button')
      expect(nextButton).toBeDisabled()
    })
  })
})
