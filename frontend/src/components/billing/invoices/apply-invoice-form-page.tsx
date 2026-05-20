import { useEffect, useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { ArrowLeft } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { TextField, TextareaField } from '@/components/shared/form-fields'
import {
  applyInvoiceSchema,
  getApplyFormDefaults,
  type PrefilledInvoiceReference,
} from '@/lib/schemas/invoice-forms'
import { sellerConfigQueryOptions } from '@/data/invoice-query-options'
import { useApplyInvoice } from '@/data/invoice-mutations'

interface ApplyInvoiceFormPageProps {
  realmId: string
  prefilledReference?: PrefilledInvoiceReference
  returnTo?: string
}

export function ApplyInvoiceFormPage({
  realmId,
  prefilledReference,
  returnTo,
}: ApplyInvoiceFormPageProps) {
  const navigate = useNavigate()
  const { mutate: apply, isPending: isSubmitting } = useApplyInvoice(realmId)
  const { data: sellerConfig } = useQuery({
    ...sellerConfigQueryOptions(realmId),
  })
  const defaultValues = useMemo(
    () => getApplyFormDefaults(sellerConfig, prefilledReference),
    [sellerConfig, prefilledReference]
  )

  const form = useAppForm({
    schema: applyInvoiceSchema,
    defaultValues,
    onSubmit: async ({ value }) => {
      apply(value, {
        onSuccess: () => {
          navigate({
            to: '/$realmId/user/invoices',
            params: { realmId },
          })
        },
      })
    },
  })

  useEffect(() => {
    if (sellerConfig !== undefined) {
      form.reset(defaultValues)
    }
  }, [sellerConfig, defaultValues, form])

  const handleCancel = () => {
    if (returnTo === `/${realmId}/user/points`) {
      navigate({
        to: '/$realmId/user/points',
        params: { realmId },
      })
      return
    }

    if (returnTo === `/${realmId}/user/subscription-history`) {
      navigate({
        to: '/$realmId/user/subscription-history',
        params: { realmId },
      })
      return
    }

    navigate({
      to: '/$realmId/user/invoices',
      params: { realmId },
    })
  }

  return (
    <div className="max-w-2xl mx-auto py-6 px-6 space-y-6" data-testid="apply-form-page">
      <div className="flex items-center gap-4">
        <Button
          type="button"
          variant="ghost"
          size="icon"
          onClick={handleCancel}
          data-testid="apply-invoice-back-button"
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <h1 className="text-2xl font-bold" data-testid="apply-form-title">
          Apply for Invoice
        </h1>
      </div>

      <form
        onSubmit={(e) => {
          e.preventDefault()
          e.stopPropagation()
          form.handleSubmit()
        }}
        className="space-y-6"
      >
        <AppForm>
          <div className="space-y-6">
            <Card data-testid="apply-form-reference-section">
              <CardHeader>
                <CardTitle>Reference</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                {prefilledReference ? (
                  <div
                    className="rounded-md border border-muted bg-muted/40 px-3 py-2"
                    data-testid="apply-prefilled-reference"
                  >
                    <p className="text-sm font-medium">
                      {prefilledReference.type === 'paymentAttempt'
                        ? 'Points package purchase'
                        : 'Subscription'}
                    </p>
                    <p className="mt-1 text-xs text-muted-foreground">
                      Invoice reference is selected from your account history.
                    </p>
                  </div>
                ) : (
                  <>
                    <TextField
                      form={form}
                      name="paymentAttemptId"
                      label="Payment Attempt ID"
                      dataTestId="apply-payment-attempt-id-input"
                      placeholder="Enter payment attempt ID"
                    />
                    <TextField
                      form={form}
                      name="subscriptionId"
                      label="Subscription ID"
                      dataTestId="apply-subscription-id-input"
                      placeholder="Enter subscription ID"
                    />
                    <div className="rounded-md border border-muted bg-muted/40 px-3 py-2">
                      <p className="text-xs text-muted-foreground">
                        At least one of Payment Attempt ID or Subscription ID is required.
                      </p>
                    </div>
                  </>
                )}
              </CardContent>
            </Card>

            <Card data-testid="apply-form-billing-section">
              <CardHeader>
                <CardTitle>Billing Information</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <TextField
                  form={form}
                  name="billingName"
                  label="Billing Name"
                  dataTestId="apply-billing-name-input"
                  placeholder="Name on invoice"
                  required
                />
                <TextField
                  form={form}
                  name="billingEmail"
                  label="Billing Email"
                  dataTestId="apply-billing-email-input"
                  type="email"
                  placeholder="billing@example.com"
                />
                <TextField
                  form={form}
                  name="billingAddress"
                  label="Billing Address"
                  dataTestId="apply-billing-address-input"
                  placeholder="Billing address"
                  required
                />
                <TextField
                  form={form}
                  name="billingPhone"
                  label="Billing Phone"
                  dataTestId="apply-billing-phone-input"
                  placeholder="+1 234 567 8900"
                />
              </CardContent>
            </Card>

            <Card data-testid="apply-form-details-section">
              <CardHeader>
                <CardTitle>Invoice Details</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <TextField
                  form={form}
                  name="dueDate"
                  label="Due Date"
                  dataTestId="apply-due-date-input"
                  type="date"
                  required
                />
                <TextareaField
                  form={form}
                  name="notes"
                  label="Notes"
                  dataTestId="apply-notes-input"
                  placeholder="Additional notes for this invoice request"
                  rows={3}
                />
                <div className="rounded-md border border-muted bg-muted/40 px-3 py-2">
                  <p className="text-xs text-muted-foreground">
                    Seller info will be auto-filled from Realm config.
                  </p>
                </div>
              </CardContent>
            </Card>
          </div>
        </AppForm>

        <div className="flex items-center gap-3 pt-4 border-t">
          <Button
            type="button"
            variant="outline"
            onClick={handleCancel}
            data-testid="apply-invoice-cancel-button"
          >
            Cancel
          </Button>
          <Button type="submit" disabled={isSubmitting} data-testid="apply-invoice-submit-button">
            {isSubmitting ? 'Submitting...' : 'Submit'}
          </Button>
        </div>
      </form>
    </div>
  )
}
