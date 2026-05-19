import { useEffect, useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Label } from '@/components/ui/label'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { BaseFormDialog } from '@/components/shared/form-dialog'
import { TextField } from '@/components/shared/form-fields'
import { getFieldErrorMessage } from '@/lib/form-utils'
import {
  invoiceSellerConfigSchema,
  type InvoiceSellerConfigFormData,
} from '@/lib/schemas/invoice-forms'
import { sellerConfigQueryOptions } from '@/data/invoice-query-options'
import { useUpsertSellerConfig } from '@/data/invoice-mutations'

interface InvoiceSellerConfigFormProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  realmId: string
}

export function InvoiceSellerConfigForm({
  open,
  onOpenChange,
  realmId,
}: InvoiceSellerConfigFormProps) {
  const { data: sellerConfig } = useQuery(sellerConfigQueryOptions(realmId))
  const { mutate: upsertConfig, isPending: isSubmitting } = useUpsertSellerConfig(realmId)

  const defaultValues = useMemo<InvoiceSellerConfigFormData>(
    () => ({
      sellerName: sellerConfig?.sellerName ?? '',
      sellerAddress: sellerConfig?.sellerAddress ?? '',
      sellerEmail: sellerConfig?.sellerEmail ?? null,
      sellerPhone: sellerConfig?.sellerPhone ?? null,
      sellerTaxId: sellerConfig?.sellerTaxId ?? '',
      defaultPaymentTerms: sellerConfig?.defaultPaymentTerms ?? null,
    }),
    [sellerConfig]
  )

  const form = useAppForm({
    schema: invoiceSellerConfigSchema,
    defaultValues,
    onSubmit: async ({ value }) => {
      upsertConfig(value, {
        onSuccess: () => {
          onOpenChange(false)
        },
      })
    },
  })

  useEffect(() => {
    if (open) {
      form.reset(defaultValues)
    }
  }, [defaultValues, form, open])

  return (
    <BaseFormDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Seller Configuration"
      description="Configure the seller information for invoices in this realm"
      data-testid="seller-config-form-dialog"
      footer={
        <>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
            data-testid="seller-config-cancel-button"
          >
            Cancel
          </Button>
          <Button
            type="submit"
            form="seller-config-form"
            disabled={isSubmitting}
            data-testid="seller-config-save-button"
          >
            {isSubmitting ? 'Saving...' : 'Save'}
          </Button>
        </>
      }
    >
      <form
        id="seller-config-form"
        onSubmit={(e) => {
          e.preventDefault()
          e.stopPropagation()
          form.handleSubmit()
        }}
      >
        <AppForm>
          <div className="space-y-4">
            <TextField
              form={form}
              name="sellerName"
              label="Seller Name"
              dataTestId="seller-config-name-input"
              placeholder="Company name"
              required
            />
            <TextField
              form={form}
              name="sellerAddress"
              label="Address"
              dataTestId="seller-config-address-input"
              placeholder="Business address"
              required
            />
            <TextField
              form={form}
              name="sellerEmail"
              label="Email"
              dataTestId="seller-config-email-input"
              type="email"
              placeholder="billing@example.com"
            />
            <TextField
              form={form}
              name="sellerPhone"
              label="Phone"
              dataTestId="seller-config-phone-input"
              placeholder="+1 234 567 8900"
            />
            <TextField
              form={form}
              name="sellerTaxId"
              label="Tax ID"
              dataTestId="seller-config-tax-id-input"
              placeholder="Tax identification number"
              required
            />
            <form.Field
              name="defaultPaymentTerms"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor={field.name}>Default Payment Terms</Label>
                  <Select
                    data-testid="seller-config-payment-terms-input"
                    value={field.state.value ?? ''}
                    onValueChange={(value) => field.handleChange(value)}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select payment terms" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="Due on Receipt">Due on Receipt</SelectItem>
                      <SelectItem value="Net 7">Net 7</SelectItem>
                      <SelectItem value="Net 15">Net 15</SelectItem>
                      <SelectItem value="Net 30">Net 30</SelectItem>
                      <SelectItem value="Net 60">Net 60</SelectItem>
                      <SelectItem value="Net 90">Net 90</SelectItem>
                    </SelectContent>
                  </Select>
                  {(field.state.meta.isTouched || form.state.isSubmitted) &&
                    field.state.meta.errors.length > 0 && (
                      <p className="text-sm text-destructive" role="alert">
                        {getFieldErrorMessage(field.state.meta)}
                      </p>
                    )}
                </div>
              )}
            />
          </div>
        </AppForm>
      </form>
    </BaseFormDialog>
  )
}
