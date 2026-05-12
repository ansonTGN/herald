import { type PointsPackageResponse } from '@/lib/api-generated'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Check, Star } from 'lucide-react'
import { formatPrice, apiPriceToDisplayPrice } from '@/lib/schemas/points-package-forms'

interface PointsPackageSelectorProps {
  packages: PointsPackageResponse[]
  selectedPackageId: string | null
  onSelect: (packageId: string) => void
  disabled?: boolean
}

export function PointsPackageSelector({
  packages,
  selectedPackageId,
  onSelect,
  disabled = false,
}: PointsPackageSelectorProps) {
  if (packages.length === 0) {
    return (
      <div className="rounded-lg border border-dashed p-8 text-center text-muted-foreground">
        No points packages available for purchase
      </div>
    )
  }

  // Find the package with the most points for the "best value" badge
  const bestValuePackage = packages.reduce((max, pkg) => (pkg.points > max.points ? pkg : max))

  return (
    <div
      data-testid="points-packages-selector"
      className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3"
    >
      {packages.map((pkg) => {
        const isSelected = selectedPackageId === pkg.id
        const isBestValue = pkg.id === bestValuePackage.id
        const isAvailable = pkg.enabled

        return (
          <Card
            key={pkg.id}
            className={`relative transition-all ${
              isSelected
                ? 'border-primary ring-2 ring-primary'
                : 'border-muted-foreground/25 hover:border-muted-foreground/50'
            } ${!isAvailable ? 'opacity-50' : ''}`}
            data-testid={`points-package-card-${pkg.id}`}
            data-selected={isSelected ? true : undefined}
          >
            {isBestValue && (
              <div
                className="absolute -top-2 -right-2 rounded-full bg-primary px-3 py-1 text-xs font-bold text-primary-foreground"
                data-testid="points-package-best-value-badge"
              >
                <Star className="mr-1 inline h-3 w-3" />
                Best Value
              </div>
            )}

            <CardHeader>
              <CardTitle className="flex items-center justify-between">
                <span>{pkg.title}</span>
                {isSelected && (
                  <div className="flex h-6 w-6 items-center justify-center rounded-full bg-primary">
                    <Check
                      className="h-4 w-4 text-primary-foreground"
                      data-testid={`points-package-selected-${pkg.id}`}
                    />
                  </div>
                )}
              </CardTitle>
              {pkg.description && (
                <p className="text-sm text-muted-foreground">{pkg.description}</p>
              )}
            </CardHeader>

            <CardContent>
              <div className="space-y-4">
                <div className="text-center">
                  <div className="text-4xl font-bold text-primary">
                    {pkg.points.toLocaleString()}
                  </div>
                  <div className="text-sm text-muted-foreground">Points</div>
                </div>

                <div className="flex items-center justify-between border-t pt-4">
                  <div className="text-sm">
                    <div className="font-medium">Price</div>
                    <div className="text-lg font-bold">{formatPrice(pkg.price, pkg.currency)}</div>
                  </div>
                  <div className="text-right text-sm">
                    <div className="font-medium">Value</div>
                    <div className="text-muted-foreground">
                      {(
                        (apiPriceToDisplayPrice(pkg.price, pkg.currency) / pkg.points) *
                        100
                      ).toFixed(4)}{' '}
                      {pkg.currency} per 100 points
                    </div>
                  </div>
                </div>

                <Button
                  className="w-full"
                  variant={isSelected ? 'default' : 'outline'}
                  onClick={() => isAvailable && !disabled && onSelect(pkg.id)}
                  disabled={!isAvailable || disabled}
                  data-testid={`points-package-select-button-${pkg.id}`}
                >
                  {isSelected ? 'Selected' : isAvailable ? 'Select Package' : 'Unavailable'}
                </Button>

                {!isAvailable && (
                  <div className="text-center text-xs text-muted-foreground">
                    This package is currently unavailable
                  </div>
                )}
              </div>
            </CardContent>
          </Card>
        )
      })}
    </div>
  )
}
