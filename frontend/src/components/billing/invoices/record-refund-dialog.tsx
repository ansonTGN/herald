'use client'

import { useState } from 'react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Label } from '@/components/ui/label'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { recordRefundFormSchema } from '@/lib/schemas/credit-note-forms'
import { useCreateCreditNote } from '@/data/invoice-mutations'
import { getErrorMessage } from '@/lib/error-utils'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { displayPriceToCents, formatInvoiceAmount } from '@/lib/invoice-utils'
import { m } from '@/paraglide/messages'
import type { InvoiceDetailResponse } from '@/lib/api-generated'

interface RecordRefundDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  realmId: string
  invoice: InvoiceDetailResponse
}

export function RecordRefundDialog({
  open,
  onOpenChange,
  realmId,
  invoice,
}: RecordRefundDialogProps) {
  const mutation = useCreateCreditNote(realmId, invoice.id)
  const [inlineError, setInlineError] = useState<string | null>(null)

  const form = useAppForm({
    schema: recordRefundFormSchema,
    defaultValues: { amount: '', memo: '' },
    onSubmit: async ({ value }) => {
      setInlineError(null)

      const amountCents = displayPriceToCents(value.amount)
      if (amountCents > invoice.amountRemaining) {
        setInlineError(
          `${m['billing.credit_note_record_refund_amount_hint']()} (${formatInvoiceAmount(
            invoice.amountRemaining,
            invoice.currency
          )})`
        )
        return
      }

      try {
        await mutation.mutateAsync(value)
        onOpenChange(false)
      } catch (error) {
        setInlineError(getErrorMessage(error))
      }
    },
  })

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="border-t-4 border-t-cyan-500 sm:max-w-[425px]"
        data-testid="record-refund-dialog"
      >
        <DialogHeader>
          <DialogTitle>{m['billing.credit_note_record_refund_title']()}</DialogTitle>
          <DialogDescription>
            {m['billing.credit_note_record_refund_description']()}
          </DialogDescription>
        </DialogHeader>

        <div
          className="rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800"
          role="alert"
        >
          {m['billing.credit_note_record_refund_irreversible_warning']()}
        </div>

        <AppForm>
          <form
            onSubmit={(e) => {
              e.preventDefault()
              e.stopPropagation()
              form.handleSubmit()
            }}
            className="space-y-4"
          >
            <form.Field name="amount">
              {(field) => (
                <div className="space-y-2">
                  <Label htmlFor={field.name}>
                    {m['billing.credit_note_record_refund_amount_label']()}
                  </Label>
                  <Input
                    id={field.name}
                    name={field.name}
                    type="number"
                    step="0.01"
                    min="0.01"
                    placeholder={m['billing.credit_note_record_refund_amount_placeholder']()}
                    value={field.state.value ?? ''}
                    onChange={(e) => field.handleChange(e.target.value)}
                    onBlur={field.handleBlur}
                    data-testid="record-refund-amount-input"
                  />
                  <p className="text-xs text-muted-foreground">
                    {m['billing.credit_note_record_refund_amount_hint']()}:{' '}
                    {formatInvoiceAmount(invoice.amountRemaining, invoice.currency)}
                  </p>
                  {(field.state.meta.isTouched || form.state.isSubmitted) &&
                    field.state.meta.errors.length > 0 && (
                      <p className="text-sm text-destructive" role="alert">
                        {getFieldErrorMessage(field.state.meta)}
                      </p>
                    )}
                </div>
              )}
            </form.Field>

            <form.Field name="memo">
              {(field) => (
                <div className="space-y-2">
                  <Label htmlFor={field.name}>
                    {m['billing.credit_note_record_refund_reason_label']()}
                  </Label>
                  <Textarea
                    id={field.name}
                    name={field.name}
                    rows={3}
                    maxLength={500}
                    value={field.state.value ?? ''}
                    onChange={(e) => field.handleChange(e.target.value)}
                    onBlur={field.handleBlur}
                    data-testid="record-refund-reason-input"
                  />
                  {(field.state.meta.isTouched || form.state.isSubmitted) &&
                    field.state.meta.errors.length > 0 && (
                      <p className="text-sm text-destructive" role="alert">
                        {getFieldErrorMessage(field.state.meta)}
                      </p>
                    )}
                </div>
              )}
            </form.Field>

            {inlineError && (
              <div
                className="rounded-md border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive"
                role="alert"
                data-testid="record-refund-error-message"
              >
                {inlineError}
              </div>
            )}

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
                disabled={mutation.isPending}
                data-testid="record-refund-cancel-button"
              >
                {m['billing.credit_note_record_refund_cancel']()}
              </Button>
              <Button
                type="submit"
                disabled={mutation.isPending}
                loading={mutation.isPending}
                data-testid="record-refund-submit-button"
              >
                {mutation.isPending
                  ? m['billing.credit_note_record_refund_submitting']()
                  : m['billing.credit_note_record_refund_submit']()}
              </Button>
            </DialogFooter>
          </form>
        </AppForm>
      </DialogContent>
    </Dialog>
  )
}
