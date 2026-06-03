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
      header: 'Payment Provider',
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
      header: 'External Product ID',
      cell: ({ row }) => (
        <span className="font-mono text-xs" data-testid={`mapping-product-id-${row.original.id}`}>
          {row.getValue('externalProductId')}
        </span>
      ),
    },
    {
      accessorKey: 'externalPriceId',
      header: 'External Price ID',
      cell: ({ row }) => (
        <span className="font-mono text-xs text-muted-foreground">
          {(row.getValue('externalPriceId') as string | null) || '-'}
        </span>
      ),
    },
    {
      accessorKey: 'enabled',
      header: 'Status',
      cell: ({ row }) => {
        const enabled = row.getValue('enabled') as boolean
        return (
          <Badge variant={enabled ? 'default' : 'secondary'}>
            {enabled ? 'Enabled' : 'Disabled'}
          </Badge>
        )
      },
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
              data-testid={`edit-mapping-button-${row.original.id}`}
            >
              <Edit className="mr-2 h-4 w-4" />
              Edit
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => onToggle(row.original)}
              data-testid={`toggle-mapping-button-${row.original.id}`}
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
              data-testid={`delete-mapping-button-${row.original.id}`}
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
      toast.success('Payment provider mapping deleted')
      setDeleteConfirmOpen(false)
      setDeletingMapping(null)
      // Invalidate queries to refresh the mapping list
      queryClient.invalidateQueries({
        queryKey: subscriptionPlanProvidersQueryOptions(realmId, planId).queryKey,
      })
      queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })
    },
    onError: (err: Error) => {
      toast.error(`Failed to delete mapping: ${err.message}`)
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
      const action = variables.enabled ? 'disabled' : 'enabled'
      toast.success(`Payment provider mapping ${action}`)
      // Invalidate queries to refresh the mapping list
      queryClient.invalidateQueries({
        queryKey: subscriptionPlanProvidersQueryOptions(realmId, planId).queryKey,
      })
      queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })
    },
    onError: (err: Error) => {
      toast.error(`Failed to toggle mapping: ${err.message}`)
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

  const columns = createMappingColumns(onEdit, handleDelete, (m) => toggleMutation.mutate(m))

  if (mappings.length === 0 && !isLoading && !error) {
    return (
      <div className="space-y-4" data-testid="provider-mapping-empty-state">
        <p className="text-sm text-muted-foreground">
          This plan has no payment providers configured. Add a payment provider to make it available
          for subscription.
        </p>
        <Button onClick={onAdd} data-testid="add-provider-mapping-button">
          <Plus className="mr-2 h-4 w-4" />
          Add Payment Provider
        </Button>
      </div>
    )
  }

  return (
    <div className="space-y-4" data-testid="provider-mapping-list">
      <div className="flex items-center justify-between">
        <h4 className="text-sm font-medium">Payment Providers</h4>
        <Button onClick={onAdd} size="sm" data-testid="add-provider-mapping-button">
          <Plus className="mr-2 h-4 w-4" />
          Add Provider
        </Button>
      </div>
      <DataTable
        columns={columns}
        data={mappings}
        isLoading={isLoading}
        error={error ?? undefined}
        loadingMessage="Loading payment providers..."
        errorMessage={error ? `Error loading providers: ${error.message}` : undefined}
        emptyMessage="No payment providers configured."
        data-testid="provider-mapping-table"
      />

      <ConfirmDialog
        open={deleteConfirmOpen}
        onOpenChange={setDeleteConfirmOpen}
        title="Delete Payment Provider Mapping"
        description={`Are you sure you want to delete the ${formatProviderName(deletingMapping?.paymentProvider ?? '')} mapping?`}
        onConfirm={confirmDelete}
        isPending={deleteMutation.isPending}
        confirmTestId="confirm-delete-mapping-button"
      />
    </div>
  )
}
