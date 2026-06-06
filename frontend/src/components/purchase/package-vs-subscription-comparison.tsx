import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Check, Info } from 'lucide-react'
import type { PointsPackageResponse } from '@/lib/api-generated'
import { formatPrice, apiPriceToDisplayPrice } from '@/lib/schemas/points-package-forms'
import { m } from '@/paraglide/messages'

// TODO: FE-D04b will restore subscription comparison using entitlement mapping data

interface PackageVsSubscriptionComparisonProps {
  packages: PointsPackageResponse[]
  onPackageClick?: (pkg: PointsPackageResponse) => void
}

export function PackageVsSubscriptionComparison({
  packages,
  onPackageClick,
}: PackageVsSubscriptionComparisonProps) {
  const getCheapestPackage = () => {
    if (packages.length === 0) return null
    return packages.reduce((cheapest, pkg) => {
      const cheapestDisplay = apiPriceToDisplayPrice(cheapest.price, cheapest.currency)
      const pkgDisplay = apiPriceToDisplayPrice(pkg.price, pkg.currency)
      return pkgDisplay < cheapestDisplay ? pkg : cheapest
    })
  }

  const calculatePointsPerDollar = (points: number, price: number) => {
    return points / price
  }

  const cheapestPackage = getCheapestPackage()

  return (
    <div className="space-y-6" data-testid="package-vs-subscription-comparison">
      {/* Info Banner */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Info className="h-5 w-5" />
            {m['points.comparison_recommend_info_title']()}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            {m['points.comparison_recommend_info_description']()}
          </p>
        </CardContent>
      </Card>

      {/* Comparison Table */}
      <div className="rounded-md border">
        <table className="w-full">
          <thead>
            <tr className="border-b bg-muted/50">
              <th className="p-4 text-left font-semibold">
                {m['points.comparison_col_feature']()}
              </th>
              <th className="p-4 text-center font-semibold">
                {m['points.comparison_col_package']()}
              </th>
              <th className="p-4 text-center font-semibold">
                {m['points.comparison_col_subscription']()}
              </th>
            </tr>
          </thead>
          <tbody>
            <tr className="border-b">
              <td className="p-4 font-medium">{m['points.comparison_row_payment_type']()}</td>
              <td className="p-4 text-center">
                <Badge variant="outline">{m['points.comparison_row_one_time']()}</Badge>
              </td>
              <td className="p-4 text-center">
                <Badge variant="outline">{m['points.comparison_row_recurring']()}</Badge>
              </td>
            </tr>
            <tr className="border-b">
              <td className="p-4 font-medium">{m['points.comparison_row_points_delivery']()}</td>
              <td className="p-4 text-center">
                <div className="flex items-center justify-center gap-2">
                  <Check className="h-4 w-4 text-green-600" />
                  <span>{m['points.comparison_row_immediate']()}</span>
                </div>
              </td>
              <td className="p-4 text-center">
                <div className="flex items-center justify-center gap-2">
                  <Check className="h-4 w-4 text-green-600" />
                  <span>{m['points.comparison_row_periodic']()}</span>
                </div>
              </td>
            </tr>
            <tr className="border-b">
              <td className="p-4 font-medium">{m['points.comparison_row_cost_flexibility']()}</td>
              <td className="p-4 text-center">
                <div className="flex items-center justify-center gap-2">
                  <Check className="h-4 w-4 text-green-600" />
                  <span>{m['points.comparison_row_pay_as_need']()}</span>
                </div>
              </td>
              <td className="p-4 text-center">
                <span>{m['points.comparison_row_fixed_commitment']()}</span>
              </td>
            </tr>
            <tr>
              <td className="p-4 font-medium">{m['points.comparison_row_starting_from']()}</td>
              <td className="p-4 text-center font-semibold">
                {cheapestPackage
                  ? formatPrice(cheapestPackage.price, cheapestPackage.currency)
                  : 'N/A'}
              </td>
              <td className="p-4 text-center font-semibold">-</td>
            </tr>
          </tbody>
        </table>
      </div>

      {/* Quick Comparison - Package Card */}
      {cheapestPackage && (
        <Card
          className="cursor-pointer transition-shadow hover:shadow-md"
          onClick={() => onPackageClick?.(cheapestPackage)}
          data-testid="cheapest-package-card"
        >
          <CardHeader>
            <CardTitle className="text-lg">{m['points.comparison_card_package_title']()}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="text-center">
              <div className="text-3xl font-bold">{cheapestPackage.points.toLocaleString()}</div>
              <div className="text-sm text-muted-foreground">
                {m['points.comparison_card_points']()}
              </div>
              <div className="mt-2 text-2xl font-semibold text-primary">
                {formatPrice(cheapestPackage.price, cheapestPackage.currency)}
              </div>
              <div className="text-xs text-muted-foreground">
                {m['points.comparison_card_one_time']()}
              </div>
            </div>
            <div className="rounded-md bg-muted p-3">
              <div className="text-sm font-medium">{m['points.comparison_card_value']()}</div>
              <div className="text-lg font-bold">
                {calculatePointsPerDollar(
                  cheapestPackage.points,
                  apiPriceToDisplayPrice(cheapestPackage.price, cheapestPackage.currency)
                ).toFixed(1)}{' '}
                pts/$
              </div>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  )
}
