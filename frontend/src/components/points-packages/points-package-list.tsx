import { type PointsPackageResponse } from '@/lib/api-generated'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Pencil, Trash2, CreditCard } from 'lucide-react'
import { Skeleton } from '@/components/ui/skeleton'
import { AlertCircle } from 'lucide-react'
import { formatPrice } from '@/lib/schemas/points-package-forms'

interface PointsPackageListProps {
  data: PointsPackageResponse[] | { packages: PointsPackageResponse[] }
  isLoading: boolean
  error?: Error
  onEdit: (pkg: PointsPackageResponse) => void
  onDelete: (pkg: PointsPackageResponse) => void
  onConfigureProviders: (pkg: PointsPackageResponse) => void
}

function normalizePackagesData(
  data: PointsPackageResponse[] | { packages: PointsPackageResponse[] }
): PointsPackageResponse[] {
  if (Array.isArray(data)) {
    return data
  }
  return data.packages ?? []
}

export function PointsPackageList({
  data,
  isLoading,
  error,
  onEdit,
  onDelete,
  onConfigureProviders,
}: PointsPackageListProps) {
  const packages = normalizePackagesData(data)

  if (isLoading) {
    return (
      <div data-testid="points-packages-loading-skeleton" className="space-y-4">
        {[...Array(5)].map((_, i) => (
          <div key={i} className="flex items-center space-x-4">
            <Skeleton className="h-12 w-full" />
          </div>
        ))}
      </div>
    )
  }

  if (error) {
    return (
      <div
        data-testid="points-packages-error"
        className="flex flex-col items-center justify-center py-12 text-center"
      >
        <AlertCircle className="mb-4 h-12 w-12 text-destructive" />
        <h3 className="text-lg font-semibold">Failed to load points packages</h3>
        <p className="text-sm text-muted-foreground">{error.message}</p>
      </div>
    )
  }

  if (!packages || packages.length === 0) {
    return (
      <div
        data-testid="points-packages-empty-state"
        className="flex flex-col items-center justify-center py-12 text-center"
      >
        <div className="mb-4 rounded-full bg-muted p-4">
          <CreditCard className="h-8 w-8 text-muted-foreground" />
        </div>
        <h3 className="text-lg font-semibold">No points packages yet</h3>
        <p className="text-sm text-muted-foreground">
          Create your first points package to get started
        </p>
      </div>
    )
  }

  return (
    <div data-testid="points-packages-table" className="rounded-md border">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Name</TableHead>
            <TableHead>Title</TableHead>
            <TableHead className="text-right">Points</TableHead>
            <TableHead className="text-right">Price</TableHead>
            <TableHead>Type</TableHead>
            <TableHead className="text-right">Original Price</TableHead>
            <TableHead>Valid Until</TableHead>
            <TableHead>Status</TableHead>
            <TableHead className="text-right">Sort Order</TableHead>
            <TableHead className="text-right">Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {packages.map((pkg) => (
            <TableRow key={pkg.id} className={pkg.isExpired ? 'opacity-50' : ''}>
              <TableCell className="font-medium">{pkg.name}</TableCell>
              <TableCell>{pkg.title}</TableCell>
              <TableCell className="text-right">{pkg.points.toLocaleString()}</TableCell>
              <TableCell className="text-right">{formatPrice(pkg.price, pkg.currency)}</TableCell>
              <TableCell>
                <div className="flex items-center gap-1">
                  {pkg.packageType === 'promotional' ? (
                    <Badge>Promotional</Badge>
                  ) : (
                    <Badge variant="secondary">Standard</Badge>
                  )}
                  {pkg.isExpired && <Badge variant="destructive">Expired</Badge>}
                </div>
              </TableCell>
              <TableCell className="text-right">
                {pkg.originalPrice != null ? (
                  <span className="line-through text-muted-foreground">
                    {formatPrice(pkg.originalPrice, pkg.currency)}
                  </span>
                ) : (
                  <span className="text-muted-foreground">—</span>
                )}
              </TableCell>
              <TableCell>
                {pkg.promoEndTime ? (
                  new Date(pkg.promoEndTime).toLocaleDateString()
                ) : (
                  <span className="text-muted-foreground">—</span>
                )}
              </TableCell>
              <TableCell>
                {pkg.enabled ? (
                  <Badge variant="default">Enabled</Badge>
                ) : (
                  <Badge variant="secondary">Disabled</Badge>
                )}
              </TableCell>
              <TableCell className="text-right">{pkg.sortOrder}</TableCell>
              <TableCell className="text-right">
                <div className="flex justify-end gap-2">
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => onEdit(pkg)}
                    data-testid={`points-package-edit-button-${pkg.id}`}
                    title="Edit package"
                  >
                    <Pencil className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => onConfigureProviders(pkg)}
                    data-testid={`points-package-configure-button-${pkg.id}`}
                    title="Configure payment providers"
                  >
                    <CreditCard className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => onDelete(pkg)}
                    data-testid={`points-package-delete-button-${pkg.id}`}
                    title="Delete package"
                  >
                    <Trash2 className="h-4 w-4 text-destructive" />
                  </Button>
                </div>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}
