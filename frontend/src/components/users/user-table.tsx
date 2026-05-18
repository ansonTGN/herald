import { type ColumnDef, flexRender, getCoreRowModel, useReactTable } from '@tanstack/react-table'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { UserTableSkeleton } from './user-table-skeleton'
import { UserTableError } from './user-table-error'
import { UserTableEmpty } from './user-table-empty'
import { USER_STATUS_LABELS, USER_STATUS_COLORS } from '@/lib/constants/user'
import type { UserResponse, RoleResponse } from '@/lib/api-generated'
import { Badge } from '@/components/ui/badge'

// Type for user data with roles included (from API that extends UserResponse)
interface UserWithRoles extends UserResponse {
  roles?: RoleResponse[]
}

interface UserTableProps {
  data?: UserWithRoles[]
  isLoading?: boolean
  error?: Error
  onEdit?: (user: UserWithRoles) => void
  onDelete?: (user: UserWithRoles) => void
  onManageRoles?: (user: UserWithRoles) => void
}

function createUserColumns(
  onEdit?: (user: UserWithRoles) => void,
  onDelete?: (user: UserWithRoles) => void,
  onManageRoles?: (user: UserWithRoles) => void
): ColumnDef<UserWithRoles>[] {
  return [
    {
      id: 'id',
      accessorKey: 'id',
      header: 'ID',
      cell: ({ row }) => (
        <div className="font-mono text-xs" data-testid={`user-table-${row.index}-id`}>
          {String(row.getValue('id')).slice(0, 8)}...
        </div>
      ),
    },
    {
      accessorKey: 'email',
      header: 'Email',
      cell: ({ row }) => row.getValue('email') || '-',
    },
    {
      accessorKey: 'nickname',
      header: 'Nickname',
      cell: ({ row }) => row.getValue('nickname') || '-',
    },
    {
      accessorKey: 'status',
      header: 'Status',
      cell: ({ row }) => {
        const status = row.getValue('status') as number
        return (
          <span
            className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ${USER_STATUS_COLORS[status]}`}
            data-testid={`user-table-status-${status}`}
          >
            {USER_STATUS_LABELS[status]}
          </span>
        )
      },
    },
    {
      accessorKey: 'createdAt',
      header: 'Created At',
      cell: ({ row }) => {
        const date = row.getValue('createdAt') as string
        return date ? new Date(date).toLocaleString() : '-'
      },
    },
    {
      id: 'roles',
      header: 'Roles',
      cell: ({ row }) => {
        const user = row.original
        const roles = user?.roles || []

        if (roles.length === 0) {
          return <span className="text-sm text-muted-foreground">No roles</span>
        }

        return (
          <div className="flex flex-wrap gap-1" data-testid={`user-table-${row.index}-roles`}>
            {roles.slice(0, 2).map((role: RoleResponse) => (
              <Badge key={role.id} variant="secondary" className="text-xs">
                {role.name}
              </Badge>
            ))}
            {roles.length > 2 && (
              <Badge variant="outline" className="text-xs">
                +{roles.length - 2}
              </Badge>
            )}
          </div>
        )
      },
    },
    {
      id: 'actions',
      header: 'Actions',
      cell: ({ row }) => (
        <div className="flex gap-2" data-testid={`user-table-${row.index}-actions`}>
          {onManageRoles && (
            <button
              onClick={() => onManageRoles(row.original)}
              className="text-sm text-purple-600 hover:text-purple-800"
              data-testid={`user-table-${row.index}-manage-roles-button`}
            >
              Roles
            </button>
          )}
          {onEdit && (
            <button
              onClick={() => onEdit(row.original)}
              className="text-sm text-blue-600 hover:text-blue-800"
              data-testid={`user-table-${row.index}-edit-button`}
            >
              Edit
            </button>
          )}
          {onDelete && (
            <button
              onClick={() => onDelete(row.original)}
              className="text-sm text-red-600 hover:text-red-800"
              data-testid={`user-table-${row.index}-delete-button`}
            >
              Delete
            </button>
          )}
        </div>
      ),
    },
  ]
}

export function UserTable({
  data,
  isLoading = false,
  error,
  onEdit,
  onDelete,
  onManageRoles,
}: UserTableProps) {
  const columns = createUserColumns(onEdit, onDelete, onManageRoles)

  const table = useReactTable({
    data: data ?? [],
    columns,
    getCoreRowModel: getCoreRowModel(),
  })

  if (isLoading) {
    return <UserTableSkeleton />
  }

  if (error) {
    return <UserTableError error={error} />
  }

  if (!data || data.length === 0) {
    return <UserTableEmpty onCreateUser={() => {}} />
  }

  return (
    <div className="rounded-md border">
      <Table data-testid="users-table">
        <TableHeader>
          {table.getHeaderGroups().map((headerGroup) => (
            <TableRow key={headerGroup.id}>
              {headerGroup.headers.map((header) => (
                <TableHead key={header.id}>
                  {header.isPlaceholder
                    ? null
                    : flexRender(header.column.columnDef.header, header.getContext())}
                </TableHead>
              ))}
            </TableRow>
          ))}
        </TableHeader>
        <TableBody>
          {table.getRowModel().rows?.length ? (
            table.getRowModel().rows.map((row) => (
              <TableRow
                key={row.id}
                data-state={row.getIsSelected() && 'selected'}
                data-testid={`user-row-${row.index}`}
              >
                {row.getVisibleCells().map((cell) => (
                  <TableCell key={cell.id}>
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </TableCell>
                ))}
              </TableRow>
            ))
          ) : (
            <TableRow>
              <TableCell colSpan={columns.length} className="h-24 text-center">
                No results.
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
    </div>
  )
}
