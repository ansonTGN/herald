import { useState } from 'react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { Label } from '@/components/ui/label'
import { ConfirmDialog } from '@/components/shared/confirm-delete-dialog'
import { m } from '@/paraglide/messages'
import type { AsyncPointsStrategy } from '@/lib/schemas/stripe-config'

interface AsyncPaymentStrategySelectorProps {
  value: AsyncPointsStrategy
  onChange: (value: AsyncPointsStrategy) => void
  disabled?: boolean
}

const STRATEGIES: {
  value: AsyncPointsStrategy
  label: string
  description: string
}[] = [
  {
    value: 'conservative',
    label: m['billing.async_strategy_conservative_label'](),
    description: m['billing.async_strategy_conservative_desc'](),
  },
  {
    value: 'eager',
    label: m['billing.async_strategy_eager_label'](),
    description: m['billing.async_strategy_eager_desc'](),
  },
]

export function AsyncPaymentStrategySelector({
  value,
  onChange,
  disabled,
}: AsyncPaymentStrategySelectorProps) {
  const [confirmOpen, setConfirmOpen] = useState(false)

  function handleValueChange(newValue: string) {
    if (newValue === 'eager' && value !== 'eager') {
      setConfirmOpen(true)
    } else {
      onChange(newValue as AsyncPointsStrategy)
    }
  }

  function handleConfirm() {
    onChange('eager')
    setConfirmOpen(false)
  }

  return (
    <Card data-testid="async-strategy-card">
      <CardHeader>
        <CardTitle>{m['billing.async_strategy_title']()}</CardTitle>
      </CardHeader>
      <CardContent>
        <RadioGroup
          value={value}
          onValueChange={handleValueChange}
          disabled={disabled}
          data-testid="async-strategy-radio-group"
          className="gap-4"
        >
          {STRATEGIES.map((s) => (
            <div key={s.value} className="flex items-start space-x-3">
              <RadioGroupItem
                value={s.value}
                id={`strategy-${s.value}`}
                data-testid={`async-strategy-${s.value}-radio`}
                className="mt-0.5"
              />
              <div className="space-y-1 leading-none">
                <Label htmlFor={`strategy-${s.value}`} className="font-medium">
                  {s.label}
                </Label>
                <CardDescription>{s.description}</CardDescription>
              </div>
            </div>
          ))}
        </RadioGroup>

        <ConfirmDialog
          open={confirmOpen}
          onOpenChange={setConfirmOpen}
          title={m['billing.async_strategy_confirm_title']()}
          description={m['billing.async_strategy_confirm_desc']()}
          onConfirm={handleConfirm}
          confirmLabel={m['billing.async_strategy_confirm']()}
          cancelLabel={m['billing.async_strategy_cancel']()}
          confirmClassName=""
          contentTestId="async-strategy-confirm-dialog"
          cancelTestId="async-strategy-cancel-button"
          confirmTestId="async-strategy-confirm-button"
        />
      </CardContent>
    </Card>
  )
}
