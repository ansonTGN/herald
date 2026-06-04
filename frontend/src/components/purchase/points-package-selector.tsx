import { m } from '@/paraglide/messages'
import { type ExtPointsPackageItem } from '@/lib/api-generated'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Check, Star } from 'lucide-react'
import { formatPrice, apiPriceToDisplayPrice } from '@/lib/schemas/points-package-forms'

interface PointsPackageSelectorProps {
  packages: ExtPointsPackageItem[]
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
        {m['points.selector_no_packages']()}
      </div>
    )
  }

  // Find the non-promotional package with the most points for the "best value" badge
  const nonPromoPackages = packages.filter((pkg) => pkg.packageType !== 'promotional')
  const bestValuePackage =
    nonPromoPackages.length > 0
      ? nonPromoPackages.reduce((max, pkg) => (pkg.points > max.points ? pkg : max))
      : null

  return (
    <div
      data-testid="points-packages-selector"
      className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3"
    >
      {packages.map((pkg) => {
        const isSelected = selectedPackageId === pkg.id
        const isBestValue = bestValuePackage != null && pkg.id === bestValuePackage.id
        const isPromo = pkg.packageType === 'promotional'

        return (
          <Card
            key={pkg.id}
            className={`relative transition-all ${
              isSelected
                ? 'border-primary ring-2 ring-primary'
                : 'border-muted-foreground/25 hover:border-muted-foreground/50'
            }`}
            data-testid={`points-package-card-${pkg.id}`}
            data-selected={isSelected ? true : undefined}
          >
            {isPromo && pkg.discountPercent != null ? (
              <Badge
                variant="destructive"
                className="absolute -top-2 -right-2 rounded-full px-3 py-1 text-xs font-bold"
                data-testid="points-package-discount-badge"
              >
                -{pkg.discountPercent}%
              </Badge>
            ) : isBestValue ? (
              <div
                className="absolute -top-2 -right-2 rounded-full bg-primary px-3 py-1 text-xs font-bold text-primary-foreground"
                data-testid="points-package-best-value-badge"
              >
                <Star className="mr-1 inline h-3 w-3" />
                {m['points.selector_best_value']()}
              </div>
            ) : null}

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
                  <div className="text-sm text-muted-foreground">
                    {m['points.selector_points']()}
                  </div>
                </div>

                <div className="flex items-center justify-between border-t pt-4">
                  <div className="text-sm">
                    <div className="font-medium">{m['points.selector_price']()}</div>
                    {pkg.originalPrice != null && (
                      <div className="text-sm text-muted-foreground line-through">
                        {formatPrice(pkg.originalPrice, pkg.currency)}
                      </div>
                    )}
                    <div className="text-lg font-bold">{formatPrice(pkg.price, pkg.currency)}</div>
                  </div>
                  <div className="text-right text-sm">
                    <div className="font-medium">{m['points.selector_value']()}</div>
                    <div className="text-muted-foreground">
                      {(
                        (apiPriceToDisplayPrice(pkg.price, pkg.currency) / pkg.points) *
                        100
                      ).toFixed(4)}{' '}
                      {m['points.selector_per_100']({ currency: pkg.currency })}
                    </div>
                  </div>
                </div>

                {pkg.promoEndTime && (
                  <div
                    className="text-xs text-amber-600 font-medium"
                    data-testid="points-package-limited-time"
                  >
                    {(() => {
                      const daysLeft = Math.ceil(
                        (new Date(pkg.promoEndTime).getTime() - Date.now()) / (1000 * 60 * 60 * 24)
                      )
                      return daysLeft > 0
                        ? daysLeft <= 7
                          ? m['points.selector_days_left']({ days: daysLeft })
                          : m['points.selector_ends_date']({
                              date: new Date(pkg.promoEndTime).toLocaleDateString(),
                            })
                        : m['points.selector_ending_soon']()
                    })()}
                  </div>
                )}

                <Button
                  className="w-full"
                  variant={isSelected ? 'default' : 'outline'}
                  onClick={() => !disabled && onSelect(pkg.id)}
                  disabled={disabled}
                  data-testid={`points-package-select-button-${pkg.id}`}
                >
                  {isSelected ? m['points.selector_selected']() : m['points.selector_select']()}
                </Button>
              </div>
            </CardContent>
          </Card>
        )
      })}
    </div>
  )
}
