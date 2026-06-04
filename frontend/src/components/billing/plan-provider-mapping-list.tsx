import { useState } from 'react'
import { type ColumnDef } from '@tanstack/react-table'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { MoreHorizontal, Edit, Trash2, ToggleLeft, ToggleRight, Plus } from 'lucide-react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { DataTable } from '@/components/shared/data-table'
import { queryKeys, subscriptionPlanProvidersQueryOptions } from '@/data/query-options'
import { removePaymentProviderFromPlan, togglePlanPaymentProvider } from '@/lib/api-generated'
import type { SubscriptionPlanPaymentProviderResponse } from '@/lib/api-generated'
import { toast } from 'sonner'
import { ConfirmDialog } from '@/components/shared'
import { formatProviderName } from './format-provider-name'
import { m } from '@/paraglide/messages'

interface PlanProviderMappingListProps {
  planId: string
  realmId: string
  onAdd: () => void
  onEdit: (mapping: SubscriptionPlanPaymentProviderResponse) => void
}

function createMappingColumns(
  onEdit: (mapping: SubscriptionPlanPaymentProviderResponse) => void,
  onDelete: (mapping: SubscriptionPlanPaymentProviderResponse) => void,
  onToggle: (mapping: SubscriptionPlanPaymentProviderResponse) => void
): ColumnDef<SubscriptionPlanPaymentProviderResponse>[] {
  return [
    {
      accessorKey: 'paymentProvider',
      header: m['billing.label_payment_provider'](),
      cell: ({ row }) => {
        const provider = row.getValue('paymentProvider') as string
        return (
          <Badge variant="outline" data-testid={`mapping-provider-name-${row.original.id}`}>
            {formatProviderName(provider)}
          </Badge>
        )
      },
    },
    {
      accessorKey: 'externalProductId',
      header: m['billing.col_external_product_id'](),
      cell: ({ row }) => (
        <span className="font-mono text-xs" data-testid={`mapping-product-id-${row.original.id}`}>
          {row.getValue('externalProductId')}
        </span>
      ),
    },
    {
      accessorKey: 'externalPriceId',
      header: m['billing.col_external_price_id'](),
      cell: ({ row }) => (
        <span className="font-mono text-xs text-muted-foreground">
          {(row.getValue('externalPriceId') as string | null) || '-'}
        </span>
      ),
    },
    {
      accessorKey: 'enabled',
      header: m['common.status'](),
      cell: ({ row }) => {
        const enabled = row.getValue('enabled') as boolean
        return (
          <Badge variant={enabled ? 'default' : 'secondary'}>
            {enabled ? m['common.enabled']() : m['common.disabled']()}
          </Badge>
        )
      },
    },
    {
      id: 'actions',
      header: m['common.actions'](),
      cell: ({ row }) => (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" className="h-8 w-8 p-0">
              <span className="sr-only">{m['billing.open_menu']()}</span>
              <MoreHorizontal className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem
              onClick={() => onEdit(row.original)}
              data-testid={`edit-mapping-button-${row.original.id}`}
            >
              <Edit className="mr-2 h-4 w-4" />
              {m['common.edit']()}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => onToggle(row.original)}
              data-testid={`toggle-mapping-button-${row.original.id}`}
            >
              {row.original.enabled ? (
                <>
                  <ToggleLeft className="mr-2 h-4 w-4" />
                  {m['common.disabled']()}
                </>
              ) : (
                <>
                  <ToggleRight className="mr-2 h-4 w-4" />
                  {m['common.enabled']()}
                </>
              )}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => onDelete(row.original)}
              className="text-destructive"
              data-testid={`delete-mapping-button-${row.original.id}`}
            >
              <Trash2 className="mr-2 h-4 w-4" />
              {m['common.delete']()}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      ),
    },
  ]
}

export function PlanProviderMappingList({
  planId,
  realmId,
  onAdd,
  onEdit,
}: PlanProviderMappingListProps) {
  const queryClient = useQueryClient()
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false)
  const [deletingMapping, setDeletingMapping] =
    useState<SubscriptionPlanPaymentProviderResponse | null>(null)

  const {
    data: mappings = [],
    isLoading,
    error,
  } = useQuery(subscriptionPlanProvidersQueryOptions(realmId, planId))

  const deleteMutation = useMutation({
    mutationFn: async (mappingId: string) => {
      const response = await removePaymentProviderFromPlan({
        path: { realmId, planId, mappingId },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      toast.success(m['billing.mapping_deleted']())
      setDeleteConfirmOpen(false)
      setDeletingMapping(null)
      // Invalidate queries to refresh the mapping list
      queryClient.invalidateQueries({
        queryKey: subscriptionPlanProvidersQueryOptions(realmId, planId).queryKey,
      })
      queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })
    },
    onError: (err: Error) => {
      toast.error(m['billing.mapping_delete_failed']({ message: err.message }))
    },
  })

  const toggleMutation = useMutation({
    mutationFn: async (mapping: SubscriptionPlanPaymentProviderResponse) => {
      const response = await togglePlanPaymentProvider({
        path: { realmId, planId, mappingId: mapping.id },
        body: { enabled: !mapping.enabled },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: (_, variables) => {
      const action = variables.enabled ? m['common.disabled']() : m['common.enabled']()
      toast.success(m['billing.mapping_toggled']({ action }))
      // Invalidate queries to refresh the mapping list
      queryClient.invalidateQueries({
        queryKey: subscriptionPlanProvidersQueryOptions(realmId, planId).queryKey,
      })
      queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })
    },
    onError: (err: Error) => {
      toast.error(m['billing.mapping_toggle_failed']({ message: err.message }))
    },
  })

  function handleDelete(mapping: SubscriptionPlanPaymentProviderResponse) {
    setDeletingMapping(mapping)
    setDeleteConfirmOpen(true)
  }

  function confirmDelete() {
    if (deletingMapping) {
      deleteMutation.mutate(deletingMapping.id)
    }
  }

  const columns = createMappingColumns(onEdit, handleDelete, (mp) => toggleMutation.mutate(mp))

  if (mappings.length === 0 && !isLoading && !error) {
    return (
      <div className="space-y-4" data-testid="provider-mapping-empty-state">
        <p className="text-sm text-muted-foreground">{m['billing.no_provider_mappings']()}</p>
        <Button onClick={onAdd} data-testid="add-provider-mapping-button">
          <Plus className="mr-2 h-4 w-4" />
          {m['billing.add_payment_provider']()}
        </Button>
      </div>
    )
  }

  return (
    <div className="space-y-4" data-testid="provider-mapping-list">
      <div className="flex items-center justify-between">
        <h4 className="text-sm font-medium">{m['billing.payment_providers_heading']()}</h4>
        <Button onClick={onAdd} size="sm" data-testid="add-provider-mapping-button">
          <Plus className="mr-2 h-4 w-4" />
          {m['billing.add_provider_short']()}
        </Button>
      </div>
      <DataTable
        columns={columns}
        data={mappings}
        isLoading={isLoading}
        error={error ?? undefined}
        loadingMessage={m['billing.loading_providers_short']()}
        errorMessage={
          error ? m['billing.error_loading_providers']({ message: error.message }) : undefined
        }
        emptyMessage={m['billing.no_providers_configured_short']()}
        data-testid="provider-mapping-table"
      />

      <ConfirmDialog
        open={deleteConfirmOpen}
        onOpenChange={setDeleteConfirmOpen}
        title={m['billing.delete_mapping_title']()}
        description={m['billing.delete_mapping_description']({
          name: formatProviderName(deletingMapping?.paymentProvider ?? ''),
        })}
        onConfirm={confirmDelete}
        isPending={deleteMutation.isPending}
        confirmTestId="confirm-delete-mapping-button"
      />
    </div>
  )
}
