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
  const mockFilters: TransactionFilters = {
    transaction_type: undefined,
    start_time: undefined,
    end_time: undefined,
    client_app_id: undefined,
  }
  const mockClientApps = [{ id: 'app-123', name: 'Test App' }]

  describe('empty state', () => {
    it('GIVEN no transactions WHEN rendering THEN should show empty state', () => {
      render(
        <TransactionHistoryTable
          transactions={[]}
          filters={mockFilters}
        />
      )

      expect(screen.getByTestId('no-transactions')).toBeInTheDocument()
      expect(screen.getByText('No transactions found matching your criteria')).toBeInTheDocument()
    })

    it('GIVEN no transactions with active filters WHEN rendering THEN should show filter hint', () => {
      const activeFilters: TransactionFilters = {
        transactionType: 'recharge',
      }
      render(
        <TransactionHistoryTable
          transactions={[]}
          filters={activeFilters}
        />
      )

      expect(screen.getByText('Try adjusting your filters')).toBeInTheDocument()
    })
  })

  describe('rendering transactions', () => {
    it('GIVEN recharge transaction WHEN rendering THEN should display with green color and plus sign', () => {
      render(
        <TransactionHistoryTable
          transactions={[mockRechargeTransaction]}
          filters={mockFilters}
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
        />
      )

      expect(screen.queryByTestId('no-transactions')).not.toBeInTheDocument()
      expect(screen.queryByTestId('transaction-row-0')).not.toBeInTheDocument()
    })
  })
})
