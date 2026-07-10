import { useEffect, useMemo, useCallback } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useStore } from '@tanstack/react-form'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { getFieldErrorMessage } from '@/lib/form-utils'
import type { InvoiceDetailResponse } from '@/lib/api-generated'
import {
  invoiceCreateFormSchema,
  invoiceEditFormSchema,
  getInvoiceFormDefaults,
  type InvoiceCreateFormData,
  type InvoiceEditFormData,
  type InvoiceLineItemFormData,
} from '@/lib/schemas/invoice-forms'
import { useCreateInvoice, useUpdateInvoice } from '@/data/invoice-mutations'
import { sellerConfigQueryOptions } from '@/data/invoice-query-options'
import {
  calculateLineSubtotal,
  calculateTotals,
  centsToDisplayPrice,
  formatInvoiceAmount,
  isExternalInvoice,
} from '@/lib/invoice-utils'
import { Plus, Trash2, ArrowLeft } from 'lucide-react'
import { m } from '@/paraglide/messages'
import { realmPath, useResolvedRealmContext } from '@/lib/realm-routing'

interface InvoiceFormPageProps {
  mode: 'create' | 'edit'
  realmId: string
  invoice?: InvoiceDetailResponse
}

function getFeeModeOptions() {
  return [
    { value: 'none', label: m['billing.invoice_fee_none']() },
    { value: 'fixed', label: m['billing.invoice_fee_fixed']() },
    { value: 'percent', label: m['billing.invoice_fee_percent']() },
  ]
}

function getShippingModeOptions() {
  return [
    { value: 'none', label: m['billing.invoice_fee_none']() },
    { value: 'fixed', label: m['billing.invoice_fee_fixed']() },
  ]
}

export function InvoiceFormPage({ mode, realmId, invoice }: InvoiceFormPageProps) {
  const isEditing = mode === 'edit'
  const navigate = useNavigate()
  const realmContext = useResolvedRealmContext()
  const invoicesPath = realmContext.isCustomDomain
    ? realmPath(realmContext, '/manage/billing/invoices')
    : '/$realmId/manage/billing/invoices'
  const invoicesParams = useMemo(
    () => (realmContext.isCustomDomain ? undefined : { realmId }),
    [realmContext.isCustomDomain, realmId]
  )

  const { data: sellerConfig } = useQuery({
    ...sellerConfigQueryOptions(realmId),
    enabled: !isEditing,
  })

  const createMutation = useCreateInvoice(realmId)
  const updateMutation = useUpdateInvoice(realmId, invoice?.id ?? '')

  const defaultValues = useMemo(() => {
    if (isEditing && invoice) {
      return getEditDefaults(invoice)
    }
    return getInvoiceFormDefaults(sellerConfig)
  }, [isEditing, invoice, sellerConfig])

  const schema = isEditing ? invoiceEditFormSchema : invoiceCreateFormSchema

  const form = useAppForm({
    schema,
    defaultValues,
    onSubmit: async ({ value }) => {
      // The mutations' `onError` surface the failure toast and log the error.
      // Catch the rejection so it doesn't propagate as an unhandled rejection
      // (per vitest config, rejections are expected to be handled in
      // components). On failure, stay on the form — navigating away would
      // unmount it and discard the operator's unsaved input.
      try {
        if (isEditing) {
          await updateMutation.mutateAsync(value as InvoiceEditFormData)
        } else {
          await createMutation.mutateAsync(value as InvoiceCreateFormData)
        }
      } catch {
        return
      }
      navigate({
        to: invoicesPath,
        params: invoicesParams,
      })
    },
  })

  const isSubmitting = createMutation.isPending || updateMutation.isPending

  // In create mode, reset form when seller config loads to populate seller fields
  useEffect(() => {
    if (!isEditing && sellerConfig !== undefined) {
      form.reset(defaultValues)
    }
  }, [isEditing, sellerConfig, defaultValues, form])

  const addLineItem = useCallback(() => {
    const currentItems = form.getFieldValue('lineItems') as InvoiceLineItemFormData[]
    form.setFieldValue('lineItems', [
      ...currentItems,
      { name: '', description: null, quantity: '1', unitPrice: '0.00' },
    ])
  }, [form])

  const removeLineItem = useCallback(
    (index: number) => {
      const currentItems = form.getFieldValue('lineItems') as InvoiceLineItemFormData[]
      if (currentItems.length <= 1) return
      form.setFieldValue(
        'lineItems',
        currentItems.filter((_, i) => i !== index)
      )
    },
    [form]
  )

  const handleCancel = useCallback(() => {
    navigate({
      to: invoicesPath,
      params: invoicesParams,
    })
  }, [invoicesParams, invoicesPath, navigate])

  // Guard: redirect external invoices back to list — they cannot be edited
  useEffect(() => {
    if (isEditing && invoice && isExternalInvoice(invoice.provider)) {
      navigate({ to: invoicesPath, params: invoicesParams })
    }
  }, [invoicesParams, invoicesPath, isEditing, invoice, navigate])

  return (
    <div
      className="container max-w-4xl mx-auto py-6 px-6 space-y-6"
      data-testid="invoice-form-page"
    >
      <div className="flex items-center gap-4">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={handleCancel}
          data-testid="invoice-form-back-button"
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <div>
          <h1 className="text-2xl font-bold" data-testid="invoice-form-title">
            {isEditing ? m['billing.invoice_edit_title']() : m['billing.invoice_create_title']()}
          </h1>
          <p className="text-muted-foreground text-sm">
            {isEditing
              ? m['billing.invoice_edit_description']()
              : m['billing.invoice_create_description']()}
          </p>
        </div>
      </div>

      <form
        onSubmit={(e) => {
          e.preventDefault()
          e.stopPropagation()
          form.handleSubmit()
        }}
        className="max-w-4xl space-y-6"
      >
        <AppForm>
          <div className="space-y-6">
            {!isEditing && <AccountSelectorField form={form} />}

            <div className="grid grid-cols-1 gap-6 md:grid-cols-2">
              <div className="space-y-3">
                <h3 className="text-sm font-semibold">{m['billing.invoice_buyer_info']()}</h3>
                <InvoiceTextField
                  form={form}
                  name="billingName"
                  label={m['billing.invoice_name']()}
                  placeholder={m['billing.invoice_placeholder_name']()}
                  required
                  dataTestId="invoice-billing-name"
                />
                <InvoiceTextField
                  form={form}
                  name="billingEmail"
                  label={m['billing.invoice_email']()}
                  placeholder={m['billing.invoice_placeholder_email']()}
                  dataTestId="invoice-billing-email"
                />
                <InvoiceTextField
                  form={form}
                  name="billingAddress"
                  label={m['billing.invoice_address']()}
                  placeholder={m['billing.invoice_placeholder_address']()}
                  required
                  dataTestId="invoice-billing-address"
                />
                <InvoiceTextField
                  form={form}
                  name="billingPhone"
                  label={m['billing.invoice_phone']()}
                  placeholder={m['billing.invoice_placeholder_phone']()}
                  dataTestId="invoice-billing-phone"
                />
                <InvoiceTextField
                  form={form}
                  name="billingTaxId"
                  label={m['billing.invoice_seller_tax_id_label']()}
                  placeholder={m['billing.invoice_placeholder_tax_id']()}
                  required
                  dataTestId="invoice-billing-tax-id"
                />
              </div>

              <div className="space-y-3">
                <h3 className="text-sm font-semibold">{m['billing.invoice_seller_info']()}</h3>
                <InvoiceTextField
                  form={form}
                  name="sellerName"
                  label={m['billing.invoice_name']()}
                  placeholder={m['billing.invoice_placeholder_seller_name']()}
                  required
                  dataTestId="invoice-seller-name"
                />
                <InvoiceTextField
                  form={form}
                  name="sellerEmail"
                  label={m['billing.invoice_email']()}
                  placeholder={m['billing.invoice_placeholder_seller_email']()}
                  dataTestId="invoice-seller-email"
                />
                <InvoiceTextField
                  form={form}
                  name="sellerAddress"
                  label={m['billing.invoice_address']()}
                  placeholder={m['billing.invoice_placeholder_seller_address']()}
                  required
                  dataTestId="invoice-seller-address"
                />
                <InvoiceTextField
                  form={form}
                  name="sellerPhone"
                  label={m['billing.invoice_phone']()}
                  placeholder={m['billing.invoice_placeholder_phone']()}
                  dataTestId="invoice-seller-phone"
                />
                <InvoiceTextField
                  form={form}
                  name="sellerTaxId"
                  label={m['billing.invoice_seller_tax_id_label']()}
                  placeholder={m['billing.invoice_placeholder_tax_id']()}
                  required
                  dataTestId="invoice-seller-tax-id"
                />
              </div>
            </div>

            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <h3 className="text-sm font-semibold">{m['billing.invoice_line_items']()}</h3>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={addLineItem}
                  data-testid="invoice-add-line-item"
                >
                  <Plus className="mr-1 h-3 w-3" />
                  {m['billing.invoice_add_line_item']()}
                </Button>
              </div>

              <div className="rounded-md border">
                <div className="grid grid-cols-[1fr_100px_120px_120px_40px] gap-2 bg-muted/50 px-3 py-2 text-xs font-medium">
                  <span>{m['billing.invoice_line_item_name_required']()}</span>
                  <span>{m['billing.invoice_line_item_quantity_required']()}</span>
                  <span>{m['billing.invoice_line_item_unit_price_required']()}</span>
                  <span>{m['billing.invoice_line_item_subtotal']()}</span>
                  <span />
                </div>

                <form.Field name="lineItems" mode="array">
                  {(field) => (
                    <div className="divide-y">
                      {field.state.value.map((_, index) => (
                        <LineItemRow
                          key={index}
                          form={form}
                          index={index}
                          onRemove={() => removeLineItem(index)}
                          canRemove={field.state.value.length > 1}
                        />
                      ))}
                    </div>
                  )}
                </form.Field>
              </div>
            </div>

            <div className="space-y-3">
              <h3 className="text-sm font-semibold">{m['billing.invoice_fees']()}</h3>
              <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
                <FeeInput
                  form={form}
                  modeName="discountMode"
                  valueName="discountValue"
                  label={m['billing.invoice_discount_label']()}
                  dataTestIdPrefix="invoice-discount"
                />
                <FeeInput
                  form={form}
                  modeName="taxMode"
                  valueName="taxValue"
                  label={m['billing.invoice_tax_label']()}
                  dataTestIdPrefix="invoice-tax"
                />
                <FeeInput
                  form={form}
                  modeName="shippingMode"
                  valueName="shippingValue"
                  label={m['billing.invoice_shipping_label']()}
                  dataTestIdPrefix="invoice-shipping"
                  modeOptions={getShippingModeOptions()}
                />
              </div>
            </div>

            <TotalsPreview
              form={form}
              defaultCurrency={isEditing ? (invoice?.currency ?? 'CNY') : 'CNY'}
            />

            <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
              {!isEditing && <CurrencySelect form={form} />}
              <InvoiceTextField
                form={form}
                name="dueDate"
                label={m['billing.invoice_due_date_label']()}
                type="date"
                required
                dataTestId="invoice-due-date"
              />
              <InvoiceTextField
                form={form}
                name="paymentTerms"
                label={m['billing.invoice_payment_terms']()}
                placeholder={m['billing.invoice_payment_terms_placeholder']()}
                dataTestId="invoice-payment-terms"
              />
            </div>

            {!isEditing && (
              <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                <InvoiceTextField
                  form={form}
                  name="subscriptionId"
                  label={m['billing.invoice_subscription_id']()}
                  placeholder={m['common.optional']()}
                  dataTestId="invoice-subscription-id"
                />
                <InvoiceTextField
                  form={form}
                  name="paymentAttemptId"
                  label={m['billing.invoice_payment_attempt_id']()}
                  placeholder={m['common.optional']()}
                  dataTestId="invoice-payment-attempt-id"
                />
              </div>
            )}

            <form.Field name="notes">
              {(field) => (
                <div className="space-y-2">
                  <Label htmlFor={field.name}>{m['billing.invoice_notes_label']()}</Label>
                  <textarea
                    id={field.name}
                    data-testid="invoice-notes"
                    value={field.state.value ?? ''}
                    onBlur={field.handleBlur}
                    onChange={(e) => field.handleChange(e.target.value || null)}
                    placeholder={m['billing.invoice_notes_placeholder']()}
                    rows={3}
                    className="flex w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm placeholder:text-placeholder focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:border-transparent disabled:bg-muted disabled:text-muted-foreground"
                  />
                  {(field.state.meta.isTouched || form.state.isSubmitted) &&
                    field.state.meta.errors.length > 0 && (
                      <p className="text-sm text-destructive">
                        {getFieldErrorMessage(field.state.meta)}
                      </p>
                    )}
                </div>
              )}
            </form.Field>
          </div>
        </AppForm>

        <div className="flex items-center gap-3 pt-4 border-t">
          <Button
            type="button"
            variant="outline"
            onClick={handleCancel}
            data-testid="invoice-form-cancel-button"
          >
            {m['common.cancel']()}
          </Button>
          <Button type="submit" disabled={isSubmitting} data-testid="invoice-form-submit-button">
            {isSubmitting ? m['shared.saving']() : m['billing.invoice_save_draft']()}
          </Button>
        </div>
      </form>
    </div>
  )
}

function getEditDefaults(invoice: InvoiceDetailResponse): InvoiceEditFormData {
  return {
    billingName: invoice.billingName ?? '',
    billingEmail: invoice.billingEmail ?? null,
    billingAddress: invoice.billingAddress ?? '',
    billingPhone: invoice.billingPhone ?? null,
    billingTaxId: invoice.billingTaxId ?? '',
    sellerName: invoice.sellerName ?? '',
    sellerEmail: invoice.sellerEmail ?? null,
    sellerAddress: invoice.sellerAddress ?? '',
    sellerPhone: invoice.sellerPhone ?? null,
    sellerTaxId: invoice.sellerTaxId ?? '',
    lineItems: invoice.lineItems.map((item) => ({
      name: item.name,
      description: item.description ?? null,
      quantity: item.quantity,
      unitPrice: centsToDisplayPrice(item.unitPrice),
    })),
    discountMode: (invoice.discountMode as 'fixed' | 'percent' | null) ?? null,
    discountValue: invoice.discountValue ?? null,
    taxMode: (invoice.taxMode as 'fixed' | 'percent' | null) ?? null,
    taxValue: invoice.taxValue ?? null,
    shippingMode: (invoice.shippingMode as 'fixed' | null) ?? null,
    shippingValue: invoice.shippingValue ?? null,
    dueDate: invoice.dueDate ?? '',
    paymentTerms: invoice.paymentTerms ?? null,
    notes: invoice.notes ?? null,
  }
}

/* eslint-disable @typescript-eslint/no-explicit-any -- sub-components accept the repository's typed form instance */

function AccountSelectorField({ form }: { form: any }) {
  return (
    <InvoiceTextField
      form={form}
      name="accountId"
      label={m['billing.invoice_label_account']()}
      placeholder={m['billing.invoice_placeholder_account']()}
      required
      dataTestId="invoice-account-id"
    />
  )
}

function InvoiceTextField({
  form,
  name,
  label,
  placeholder,
  type = 'text',
  required = false,
  dataTestId,
}: {
  form: any
  name: string
  label: string
  placeholder?: string
  type?: React.InputHTMLAttributes<HTMLInputElement>['type']
  required?: boolean
  dataTestId: string
}) {
  return (
    <form.Field name={name}>
      {(field: any) => (
        <div className="space-y-1">
          <Label htmlFor={field.name} className="text-xs">
            {label}
            {required ? <span className="text-destructive"> *</span> : null}
          </Label>
          <Input
            id={field.name}
            data-testid={dataTestId}
            type={type}
            value={field.state.value ?? ''}
            onBlur={field.handleBlur}
            onChange={(e) => {
              const val = e.target.value
              field.handleChange(val === '' && !required ? null : val)
            }}
            placeholder={placeholder}
            className="h-8 text-sm"
          />
          {(field.state.meta.isTouched || form.state.isSubmitted) &&
            field.state.meta.errors.length > 0 && (
              <p className="text-xs text-destructive">{getFieldErrorMessage(field.state.meta)}</p>
            )}
        </div>
      )}
    </form.Field>
  )
}

function LineItemRow({
  form,
  index,
  onRemove,
  canRemove,
}: {
  form: any
  index: number
  onRemove: () => void
  canRemove: boolean
}) {
  return (
    <form.Field name={`lineItems[${index}]`}>
      {(field: any) => {
        const item = field.state.value ?? {}
        const quantity = (item.quantity as string) ?? '0'
        const unitPrice = (item.unitPrice as string) ?? '0'
        const subtotal = calculateLineSubtotal(quantity, unitPrice)

        return (
          <div className="grid grid-cols-[1fr_100px_120px_120px_40px] gap-2 px-3 py-2 items-center">
            <form.Field name={`lineItems[${index}].name`}>
              {(nameField: any) => (
                <div>
                  <Input
                    data-testid={`invoice-line-item-name-${index}`}
                    value={nameField.state.value ?? ''}
                    onBlur={nameField.handleBlur}
                    onChange={(e) => nameField.handleChange(e.target.value)}
                    placeholder={m['billing.invoice_line_item_name_placeholder']()}
                    className="h-8 text-sm"
                  />
                  {(nameField.state.meta.isTouched || form.state.isSubmitted) &&
                    nameField.state.meta.errors.length > 0 && (
                      <p className="text-xs text-destructive mt-0.5">
                        {getFieldErrorMessage(nameField.state.meta)}
                      </p>
                    )}
                </div>
              )}
            </form.Field>

            <form.Field name={`lineItems[${index}].quantity`}>
              {(qtyField: any) => (
                <div>
                  <Input
                    data-testid={`invoice-line-item-quantity-${index}`}
                    value={qtyField.state.value ?? ''}
                    onBlur={qtyField.handleBlur}
                    onChange={(e) => qtyField.handleChange(e.target.value)}
                    placeholder={m['billing.invoice_line_item_quantity_placeholder']()}
                    className="h-8 text-sm"
                  />
                  {(qtyField.state.meta.isTouched || form.state.isSubmitted) &&
                    qtyField.state.meta.errors.length > 0 && (
                      <p className="text-xs text-destructive mt-0.5">
                        {getFieldErrorMessage(qtyField.state.meta)}
                      </p>
                    )}
                </div>
              )}
            </form.Field>

            <form.Field name={`lineItems[${index}].unitPrice`}>
              {(priceField: any) => (
                <div>
                  <Input
                    data-testid={`invoice-line-item-unit-price-${index}`}
                    type="text"
                    inputMode="decimal"
                    value={priceField.state.value ?? ''}
                    onBlur={priceField.handleBlur}
                    onChange={(e) => priceField.handleChange(e.target.value)}
                    placeholder={m['billing.invoice_line_item_unit_price_placeholder']()}
                    className="h-8 text-sm"
                  />
                  {(priceField.state.meta.isTouched || form.state.isSubmitted) &&
                    priceField.state.meta.errors.length > 0 && (
                      <p className="text-xs text-destructive mt-0.5">
                        {getFieldErrorMessage(priceField.state.meta)}
                      </p>
                    )}
                </div>
              )}
            </form.Field>

            <span
              className="text-sm font-mono text-right"
              data-testid={`invoice-line-item-subtotal-${index}`}
            >
              {centsToDisplayPrice(subtotal)}
            </span>

            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-6 w-6 p-0"
              onClick={onRemove}
              disabled={!canRemove}
              data-testid={`invoice-line-item-remove-${index}`}
            >
              <Trash2 className="h-3 w-3" />
            </Button>
          </div>
        )
      }}
    </form.Field>
  )
}

function FeeInput({
  form,
  modeName,
  valueName,
  label,
  dataTestIdPrefix,
  modeOptions = getFeeModeOptions(),
}: {
  form: any
  modeName: string
  valueName: string
  label: string
  dataTestIdPrefix: string
  modeOptions?: { value: string; label: string }[]
}) {
  const currentMode = useStore(form.store, (state: any) => state.values[modeName])

  return (
    <div className="space-y-1">
      <Label className="text-xs">{label}</Label>
      <div className="flex gap-2">
        <form.Field name={modeName}>
          {(modeField: any) => (
            <Select
              data-testid={`${dataTestIdPrefix}-mode`}
              value={modeField.state.value ?? 'none'}
              onValueChange={(value) => {
                modeField.handleChange(value === 'none' ? null : value)
              }}
            >
              <SelectTrigger
                className="h-8 w-[110px] text-sm"
                data-testid={`${dataTestIdPrefix}-mode-trigger`}
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {modeOptions.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        </form.Field>

        <form.Field name={valueName}>
          {(valueField: any) => (
            <Input
              data-testid={`${dataTestIdPrefix}-value`}
              type="number"
              value={valueField.state.value ?? ''}
              onBlur={valueField.handleBlur}
              onChange={(e) => valueField.handleChange(e.target.value || null)}
              placeholder={
                currentMode === 'percent' ? '%' : m['billing.invoice_fee_amount_placeholder']()
              }
              disabled={!currentMode}
              min={0}
              step="0.01"
              className="h-8 text-sm"
            />
          )}
        </form.Field>
      </div>
    </div>
  )
}

function TotalsPreview({ form, defaultCurrency }: { form: any; defaultCurrency: string }) {
  const formState = useStore(form.store, (state: any) => state.values)

  const currency = formState.currency ?? defaultCurrency
  const lineItems = formState.lineItems
  const { discountMode, discountValue, taxMode, taxValue, shippingMode, shippingValue } = formState

  const totals = calculateTotals(
    lineItems as Pick<InvoiceLineItemFormData, 'quantity' | 'unitPrice'>[],
    {
      discountMode,
      discountValue,
      taxMode,
      taxValue,
      shippingMode,
      shippingValue,
    }
  )

  const fmt = (amount: number) => formatInvoiceAmount(amount, currency)

  return (
    <div
      className="rounded-md border bg-muted/30 p-4 space-y-2"
      data-testid="invoice-totals-preview"
    >
      <div className="flex justify-between text-sm">
        <span className="text-muted-foreground">{m['billing.invoice_subtotal']()}</span>
        <span className="font-mono" data-testid="invoice-totals-subtotal">
          {fmt(totals.subtotal)}
        </span>
      </div>
      <div className="flex justify-between text-sm">
        <span className="text-muted-foreground">{m['billing.invoice_discount']()}</span>
        <span className="font-mono" data-testid="invoice-totals-discount">
          -{fmt(totals.discountAmount)}
        </span>
      </div>
      <div className="flex justify-between text-sm">
        <span className="text-muted-foreground">{m['billing.invoice_tax']()}</span>
        <span className="font-mono" data-testid="invoice-totals-tax">
          +{fmt(totals.taxAmount)}
        </span>
      </div>
      <div className="flex justify-between text-sm">
        <span className="text-muted-foreground">{m['billing.invoice_shipping']()}</span>
        <span className="font-mono" data-testid="invoice-totals-shipping">
          +{fmt(totals.shippingAmount)}
        </span>
      </div>
      <div className="border-t pt-2 flex justify-between font-semibold">
        <span>{m['billing.invoice_total_label']()}</span>
        <span className="font-mono" data-testid="invoice-totals-total">
          {fmt(totals.total)}
        </span>
      </div>
    </div>
  )
}

function CurrencySelect({ form }: { form: any }) {
  return (
    <form.Field name="currency">
      {(currencyField: any) => (
        <div className="space-y-1">
          <Label htmlFor="currency" className="text-xs">
            {m['billing.invoice_currency']()}
          </Label>
          <Select
            value={currencyField.state.value}
            onValueChange={(val: string) => currencyField.handleChange(val)}
          >
            <SelectTrigger className="h-8 text-sm" data-testid="invoice-currency-select">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="CNY">CNY (¥)</SelectItem>
              <SelectItem value="USD">USD ($)</SelectItem>
              <SelectItem value="EUR">EUR (€)</SelectItem>
            </SelectContent>
          </Select>
        </div>
      )}
    </form.Field>
  )
}

/* eslint-enable @typescript-eslint/no-explicit-any */
