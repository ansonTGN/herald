import { useState } from 'react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { Label } from '@/components/ui/label'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import type { AsyncPointsStrategy } from '@/lib/schemas/stripe-config'

interface AsyncPaymentStrategySelectorProps {
  value: AsyncPointsStrategy
  onChange: (value: AsyncPointsStrategy) => void
  disabled?: boolean
}

// TODO: i18n — add paraglide message keys for all user-facing strings below
const CONSERVATIVE_LABEL = '保守模式'
const CONSERVATIVE_DESCRIPTION = '异步支付确认到账后才发放积分。银行转账用户需等待 2-14 个工作日。'
const EAGER_LABEL = '激进模式'
const EAGER_DESCRIPTION = '异步支付发起后立即发放积分。如果支付失败，系统将自动回收积分。'
const CARD_TITLE = '异步支付积分发放策略'
const DIALOG_TITLE = '确认切换到激进模式'
const DIALOG_DESCRIPTION =
  '异步支付发起后立即发放积分。如果支付失败，系统将自动回收积分。确定要切换吗？'
const DIALOG_CONFIRM = '确认'
const DIALOG_CANCEL = '取消'

export function AsyncPaymentStrategySelector({
  value,
  onChange,
  disabled,
}: AsyncPaymentStrategySelectorProps) {
  const [pendingValue, setPendingValue] = useState<AsyncPointsStrategy | null>(null)

  const displayValue = pendingValue ?? value
  const dialogOpen = pendingValue === 'eager'

  function handleValueChange(newValue: string) {
    const strategy = newValue as AsyncPointsStrategy
    if (strategy === 'eager' && value !== 'eager') {
      setPendingValue('eager')
    } else {
      onChange(strategy)
    }
  }

  function handleConfirm() {
    onChange('eager')
    setPendingValue(null)
  }

  function handleCancel() {
    setPendingValue(null)
  }

  return (
    <Card data-testid="async-strategy-card">
      <CardHeader>
        <CardTitle>{CARD_TITLE}</CardTitle>
      </CardHeader>
      <CardContent>
        <RadioGroup
          value={displayValue}
          onValueChange={handleValueChange}
          disabled={disabled}
          data-testid="async-strategy-radio-group"
          className="gap-4"
        >
          <div className="flex items-start space-x-3">
            <RadioGroupItem
              value="conservative"
              id="strategy-conservative"
              data-testid="async-strategy-conservative-radio"
              className="mt-0.5"
            />
            <div className="space-y-1 leading-none">
              <Label htmlFor="strategy-conservative" className="font-medium">
                {CONSERVATIVE_LABEL}
              </Label>
              <CardDescription>{CONSERVATIVE_DESCRIPTION}</CardDescription>
            </div>
          </div>

          <div className="flex items-start space-x-3">
            <RadioGroupItem
              value="eager"
              id="strategy-eager"
              data-testid="async-strategy-eager-radio"
              className="mt-0.5"
            />
            <div className="space-y-1 leading-none">
              <Label htmlFor="strategy-eager" className="font-medium">
                {EAGER_LABEL}
              </Label>
              <CardDescription>{EAGER_DESCRIPTION}</CardDescription>
            </div>
          </div>
        </RadioGroup>

        <AlertDialog open={dialogOpen} onOpenChange={(open) => !open && handleCancel()}>
          <AlertDialogContent data-testid="async-strategy-confirm-dialog">
            <AlertDialogHeader>
              <AlertDialogTitle>{DIALOG_TITLE}</AlertDialogTitle>
              <AlertDialogDescription>{DIALOG_DESCRIPTION}</AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel data-testid="async-strategy-cancel-button">
                {DIALOG_CANCEL}
              </AlertDialogCancel>
              <AlertDialogAction
                data-testid="async-strategy-confirm-button"
                onClick={handleConfirm}
              >
                {DIALOG_CONFIRM}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </CardContent>
    </Card>
  )
}
