import { type ColumnDef } from '@tanstack/react-table'
import { type ProductResponse } from '@/lib/api-generated'
import { Badge } from '@/components/ui/badge'
import { MoreHorizontal, Edit, Trash2 } from 'lucide-react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Button } from '@/components/ui/button'
import { DataTable } from '@/components/shared/data-table'
import { m } from '@/paraglide/messages'

interface ProductTableProps {
  data?: ProductResponse[]
  isLoading?: boolean
  error?: Error
  onEdit?: (product: ProductResponse) => void
  onDelete?: (product: ProductResponse) => void
}

function createProductColumns(
  onEdit?: (product: ProductResponse) => void,
  onDelete?: (product: ProductResponse) => void
): ColumnDef<ProductResponse>[] {
  return [
    {
      accessorKey: 'code',
      header: m['billing.col_code'](),
      cell: ({ row }) => (
        <div className="font-medium" data-testid={`product-code-${row.index}`}>
          {(row.getValue('code') as string) || ''}
        </div>
      ),
    },
    {
      accessorKey: 'title',
      header: m['billing.col_title'](),
      cell: ({ row }) => row.getValue('title'),
    },
    {
      accessorKey: 'description',
      header: m['common.description'](),
      cell: ({ row }) => {
        const desc = row.getValue('description') as string | null | undefined
        if (!desc) return <span className="text-muted-foreground">&mdash;</span>
        return (
          <span className="max-w-[200px] truncate block">
            {desc.length > 50 ? `${desc.slice(0, 50)}...` : desc}
          </span>
        )
      },
    },
    {
      accessorKey: 'plansCount',
      header: m['billing.col_plans'](),
      cell: ({ row }) => (row.getValue('plansCount') as number).toString(),
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
            {onEdit && (
              <DropdownMenuItem
                onClick={() => onEdit(row.original)}
                data-testid={`edit-product-button-${row.original.id}`}
              >
                <Edit className="mr-2 h-4 w-4" />
                {m['common.edit']()}
              </DropdownMenuItem>
            )}
            {onDelete && (
              <DropdownMenuItem
                onClick={() => onDelete(row.original)}
                className="text-destructive"
                data-testid={`delete-product-button-${row.original.id}`}
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

export function ProductTable({
  data = [],
  isLoading = false,
  error,
  onEdit,
  onDelete,
}: ProductTableProps) {
  const columns = createProductColumns(onEdit, onDelete)

  return (
    <DataTable
      columns={columns}
      data={data}
      isLoading={isLoading}
      error={error}
      loadingMessage={m['billing.loading_products']()}
      errorMessage={
        error ? m['billing.error_loading_products']({ message: error.message }) : undefined
      }
      emptyMessage={m['billing.no_products']()}
      data-testid="product-table"
    />
  )
}
