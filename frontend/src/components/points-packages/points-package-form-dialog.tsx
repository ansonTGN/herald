import { useEffect, useMemo } from 'react'
import { Button } from '@/components/ui/button'
import { type PointsPackageResponse } from '@/lib/api-generated'
import {
  pointsPackageFormSchema,
  type PointsPackageFormData,
  getPointsPackageDefaults,
  apiPriceToDisplayPrice,
} from '@/lib/schemas/points-package-forms'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { NumberField, TextField, TextareaField } from '@/components/shared/form-fields'

interface PointsPackageFormDialogProps {
  package?: PointsPackageResponse
  open: boolean
  onOpenChange: (open: boolean) => void
  onSubmit: (data: PointsPackageFormData) => void
  isSubmitting: boolean
}

export function PointsPackageFormDialog({
  package: pkg,
  open,
  onOpenChange,
  onSubmit,
  isSubmitting,
}: PointsPackageFormDialogProps) {
  const isEditing = !!pkg
  const defaultValues = useMemo(
    () =>
      getPointsPackageDefaults(
        pkg
          ? {
              name: pkg.name,
              title: pkg.title,
              description: pkg.description ?? undefined,
              points: pkg.points,
              price: apiPriceToDisplayPrice(pkg.price, pkg.currency), // KEY: Convert cents to dollars
              currency: pkg.currency,
              sortOrder: pkg.sortOrder,
              enabled: pkg.enabled,
            }
          : undefined
      ),
    [pkg]
  )

  const form = useAppForm({
    schema: pointsPackageFormSchema,
    defaultValues,
    onSubmit: ({ value }) => onSubmit(value),
  })

  useEffect(() => {
    if (open) {
      form.reset(defaultValues)
    }
  }, [defaultValues, form, open])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg" data-testid="points-package-form-dialog">
        <DialogHeader>
          <DialogTitle>{isEditing ? 'Edit Points Package' : 'Create Points Package'}</DialogTitle>
          <DialogDescription>
            {isEditing
              ? 'Update points package details'
              : 'Create a new points package for users to purchase'}
          </DialogDescription>
        </DialogHeader>

        <AppForm>
          <form
            id="points-package-form"
            onSubmit={async (e) => {
              e.preventDefault()
              await form.validateAllFields('submit')
              if (form.state.isFieldsValid) {
                onSubmit(form.state.values)
              }
            }}
          >
            <div className="space-y-6">
              <div className="space-y-2">
                <TextField
                  form={form}
                  name="name"
                  label="Package Name"
                  dataTestId="points-package-name-input"
                  placeholder="basic-points-package"
                  disabled={isEditing}
                  required
                />
                <p className="text-xs text-muted-foreground">
                  Unique identifier for the package (lowercase letters, numbers, hyphens, and
                  underscores only)
                </p>
              </div>

              <div className="space-y-2">
                <TextField
                  form={form}
                  name="title"
                  label="Title"
                  dataTestId="points-package-title-input"
                  placeholder="Basic Points Package"
                  required
                />
                <p className="text-xs text-muted-foreground">Display name shown to users</p>
              </div>

              <div className="space-y-2">
                <TextareaField
                  form={form}
                  name="description"
                  label="Description"
                  dataTestId="points-package-description-input"
                  placeholder="A great starter package for new users"
                  rows={3}
                />
                <p className="text-xs text-muted-foreground">
                  Optional description of what users will get
                </p>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <NumberField
                    form={form}
                    name="points"
                    label="Points"
                    dataTestId="points-package-points-input"
                    placeholder="100"
                    required
                  />
                  <p className="text-xs text-muted-foreground">Number of points granted</p>
                </div>

                <div className="space-y-2">
                  <NumberField
                    form={form}
                    name="price"
                    label="Price"
                    dataTestId="points-package-price-input"
                    placeholder="9.99"
                    step="0.01"
                    required
                  />
                  <p className="text-xs text-muted-foreground">Price in specified currency</p>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <TextField
                    form={form}
                    name="currency"
                    label="Currency"
                    dataTestId="points-package-currency-select"
                    placeholder="USD"
                    required
                  />
                  <p className="text-xs text-muted-foreground">
                    ISO 4217 currency code (e.g., USD, EUR, CNY)
                  </p>
                </div>

                <div className="space-y-2">
                  <NumberField
                    form={form}
                    name="sortOrder"
                    label="Sort Order"
                    dataTestId="points-package-sort-order-input"
                    placeholder="0"
                  />
                  <p className="text-xs text-muted-foreground">Lower numbers appear first</p>
                </div>
              </div>

              <form.Field name="enabled">
                {(field) => (
                  <div className="flex items-center space-x-2">
                    <Switch
                      checked={field.state.value}
                      onCheckedChange={(checked) => field.handleChange(checked)}
                      data-testid="points-package-enabled-switch"
                    />
                    <Label htmlFor="enabled">Enabled</Label>
                    <p className="text-xs text-muted-foreground">
                      Allow users to purchase this package
                    </p>
                  </div>
                )}
              </form.Field>
            </div>

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
                data-testid="points-package-cancel-button"
              >
                Cancel
              </Button>
              <Button
                type="submit"
                disabled={isSubmitting}
                data-testid="points-package-submit-button"
              >
                {isSubmitting ? 'Saving...' : isEditing ? 'Update Package' : 'Create Package'}
              </Button>
            </DialogFooter>
          </form>
        </AppForm>
      </DialogContent>
    </Dialog>
  )
}
