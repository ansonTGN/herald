import { useEffect, useMemo } from 'react'
import { Button } from '@/components/ui/button'
import { type ProductResponse } from '@/lib/api-generated'
import {
  productFormSchema,
  type ProductFormData,
  getProductDefaults,
} from '@/lib/schemas/billing-forms'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { BaseFormDialog } from '@/components/shared/form-dialog'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { TextField, TextareaField } from '@/components/shared/form-fields'
import { m } from '@/paraglide/messages'

interface ProductFormDialogProps {
  product?: ProductResponse
  open: boolean
  onOpenChange: (open: boolean) => void
  onSubmit: (data: ProductFormData) => void
  isSubmitting: boolean
}

export function ProductFormDialog({
  product,
  open,
  onOpenChange,
  onSubmit,
  isSubmitting,
}: ProductFormDialogProps) {
  const isEditing = !!product
  const defaultValues = useMemo(
    () =>
      getProductDefaults(
        product
          ? {
              code: product.code,
              title: product.title,
              description: product.description ?? undefined,
              enabled: product.enabled,
            }
          : undefined
      ),
    [product]
  )

  const form = useAppForm({
    schema: productFormSchema,
    defaultValues,
    onSubmit: ({ value }) => onSubmit(value),
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
      title={isEditing ? m['billing.edit_product']() : m['billing.create_product']()}
      description={
        isEditing
          ? m['billing.edit_product_description']()
          : m['billing.create_product_description']()
      }
      className="max-w-lg"
      isSubmitting={isSubmitting}
      data-testid="product-form-dialog"
      footer={
        <>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
            data-testid="product-form-cancel-button"
          >
            {m['common.cancel']()}
          </Button>
          <Button
            type="submit"
            form="product-form"
            disabled={isSubmitting}
            data-testid="product-form-submit-button"
          >
            {isSubmitting
              ? m['shared.saving']()
              : isEditing
                ? m['billing.update_product']()
                : m['billing.create_product']()}
          </Button>
        </>
      }
    >
      <form
        id="product-form"
        onSubmit={(e) => {
          e.preventDefault()
          e.stopPropagation()
          form.handleSubmit()
        }}
      >
        <AppForm>
          <div className="space-y-6">
            <div className="space-y-2">
              <TextField
                form={form}
                name="code"
                label={m['billing.label_product_code']()}
                dataTestId="product-code-input"
                placeholder="basic-product"
                disabled={isEditing}
                required
              />
              <TextField
                form={form}
                name="title"
                label={m['billing.label_title']()}
                dataTestId="product-title-input"
                placeholder="Basic Product"
                required
              />
              <TextareaField
                form={form}
                name="description"
                label={m['common.description']()}
                dataTestId="product-description-input"
                placeholder={m['common.description']()}
              />
            </div>

            <div className="flex items-center space-x-2">
              <form.Field
                name="enabled"
                children={(field) => (
                  <>
                    <Label htmlFor={field.name}>{m['billing.label_enabled']()}</Label>
                    <Switch
                      id={field.name}
                      data-testid="product-enabled-switch"
                      checked={field.state.value}
                      onCheckedChange={(checked) => field.handleChange(checked)}
                    />
                    {(field.state.meta.isTouched || form.state.isSubmitted) &&
                      field.state.meta.errors.length > 0 && (
                        <p className="text-sm text-destructive">
                          {getFieldErrorMessage(field.state.meta)}
                        </p>
                      )}
                  </>
                )}
              />
              <p className="text-xs text-muted-foreground">{m['billing.help_enabled']()}</p>
            </div>
          </div>
        </AppForm>
      </form>
    </BaseFormDialog>
  )
}
