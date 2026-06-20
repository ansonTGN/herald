import { describe, expect, it } from 'vitest'
import { pointsTransactionsQueryOptions } from '@/data/query-options'

describe('points query options — cache key isolation', () => {
  it('should create unique cache keys for different transaction types', () => {
    const options1 = pointsTransactionsQueryOptions('realm-1', { transactionType: 'recharge' })
    const options2 = pointsTransactionsQueryOptions('realm-1', { transactionType: 'consume' })

    expect(options1.queryKey).not.toEqual(options2.queryKey)
  })
})
