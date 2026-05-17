import { useMemo, useCallback } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { Button } from '@/components/ui/button'
import { type SubscriptionPlanResponse, createPlan, updatePlan } from '@/lib/api-generated'
import {
  subscriptionPlanSchema,
  type SubscriptionPlanFormData,
  getSubscriptionPlanDefaults,
} from '@/lib/schemas/billing-forms'
import { productsQueryOptions, queryKeys } from '@/data/query-options'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { NumberField, TextField, TextareaField } from '@/components/shared/form-fields'
import { ArrowLeft } from 'lucide-react'
import { toast } from 'sonner'

interface PlanFormPageProps {
  mode: 'create' | 'edit'
  realmId: string
  plan?: SubscriptionPlanResponse
}

export function PlanFormPage({ mode, realmId, plan }: PlanFormPageProps) {
  const isEditing = mode === 'edit'
  const navigate = useNavigate()
  const queryClient = useQueryClient()

  const { data: products } = useQuery(productsQueryOptions(realmId))

  const defaultValues = useMemo(() => getSubscriptionPlanDefaults(plan), [plan])

  const saveMutation = useMutation({
    mutationFn: async (formData: SubscriptionPlanFormData) => {
      if (isEditing && plan) {
        const response = await updatePlan({
          path: { realmId, planId: plan.id },
          body: formData,
        })
        if (response.error) throw response.error
        if (!response.data) throw new Error('Failed to update plan')
        return response.data
      } else {
        const response = await createPlan({
          path: { realmId },
          body: formData,
        })
        if (response.error) throw response.error
        if (!response.data) throw new Error('Failed to create plan')
        return response.data
      }
    },
    onSuccess: async (data: SubscriptionPlanResponse) => {
      const action = isEditing ? 'updated' : 'created'
      toast.success(`Subscription Plan "${data?.title}" ${action} successfully`)
      await queryClient.invalidateQueries({ queryKey: queryKeys.billingPlans(realmId) })
      navigate({
        to: '/$realmId/manage/billing',
        params: { realmId },
        search: { status: 'all' },
      })
    },
    onError: (error: Error) => {
      toast.error(`Failed to save plan: ${error.message}`)
    },
  })

  const form = useAppForm({
    schema: subscriptionPlanSchema,
    defaultValues,
    onSubmit: async ({ value }) => {
      await saveMutation.mutateAsync(value)
    },
  })

  const handleCancel = useCallback(() => {
    navigate({
      to: '/$realmId/manage/billing',
      params: { realmId },
      search: { status: 'all' },
    })
  }, [navigate, realmId])

  return (
    <div className="space-y-6" data-testid="plan-form-page">
      <div className="flex items-center gap-4">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={handleCancel}
          data-testid="plan-form-back-button"
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <div>
          <h1 className="text-2xl font-bold" data-testid="plan-form-title">
            {isEditing ? 'Edit Subscription Plan' : 'Create Subscription Plan'}
          </h1>
          <p className="text-muted-foreground text-sm">
            {isEditing ? 'Update subscription plan details' : 'Create a new subscription plan'}
          </p>
        </div>
      </div>

      <form
        onSubmit={(e) => {
          e.preventDefault()
          e.stopPropagation()
          form.handleSubmit()
        }}
        className="max-w-3xl space-y-6"
      >
        <AppForm>
          <div className="space-y-6">
            {/* Basic Fields */}
            <div className="space-y-4">
              <div className="space-y-2">
                <form.Field
                  name="productId"
                  children={(field) => (
                    <>
                      <Label>
                        Product <span className="text-destructive">*</span>
                      </Label>
                      <Select
                        data-testid="plan-product-select"
                        value={field.state.value || ''}
                        onValueChange={(value) => field.handleChange(value)}
                      >
                        <SelectTrigger data-testid="plan-product-select-trigger">
                          <SelectValue placeholder="Select a product" />
                        </SelectTrigger>
                        <SelectContent>
                          {products?.map((product) => (
                            <SelectItem
                              key={product.id}
                              value={product.id}
                              data-testid={`plan-product-${product.id}`}
                            >
                              {product.title}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      {(field.state.meta.isTouched || form.state.isSubmitted) &&
                        field.state.meta.errors.length > 0 && (
                          <p className="text-sm text-destructive">
                            {getFieldErrorMessage(field.state.meta)}
                          </p>
                        )}
                    </>
                  )}
                />
                <TextField
                  form={form}
                  name="name"
                  label="Plan Name"
                  dataTestId="plan-name-input"
                  placeholder="basic-monthly"
                  disabled={isEditing}
                  required
                />
                <TextField
                  form={form}
                  name="title"
                  label="Title"
                  dataTestId="plan-title-input"
                  placeholder="Basic Monthly"
                  required
                />
                <TextareaField
                  form={form}
                  name="description"
                  label="Description"
                  dataTestId="plan-description-input"
                  placeholder="Subscription plan description"
                />
              </div>
            </div>

            {/* Pricing Fields */}
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <form.Field
                    name="type"
                    children={(field) => (
                      <>
                        <Label>
                          Billing Period <span className="text-destructive">*</span>
                        </Label>
                        <Select
                          data-testid="plan-type-select"
                          value={field.state.value || 'monthly'}
                          onValueChange={(value) =>
                            field.handleChange(value as 'monthly' | 'yearly')
                          }
                        >
                          <SelectTrigger data-testid="plan-type-select-trigger">
                            <SelectValue placeholder="Select period" />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="monthly" data-testid="plan-type-monthly">
                              Monthly
                            </SelectItem>
                            <SelectItem value="yearly" data-testid="plan-type-yearly">
                              Yearly
                            </SelectItem>
                          </SelectContent>
                        </Select>
                        {(field.state.meta.isTouched || form.state.isSubmitted) &&
                          field.state.meta.errors.length > 0 && (
                            <p className="text-sm text-destructive">
                              {getFieldErrorMessage(field.state.meta)}
                            </p>
                          )}
                      </>
                    )}
                  />
                </div>

                <div className="space-y-2">
                  <form.Field
                    name="currency"
                    children={(field) => (
                      <>
                        <Label>
                          Currency <span className="text-destructive">*</span>
                        </Label>
                        <Select
                          data-testid="plan-currency-select"
                          value={field.state.value || 'USD'}
                          onValueChange={(value) => field.handleChange(value)}
                        >
                          <SelectTrigger data-testid="plan-currency-select-trigger">
                            <SelectValue placeholder="USD" />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="USD" data-testid="plan-currency-usd">
                              USD
                            </SelectItem>
                            <SelectItem value="EUR" data-testid="plan-currency-eur">
                              EUR
                            </SelectItem>
                            <SelectItem value="GBP" data-testid="plan-currency-gbp">
                              GBP
                            </SelectItem>
                          </SelectContent>
                        </Select>
                        {(field.state.meta.isTouched || form.state.isSubmitted) &&
                          field.state.meta.errors.length > 0 && (
                            <p className="text-sm text-destructive">
                              {getFieldErrorMessage(field.state.meta)}
                            </p>
                          )}
                      </>
                    )}
                  />
                </div>
              </div>

              <NumberField
                form={form}
                name="price"
                label="Price (USD)"
                dataTestId="plan-price-input"
                placeholder="10.00"
                min={0.01}
                max={99999.99}
                step="0.01"
                helpText="Price in dollars (e.g., 10.00 = $10.00)"
                required
              />
            </div>

            {/* Checkout URL */}
            <div className="space-y-4">
              <TextField
                form={form}
                name="checkoutUrl"
                label="Checkout URL"
                dataTestId="plan-checkout-url-input"
                placeholder="https://checkout.example.com/..."
                transformValue={(value) => (value === '' ? undefined : value)}
              />
            </div>

            {/* Advanced Fields */}
            <div className="space-y-4">
              <NumberField
                form={form}
                name="trialDays"
                label="Trial Days"
                dataTestId="plan-trial-days-input"
                placeholder="14"
                min={0}
                max={365}
                helpText="Number of free trial days (0 for no trial)"
              />
              <NumberField
                form={form}
                name="sortOrder"
                label="Sort Order"
                dataTestId="plan-sort-order-input"
                placeholder="0"
                min={0}
                helpText="Display order in plan list (lower numbers appear first)"
              />
              <div className="flex items-center space-x-2">
                <form.Field
                  name="active"
                  children={(field) => (
                    <>
                      <Label htmlFor={field.name}>Active</Label>
                      <Switch
                        id={field.name}
                        data-testid="plan-active-switch"
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
                <p className="text-xs text-muted-foreground">
                  Enable/disable plan for new subscriptions
                </p>
              </div>
            </div>
          </div>
        </AppForm>

        <div className="flex items-center gap-3 pt-4 border-t">
          <Button
            type="button"
            variant="outline"
            onClick={handleCancel}
            data-testid="plan-form-cancel-button"
          >
            Cancel
          </Button>
          <Button
            type="submit"
            disabled={saveMutation.isPending}
            data-testid="plan-form-submit-button"
          >
            {saveMutation.isPending
              ? 'Saving...'
              : isEditing
                ? 'Update Subscription Plan'
                : 'Create Subscription Plan'}
          </Button>
        </div>
      </form>
    </div>
  )
}
