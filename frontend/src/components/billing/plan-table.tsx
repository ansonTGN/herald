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
import { m } from '@/paraglide/messages'

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
      header: m['billing.col_id'](),
      cell: ({ row }) => (
        <div className="font-mono text-xs" data-testid={`plan-id-${row.index}`}>
          {String(row.getValue('id')).slice(0, 8)}...
        </div>
      ),
    },
    {
      accessorKey: 'name',
      header: m['billing.col_plan_name'](),
      cell: ({ row }) => (
        <div className="font-medium" data-testid={`plan-name-${row.index}`}>
          {(row.getValue('name') as string) || ''}
        </div>
      ),
    },
    {
      accessorKey: 'title',
      header: m['billing.col_title'](),
      cell: ({ row }) => row.getValue('title'),
    },
    {
      accessorKey: 'type',
      header: m['billing.col_billing'](),
      cell: ({ row }) => {
        const type = row.getValue('type') as string
        return (
          <Badge variant={type === 'monthly' ? 'default' : 'secondary'}>
            {type === 'monthly'
              ? m['billing.billing_monthly']()
              : type === 'yearly'
                ? m['billing.billing_yearly']()
                : type}
          </Badge>
        )
      },
    },
    {
      accessorKey: 'price',
      header: m['billing.col_price'](),
      cell: ({ row }) => {
        const price = row.getValue('price') as number
        const currency = row.original.currency || 'USD'
        return `$${(price / 100).toFixed(2)} ${currency}`
      },
    },
    {
      accessorKey: 'paymentProviders',
      header: m['billing.col_payment_providers'](),
      cell: ({ row }) => {
        const providers = row.getValue('paymentProviders') as
          | Array<PaymentProviderSummary>
          | undefined

        if (!providers || providers.length === 0) {
          return <Badge variant="destructive">{m['billing.not_configured']()}</Badge>
        }

        const enabledProviders = getEnabledProviders(providers)

        if (enabledProviders.length === 0) {
          return <Badge variant="secondary">{m['billing.all_disabled']()}</Badge>
        }

        const names = formatProviderNames(enabledProviders)

        return <span className="text-sm">{names}</span>
      },
    },
    {
      accessorKey: 'trialDays',
      header: m['billing.col_trial_days'](),
      cell: ({ row }) => (row.getValue('trialDays') as number).toString(),
    },
    {
      accessorKey: 'active',
      header: m['billing.col_status'](),
      cell: ({ row }) => {
        const active = row.getValue('active') as boolean
        return (
          <Badge variant={active ? 'default' : 'secondary'}>
            {active ? m['billing.status_active']() : m['billing.status_disabled']()}
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
            {onEdit && (
              <DropdownMenuItem
                onClick={() => onEdit(row.original)}
                data-testid={`edit-plan-button-${row.original.id}`}
              >
                <Edit className="mr-2 h-4 w-4" />
                {m['billing.edit_plan']()}
              </DropdownMenuItem>
            )}
            {onManageProviders && (
              <DropdownMenuItem
                onClick={() => onManageProviders(row.original)}
                data-testid={`manage-providers-button-${row.original.id}`}
              >
                <Settings className="mr-2 h-4 w-4" />
                {m['billing.manage_providers']()}
              </DropdownMenuItem>
            )}
            {onAssign && (
              <DropdownMenuItem
                onClick={() => onAssign(row.original)}
                data-testid={`assign-plan-button-${row.original.id}`}
              >
                <Users className="mr-2 h-4 w-4" />
                {m['billing.assign_to_app']()}
              </DropdownMenuItem>
            )}
            {onDelete && (
              <DropdownMenuItem
                onClick={() => onDelete(row.original)}
                className="text-destructive"
                data-testid={`delete-plan-button-${row.original.id}`}
              >
                <Trash2 className="mr-2 h-4 w-4" />
                {m['common.delete']()}
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
      loadingMessage={m['billing.loading_plans']()}
      errorMessage={
        error ? m['billing.error_loading_plans']({ message: error.message }) : undefined
      }
      emptyMessage={m['billing.no_plans_found']()}
      data-testid="plans-table"
    />
  )
}
