import { useMemo } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import {
  type PointsPackageResponse,
  createPointsPackage,
  updatePointsPackage,
} from '@/lib/api-generated'
import {
  pointsPackageFormSchema,
  type PointsPackageFormData,
  getPointsPackageDefaults,
  apiPriceToDisplayPrice,
  displayPriceToApiPrice,
} from '@/lib/schemas/points-package-forms'
import { Label } from '@/components/ui/label'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { Switch } from '@/components/ui/switch'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { NumberField, TextField, TextareaField } from '@/components/shared/form-fields'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { handleApiResponse } from '@/lib/api-utils'
import { queryKeys } from '@/data/query-options'
import { ArrowLeft } from 'lucide-react'

interface PointsPackageFormPageProps {
  mode: 'create' | 'edit'
  realmId: string
  pkg?: PointsPackageResponse
}

export function PointsPackageFormPage({ mode, realmId, pkg }: PointsPackageFormPageProps) {
  const isCreate = mode === 'create'
  const navigate = useNavigate()

  const handleCancel = () => {
    navigate({ to: '/$realmId/manage/points-packages', params: { realmId } })
  }

  const defaultValues = useMemo(
    () =>
      getPointsPackageDefaults(
        pkg
          ? {
              name: pkg.name,
              title: pkg.title,
              description: pkg.description ?? undefined,
              points: pkg.points,
              price: apiPriceToDisplayPrice(pkg.price, pkg.currency),
              currency: pkg.currency,
              sortOrder: pkg.sortOrder,
              enabled: pkg.enabled,
              packageType: (pkg.packageType as 'standard' | 'promotional') ?? 'standard',
              originalPrice: pkg.originalPrice ?? undefined,
              promoStartTime: pkg.promoStartTime ?? '',
              promoEndTime: pkg.promoEndTime ?? '',
            }
          : undefined
      ),
    [pkg]
  )

  const { isSubmitting, mutate } = useFormMutation({
    mutationFn: (data: PointsPackageFormData) => {
      const apiPrice = displayPriceToApiPrice(data.price, data.currency)
      const toApiTime = (v: string | undefined | null) => (v ? new Date(v).toISOString() : null)

      if (isCreate) {
        return createPointsPackage({
          path: { realmId },
          body: {
            ...data,
            price: apiPrice,
            packageType: data.packageType,
            originalPrice:
              data.packageType === 'promotional' && data.originalPrice
                ? displayPriceToApiPrice(data.originalPrice, data.currency)
                : null,
            promoStartTime: toApiTime(data.promoStartTime),
            promoEndTime: toApiTime(data.promoEndTime),
          },
        }).then(handleApiResponse)
      }

      return updatePointsPackage({
        path: { realmId, packageId: pkg!.id },
        body: {
          title: data.title,
          description: data.description ?? null,
          price: apiPrice,
          currency: data.currency,
          sortOrder: data.sortOrder,
          enabled: data.enabled,
          packageType: data.packageType,
          originalPrice:
            data.packageType === 'promotional' && data.originalPrice
              ? displayPriceToApiPrice(data.originalPrice, data.currency)
              : null,
          promoStartTime: toApiTime(data.promoStartTime),
          promoEndTime: toApiTime(data.promoEndTime),
        },
      }).then(handleApiResponse)
    },
    getSuccessMessage: (data) =>
      `Points package "${data?.title}" ${isCreate ? 'created' : 'updated'} successfully`,
    invalidateQueries: [queryKeys.pointsPackages(realmId), queryKeys.featureAvailability(realmId)],
    onSuccess: () => {
      navigate({ to: '/$realmId/manage/points-packages', params: { realmId } })
    },
  })

  const form = useAppForm({
    schema: pointsPackageFormSchema,
    defaultValues,
    onSubmit: async ({ value }) => {
      await mutate(value)
    },
  })

  return (
    <div
      className="container max-w-4xl mx-auto py-6 px-6 space-y-6"
      data-testid="points-package-form-page"
    >
      <div className="flex items-center gap-4">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={handleCancel}
          data-testid="points-package-form-back-button"
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <div>
          <h1 className="text-2xl font-bold" data-testid="page-title">
            {isCreate ? 'Create Points Package' : 'Edit Points Package'}
          </h1>
          <p className="text-muted-foreground text-sm">
            {isCreate
              ? 'Create a new points package for users to purchase'
              : 'Update points package details'}
          </p>
        </div>
      </div>

      <AppForm>
        <form
          onSubmit={(e) => {
            e.preventDefault()
            e.stopPropagation()
            form.handleSubmit()
          }}
          className="max-w-4xl space-y-6"
        >
          <Card>
            <CardContent className="pt-6 space-y-6">
              <div className="space-y-2">
                <TextField
                  form={form}
                  name="name"
                  label="Package Name"
                  dataTestId="points-package-name-input"
                  placeholder="basic-points-package"
                  disabled={!isCreate}
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
                    disabled={!isCreate}
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

              <form.Field name="packageType">
                {(field) => (
                  <div className="space-y-2">
                    <Label>Package Type</Label>
                    <RadioGroup
                      value={field.state.value}
                      onValueChange={(val) => {
                        field.handleChange(val as 'standard' | 'promotional')
                        if (val === 'standard') {
                          form.setFieldValue('originalPrice', undefined)
                          form.setFieldValue('promoStartTime', '')
                          form.setFieldValue('promoEndTime', '')
                        }
                      }}
                      className="flex gap-4"
                    >
                      <div className="flex items-center space-x-2">
                        <RadioGroupItem
                          value="standard"
                          id="package-type-standard"
                          data-testid="points-package-type-standard"
                        />
                        <Label htmlFor="package-type-standard">Standard</Label>
                      </div>
                      <div className="flex items-center space-x-2">
                        <RadioGroupItem
                          value="promotional"
                          id="package-type-promotional"
                          data-testid="points-package-type-promotional"
                        />
                        <Label htmlFor="package-type-promotional">Promotional</Label>
                      </div>
                    </RadioGroup>
                  </div>
                )}
              </form.Field>

              <form.Subscribe selector={(state) => state.values.packageType}>
                {(packageType) =>
                  packageType === 'promotional' && (
                    <div className="space-y-4 pl-4 border-l-2 border-primary/20">
                      <div className="grid grid-cols-2 gap-4">
                        <div className="space-y-2">
                          <NumberField
                            form={form}
                            name="originalPrice"
                            label="Original Price (Before Discount)"
                            dataTestId="points-package-original-price-input"
                            placeholder="19.99"
                            step="0.01"
                            helpText="Must be greater than selling price"
                            required
                          />
                        </div>
                      </div>

                      <div className="grid grid-cols-2 gap-4">
                        <TextField
                          form={form}
                          name="promoStartTime"
                          label="Promotion Start Time"
                          dataTestId="points-package-promo-start-input"
                          type="datetime-local"
                        />
                        <TextField
                          form={form}
                          name="promoEndTime"
                          label="Promotion End Time"
                          dataTestId="points-package-promo-end-input"
                          type="datetime-local"
                        />
                      </div>
                    </div>
                  )
                }
              </form.Subscribe>
            </CardContent>
          </Card>

          <div className="flex justify-end gap-2">
            <Button
              type="button"
              variant="outline"
              onClick={handleCancel}
              data-testid="points-package-cancel-button"
            >
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={isSubmitting}
              data-testid="points-package-submit-button"
            >
              {isSubmitting ? 'Saving...' : isCreate ? 'Create Package' : 'Save Changes'}
            </Button>
          </div>
        </form>
      </AppForm>
    </div>
  )
}
