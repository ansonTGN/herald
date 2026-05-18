import { useMemo } from 'react'
import type { TransactionFilters } from '@/lib/schemas/points-forms'

/**
 * Memoized check for whether any transaction filters are active.
 * Shared between TransactionFilters and TransactionHistoryTable.
 */
export function useActiveFilters(filters: TransactionFilters): boolean {
  return useMemo(
    () =>
      !!filters.transactionType ||
      !!filters.startTime ||
      !!filters.endTime ||
      !!filters.clientAppId,
    [filters]
  )
}
