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
      accessorKey: 'name',
      header: 'Name',
      cell: ({ row }) => (
        <div className="font-medium" data-testid={`product-name-${row.index}`}>
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
      accessorKey: 'description',
      header: 'Description',
      cell: ({ row }) => {
        const desc = row.getValue('description') as string | null | undefined
        if (!desc) return <span className="text-muted-foreground">—</span>
        return (
          <span className="max-w-[200px] truncate block">
            {desc.length > 50 ? `${desc.slice(0, 50)}...` : desc}
          </span>
        )
      },
    },
    {
      accessorKey: 'sortOrder',
      header: 'Sort Order',
      cell: ({ row }) => (row.getValue('sortOrder') as number).toString(),
    },
    {
      accessorKey: 'plansCount',
      header: 'Plans',
      cell: ({ row }) => (row.getValue('plansCount') as number).toString(),
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
            {onEdit && (
              <DropdownMenuItem
                onClick={() => onEdit(row.original)}
                data-testid={`edit-product-button-${row.original.id}`}
              >
                <Edit className="mr-2 h-4 w-4" />
                Edit
              </DropdownMenuItem>
            )}
            {onDelete && (
              <DropdownMenuItem
                onClick={() => onDelete(row.original)}
                className="text-destructive"
                data-testid={`delete-product-button-${row.original.id}`}
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
      loadingMessage="Loading products..."
      errorMessage={error ? `Error loading products: ${error.message}` : undefined}
      emptyMessage="No products found. Create your first product."
      data-testid="product-table"
    />
  )
}
