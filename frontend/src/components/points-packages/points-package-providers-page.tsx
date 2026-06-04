import { useEffect, useMemo, useState } from 'react'
import { type ColumnDef } from '@tanstack/react-table'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { z } from 'zod'
import { toast } from 'sonner'
import {
  ArrowLeft,
  Edit,
  MoreHorizontal,
  Plus,
  ToggleLeft,
  ToggleRight,
  Trash2,
} from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { BaseFormDialog, ConfirmDialog, DataTable, PageHeader } from '@/components/shared'
import { TextField } from '@/components/shared/form-fields'
import { getFieldErrorMessage } from '@/lib/form-utils'
import {
  paymentProviderMappingsQueryOptions,
  paymentProvidersQueryOptions,
  pointsPackageQueryOptions,
  queryKeys,
} from '@/data/query-options'
import {
  createPaymentProviderMapping,
  deletePaymentProviderMapping,
  updatePaymentProviderMapping,
  type PaymentProviderMappingResponse,
} from '@/lib/api-generated'
import { formatProviderName } from '@/components/billing/format-provider-name'
import { m } from '@/paraglide/messages'

const packageProviderMappingSchema = z.object({
  paymentProvider: z.string().min(1, m['points.packages_providers_form_provider_required']()),
  externalProductId: z
    .string()
    .max(255, m['points.packages_providers_form_product_id_max_length']())
    .transform((value) => (value === '' ? null : value)),
  enabled: z.boolean().default(true),
})

type PackageProviderMappingFormData = z.infer<typeof packageProviderMappingSchema>

interface PointsPackageProvidersPageProps {
  realmId: string
  packageId: string
}

function getMappingDefaults(
  mapping?: PaymentProviderMappingResponse
): PackageProviderMappingFormData {
  return {
    paymentProvider: mapping?.paymentProvider ?? '',
    externalProductId: mapping?.externalProductId ?? '',
    enabled: mapping?.enabled ?? true,
  }
}

function createMappingColumns(
  onEdit: (mapping: PaymentProviderMappingResponse) => void,
  onToggle: (mapping: PaymentProviderMappingResponse) => void,
  onDelete: (mapping: PaymentProviderMappingResponse) => void
): ColumnDef<PaymentProviderMappingResponse>[] {
  return [
    {
      accessorKey: 'paymentProvider',
      header: 'Payment Provider',
      cell: ({ row }) => (
        <Badge variant="outline" data-testid={`package-provider-name-${row.original.id}`}>
          {formatProviderName(row.original.paymentProvider)}
        </Badge>
      ),
    },
    {
      accessorKey: 'externalProductId',
      header: 'External Product ID',
      cell: ({ row }) => (
        <span
          className="font-mono text-xs"
          data-testid={`package-provider-product-id-${row.original.id}`}
        >
          {row.original.externalProductId || '-'}
        </span>
      ),
    },
    {
      accessorKey: 'enabled',
      header: 'Status',
      cell: ({ row }) => (
        <Badge variant={row.original.enabled ? 'default' : 'secondary'}>
          {row.original.enabled ? 'Enabled' : 'Disabled'}
        </Badge>
      ),
    },
    {
      id: 'actions',
      header: 'Actions',
      cell: ({ row }) => (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" className="h-8 w-8 p-0">
              <span className="sr-only">Open menu</span>
              <MoreHorizontal className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem
              onClick={() => onEdit(row.original)}
              data-testid={`payment-provider-edit-button-${row.original.paymentProvider}`}
            >
              <Edit className="mr-2 h-4 w-4" />
              Edit
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => onToggle(row.original)}
              data-testid={`payment-provider-enabled-switch-${row.original.paymentProvider}`}
            >
              {row.original.enabled ? (
                <>
                  <ToggleLeft className="mr-2 h-4 w-4" />
                  Disable
                </>
              ) : (
                <>
                  <ToggleRight className="mr-2 h-4 w-4" />
                  Enable
                </>
              )}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => onDelete(row.original)}
              className="text-destructive"
              data-testid={`payment-provider-remove-button-${row.original.paymentProvider}`}
            >
              <Trash2 className="mr-2 h-4 w-4" />
              Delete
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      ),
    },
  ]
}

function PointsPackageProviderFormDialog({
  open,
  onOpenChange,
  onSubmit,
  isSubmitting,
  mapping,
  availableProviders,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  onSubmit: (data: PackageProviderMappingFormData) => void
  isSubmitting: boolean
  mapping?: PaymentProviderMappingResponse
  availableProviders: string[]
}) {
  const isEditing = !!mapping
  const defaultValues = useMemo(() => getMappingDefaults(mapping), [mapping])
  const form = useAppForm({
    schema: packageProviderMappingSchema,
    defaultValues,
    onSubmit: async ({ value }) => {
      await onSubmit(value)
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
      title={
        isEditing
          ? m['points.packages_providers_form_edit_title']()
          : m['points.packages_providers_form_add_title']()
      }
      description={
        isEditing
          ? m['points.packages_providers_form_edit_description']()
          : m['points.packages_providers_form_add_description']()
      }
      className="max-w-lg"
      data-testid="provider-mapping-form-dialog"
      footer={
        <>
          <Button
            type="button"
            variant="outline"
            data-testid="provider-mapping-cancel-button"
            onClick={() => onOpenChange(false)}
          >
            Cancel
          </Button>
          <Button
            type="submit"
            form="points-package-provider-mapping-form"
            disabled={isSubmitting || (!isEditing && availableProviders.length === 0)}
            data-testid="provider-mapping-submit-button"
          >
            {isSubmitting
              ? m['shared.saving']()
              : isEditing
                ? m['points.packages_providers_form_update']()
                : m['points.packages_providers_form_add']()}
          </Button>
        </>
      }
    >
      <form
        id="points-package-provider-mapping-form"
        onSubmit={(event) => {
          event.preventDefault()
          event.stopPropagation()
          form.handleSubmit()
        }}
      >
        <AppForm>
          <div className="space-y-4">
            {isEditing ? (
              <div className="space-y-2">
                <Label>{m['points.packages_providers_form_provider_label']()}</Label>
                <div
                  className="flex h-10 items-center rounded-md border bg-muted px-3 text-sm"
                  data-testid="provider-mapping-provider-readonly"
                >
                  {formatProviderName(mapping.paymentProvider)}
                </div>
              </div>
            ) : (
              <form.Field
                name="paymentProvider"
                children={(field) => (
                  <div className="space-y-2">
                    <Label>
                      {m['points.packages_providers_form_provider_label']()}{' '}
                      <span className="text-destructive">*</span>
                    </Label>
                    {availableProviders.length === 0 ? (
                      <div
                        className="text-sm text-muted-foreground"
                        data-testid="no-providers-message"
                      >
                        {m['points.packages_providers_form_no_providers']()}
                      </div>
                    ) : (
                      <Select
                        data-testid="provider-mapping-provider-select"
                        value={field.state.value || ''}
                        onValueChange={(value) => field.handleChange(value)}
                      >
                        <SelectTrigger data-testid="provider-mapping-provider-select-trigger">
                          <SelectValue
                            placeholder={m['points.packages_providers_form_provider_placeholder']()}
                          />
                        </SelectTrigger>
                        <SelectContent>
                          {availableProviders.map((provider) => (
                            <SelectItem
                              key={provider}
                              value={provider}
                              data-testid={`provider-option-${provider}`}
                            >
                              {formatProviderName(provider)}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    )}
                    {(field.state.meta.isTouched || form.state.isSubmitted) &&
                      field.state.meta.errors.length > 0 && (
                        <p className="text-sm text-destructive">
                          {getFieldErrorMessage(field.state.meta)}
                        </p>
                      )}
                  </div>
                )}
              />
            )}

            <TextField
              form={form}
              name="externalProductId"
              label={m['points.packages_providers_form_product_id_label']()}
              dataTestId="provider-mapping-product-id-input"
              placeholder={m['points.packages_providers_form_product_id_placeholder']()}
            />

            <div className="flex items-center space-x-2">
              <form.Field
                name="enabled"
                children={(field) => (
                  <>
                    <Label htmlFor="package-provider-enabled">
                      {m['points.packages_providers_form_enabled_label']()}
                    </Label>
                    <Switch
                      id="package-provider-enabled"
                      data-testid="provider-mapping-enabled-switch"
                      checked={field.state.value}
                      onCheckedChange={(checked) => field.handleChange(checked)}
                    />
                  </>
                )}
              />
            </div>
          </div>
        </AppForm>
      </form>
    </BaseFormDialog>
  )
}

export function PointsPackageProvidersPage({
  realmId,
  packageId,
}: PointsPackageProvidersPageProps) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const [editingMapping, setEditingMapping] = useState<PaymentProviderMappingResponse | undefined>()
  const [deletingMapping, setDeletingMapping] = useState<
    PaymentProviderMappingResponse | undefined
  >()
  const [formOpen, setFormOpen] = useState(false)

  const { data: pkg } = useQuery(pointsPackageQueryOptions(realmId, packageId))
  const {
    data: mappings = [],
    isLoading,
    error,
  } = useQuery(paymentProviderMappingsQueryOptions(realmId, packageId))
  const { data: providers = [] } = useQuery(paymentProvidersQueryOptions(realmId))

  const availableProviders = useMemo(() => {
    const usedProviders = new Set(mappings.map((mapping) => mapping.paymentProvider))
    return providers
      .map((provider) => provider.platform)
      .filter((provider) => !usedProviders.has(provider))
  }, [mappings, providers])

  async function invalidateProviderQueries() {
    await queryClient.invalidateQueries({
      queryKey: paymentProviderMappingsQueryOptions(realmId, packageId).queryKey,
    })
    await queryClient.invalidateQueries({ queryKey: queryKeys.pointsPackages(realmId) })
    await queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })
  }

  const createMutation = useMutation({
    mutationFn: async (data: PackageProviderMappingFormData) => {
      const response = await createPaymentProviderMapping({
        path: { realmId, packageId },
        body: {
          paymentProvider: data.paymentProvider,
          externalProductId: data.externalProductId,
          enabled: data.enabled,
        },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: async () => {
      toast.success(m['points.packages_providers_added']())
      setFormOpen(false)
      await invalidateProviderQueries()
    },
    onError: (error: Error) => {
      toast.error(m['points.packages_providers_add_failed']({ message: error.message }))
    },
  })

  const updateMutation = useMutation({
    mutationFn: async ({
      mapping,
      data,
    }: {
      mapping: PaymentProviderMappingResponse
      data: PackageProviderMappingFormData
    }) => {
      const response = await updatePaymentProviderMapping({
        path: { realmId, packageId, mappingId: mapping.id },
        body: {
          externalProductId: data.externalProductId,
          enabled: data.enabled,
        },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: async () => {
      toast.success(m['points.packages_providers_updated']())
      setFormOpen(false)
      setEditingMapping(undefined)
      await invalidateProviderQueries()
    },
    onError: (error: Error) => {
      toast.error(m['points.packages_providers_update_failed']({ message: error.message }))
    },
  })

  const deleteMutation = useMutation({
    mutationFn: async (mappingId: string) => {
      const response = await deletePaymentProviderMapping({
        path: { realmId, packageId, mappingId },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: async () => {
      toast.success(m['points.packages_providers_removed']())
      setDeletingMapping(undefined)
      await invalidateProviderQueries()
    },
    onError: (error: Error) => {
      toast.error(m['points.packages_providers_remove_failed']({ message: error.message }))
    },
  })

  async function handleSubmit(data: PackageProviderMappingFormData) {
    if (editingMapping) {
      await updateMutation.mutateAsync({ mapping: editingMapping, data })
    } else {
      await createMutation.mutateAsync(data)
    }
  }

  function handleAdd() {
    setEditingMapping(undefined)
    setFormOpen(true)
  }

  function handleEdit(mapping: PaymentProviderMappingResponse) {
    setEditingMapping(mapping)
    setFormOpen(true)
  }

  function handleFormOpenChange(open: boolean) {
    setFormOpen(open)
    if (!open) {
      setEditingMapping(undefined)
    }
  }

  const columns = createMappingColumns(
    handleEdit,
    (mapping) =>
      updateMutation.mutate({
        mapping,
        data: {
          paymentProvider: mapping.paymentProvider,
          externalProductId: mapping.externalProductId ?? '',
          enabled: !mapping.enabled,
        },
      }),
    setDeletingMapping
  )

  return (
    <div className="space-y-6" data-testid="points-package-providers-page">
      <PageHeader
        title={m['points.packages_providers_page_title']()}
        subtitle={
          pkg?.title
            ? m['points.packages_providers_page_subtitle']({ title: pkg.title })
            : undefined
        }
        action={{
          label: m['points.packages_providers_back_button'](),
          onClick: () =>
            navigate({
              to: '/$realmId/manage/points-packages',
              params: { realmId },
            }),
          testId: 'back-to-points-packages-button',
          icon: <ArrowLeft className="mr-2 h-4 w-4" />,
        }}
      />

      <div className="space-y-4" data-testid="payment-provider-list">
        <div className="flex items-center justify-between">
          <h4 className="text-sm font-medium">{m['points.packages_providers_heading']()}</h4>
          <Button onClick={handleAdd} size="sm" data-testid="add-provider-mapping-button">
            <Plus className="mr-2 h-4 w-4" />
            {m['points.packages_providers_add_button']()}
          </Button>
        </div>

        <DataTable
          columns={columns}
          data={mappings}
          isLoading={isLoading}
          error={error ?? undefined}
          loadingMessage={m['points.packages_providers_loading']()}
          errorMessage={
            error ? m['points.packages_providers_error']({ message: error.message }) : undefined
          }
          emptyMessage={m['points.packages_providers_empty']()}
          data-testid="provider-mapping-table"
        />
      </div>

      <PointsPackageProviderFormDialog
        open={formOpen}
        onOpenChange={handleFormOpenChange}
        onSubmit={handleSubmit}
        isSubmitting={createMutation.isPending || updateMutation.isPending}
        mapping={editingMapping}
        availableProviders={availableProviders}
      />

      <ConfirmDialog
        open={!!deletingMapping}
        onOpenChange={(open) => !open && setDeletingMapping(undefined)}
        title={m['points.packages_providers_delete_title']()}
        description={m['points.packages_providers_delete_description']({
          name: formatProviderName(deletingMapping?.paymentProvider ?? ''),
        })}
        onConfirm={() => deletingMapping && deleteMutation.mutate(deletingMapping.id)}
        isPending={deleteMutation.isPending}
        confirmTestId="confirm-delete-mapping-button"
      />
    </div>
  )
}
