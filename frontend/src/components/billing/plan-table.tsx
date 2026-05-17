import { type ColumnDef } from '@tanstack/react-table'
import { type SubscriptionPlanResponse, type PaymentProviderSummary } from '@/lib/api-generated'
import { Badge } from '@/components/ui/badge'
import { MoreHorizontal, Edit, Trash2, Users, Settings } from 'lucide-react'
import { getEnabledProviders, formatProviderNames } from '@/lib/billing-utils'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Button } from '@/components/ui/button'
import { DataTable } from '@/components/shared/data-table'

interface PlanTableProps {
  data?: SubscriptionPlanResponse[]
  isLoading?: boolean
  error?: Error
  onEdit?: (plan: SubscriptionPlanResponse) => void
  onDelete?: (plan: SubscriptionPlanResponse) => void
  onAssign?: (plan: SubscriptionPlanResponse) => void
  onManageProviders?: (plan: SubscriptionPlanResponse) => void
}

function createPlanColumns(
  onEdit?: (plan: SubscriptionPlanResponse) => void,
  onDelete?: (plan: SubscriptionPlanResponse) => void,
  onAssign?: (plan: SubscriptionPlanResponse) => void,
  onManageProviders?: (plan: SubscriptionPlanResponse) => void
): ColumnDef<SubscriptionPlanResponse>[] {
  return [
    {
      id: 'id',
      accessorKey: 'id',
      header: 'ID',
      cell: ({ row }) => (
        <div className="font-mono text-xs" data-testid={`plan-id-${row.index}`}>
          {String(row.getValue('id')).slice(0, 8)}...
        </div>
      ),
    },
    {
      accessorKey: 'name',
      header: 'Subscription Plan Name',
      cell: ({ row }) => (
        <div className="font-medium" data-testid={`plan-name-${row.index}`}>
          {(row.getValue('name') as string) || ''}
        </div>
      ),
    },
    {
      accessorKey: 'title',
      header: 'Title',
      cell: ({ row }) => row.getValue('title'),
    },
    {
      accessorKey: 'type',
      header: 'Billing',
      cell: ({ row }) => {
        const type = row.getValue('type') as string
        return (
          <Badge variant={type === 'monthly' ? 'default' : 'secondary'}>{type || 'Unknown'}</Badge>
        )
      },
    },
    {
      accessorKey: 'price',
      header: 'Price',
      cell: ({ row }) => {
        const price = row.getValue('price') as number
        const currency = row.original.currency || 'USD'
        return `$${(price / 100).toFixed(2)} ${currency}`
      },
    },
    {
      accessorKey: 'paymentProviders',
      header: 'Payment Providers',
      cell: ({ row }) => {
        const providers = row.getValue('paymentProviders') as
          | Array<PaymentProviderSummary>
          | undefined

        if (!providers || providers.length === 0) {
          return <Badge variant="destructive">Not configured</Badge>
        }

        const enabledProviders = getEnabledProviders(providers)

        if (enabledProviders.length === 0) {
          return <Badge variant="secondary">All disabled</Badge>
        }

        const names = formatProviderNames(enabledProviders)

        return <span className="text-sm">{names}</span>
      },
    },
    {
      accessorKey: 'trialDays',
      header: 'Trial Days',
      cell: ({ row }) => (row.getValue('trialDays') as number).toString(),
    },
    {
      accessorKey: 'active',
      header: 'Status',
      cell: ({ row }) => {
        const active = row.getValue('active') as boolean
        return (
          <Badge variant={active ? 'default' : 'secondary'}>{active ? 'Active' : 'Disabled'}</Badge>
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
            {onEdit && (
              <DropdownMenuItem
                onClick={() => onEdit(row.original)}
                data-testid={`edit-plan-button-${row.original.id}`}
              >
                <Edit className="mr-2 h-4 w-4" />
                Edit
              </DropdownMenuItem>
            )}
            {onManageProviders && (
              <DropdownMenuItem
                onClick={() => onManageProviders(row.original)}
                data-testid={`manage-providers-button-${row.original.id}`}
              >
                <Settings className="mr-2 h-4 w-4" />
                Manage Providers
              </DropdownMenuItem>
            )}
            {onAssign && (
              <DropdownMenuItem
                onClick={() => onAssign(row.original)}
                data-testid={`assign-plan-button-${row.original.id}`}
              >
                <Users className="mr-2 h-4 w-4" />
                Assign to App
              </DropdownMenuItem>
            )}
            {onDelete && (
              <DropdownMenuItem
                onClick={() => onDelete(row.original)}
                className="text-destructive"
                data-testid={`delete-plan-button-${row.original.id}`}
              >
                <Trash2 className="mr-2 h-4 w-4" />
                Delete
              </DropdownMenuItem>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      ),
    },
  ]
}

export function PlanTable({
  data = [],
  isLoading = false,
  error,
  onEdit,
  onDelete,
  onAssign,
  onManageProviders,
}: PlanTableProps) {
  const columns = createPlanColumns(onEdit, onDelete, onAssign, onManageProviders)

  return (
    <DataTable
      columns={columns}
      data={data}
      isLoading={isLoading}
      error={error}
      loadingMessage="Loading subscription plans..."
      errorMessage={error ? `Error loading subscription plans: ${error.message}` : undefined}
      emptyMessage="No subscription plans found."
      data-testid="plans-table"
    />
  )
}
