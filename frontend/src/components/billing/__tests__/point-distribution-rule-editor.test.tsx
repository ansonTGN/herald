import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PointDistributionRuleEditor } from '../point-distribution-rule-editor'
import { pointDistributionRulesSchema } from '@/lib/schemas/billing-forms'

const buckets = [
  {
    id: 'bucket-a',
    name: 'General',
    bucketKey: 'general',
    displayOrder: 0,
    enabled: true,
    coveredClientAppCount: 1,
    ruleReferenceCount: 0,
  },
]

describe('PointDistributionRuleEditor', () => {
  it('removes unsaved rules but explicitly disables persisted rules', async () => {
    const user = userEvent.setup()
    const onChange = vi.fn()
    const { rerender } = render(
      <PointDistributionRuleEditor
        value={[
          {
            id: 'rule-1',
            bucketId: 'bucket-a',
            triggerSources: ['topup'],
            grantMode: 'fixed',
            pointsAmount: 100,
            validityDays: 0,
            quotaWindows: null,
            enabled: true,
          },
        ]}
        onChange={onChange}
        buckets={buckets}
        triggers={[{ value: 'topup', label: 'Top-up' }]}
        allowQuota={false}
      />
    )

    await user.click(screen.getByTestId('point-rule-remove-rule-1'))
    expect(onChange).toHaveBeenLastCalledWith([
      expect.objectContaining({ id: 'rule-1', enabled: false }),
    ])

    rerender(
      <PointDistributionRuleEditor
        value={[
          {
            bucketId: 'bucket-a',
            triggerSources: ['topup'],
            grantMode: 'fixed',
            pointsAmount: 100,
            validityDays: 0,
            quotaWindows: null,
            enabled: true,
          },
        ]}
        onChange={onChange}
        buckets={buckets}
        triggers={[{ value: 'topup', label: 'Top-up' }]}
        allowQuota={false}
      />
    )
    await user.click(screen.getByTestId('point-rule-remove-new-0'))
    expect(onChange).toHaveBeenLastCalledWith([])
  })

  it('rejects fixed and quota rules with missing policy data', () => {
    expect(
      pointDistributionRulesSchema.safeParse([
        {
          bucketId: 'bucket-a',
          triggerSources: ['topup'],
          grantMode: 'fixed',
          pointsAmount: 0,
        },
      ]).success
    ).toBe(false)
    expect(
      pointDistributionRulesSchema.safeParse([
        {
          bucketId: 'bucket-a',
          triggerSources: ['subscription_initial'],
          grantMode: 'quota',
          quotaWindows: [],
        },
      ]).success
    ).toBe(false)
  })
})
