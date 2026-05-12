import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Check, X, Info } from 'lucide-react'
import type { PointsPackageResponse } from '@/lib/api-generated'
import type { PlanResponse } from '@/lib/api-generated'
import { formatPrice, apiPriceToDisplayPrice } from '@/lib/schemas/points-package-forms'

interface PackageVsSubscriptionComparisonProps {
  packages: PointsPackageResponse[]
  plans: PlanResponse[]
  onPackageClick?: (pkg: PointsPackageResponse) => void
  onPlanClick?: (plan: PlanResponse) => void
}

export function PackageVsSubscriptionComparison({
  packages,
  plans,
  onPackageClick,
  onPlanClick,
}: PackageVsSubscriptionComparisonProps) {
  const getCheapestPackage = () => {
    if (packages.length === 0) return null
    return packages.reduce((cheapest, pkg) => {
      const cheapestDisplay = apiPriceToDisplayPrice(cheapest.price, cheapest.currency)
      const pkgDisplay = apiPriceToDisplayPrice(pkg.price, pkg.currency)
      return pkgDisplay < cheapestDisplay ? pkg : cheapest
    })
  }

  const getCheapestPlan = () => {
    if (plans.length === 0) return null
    const activePlans = plans.filter((plan) => plan.active)
    if (activePlans.length === 0) return null
    return activePlans.reduce((cheapest, plan) => (plan.price < cheapest.price ? plan : cheapest))
  }

  const calculatePointsPerDollar = (points: number, price: number) => {
    return points / price
  }

  const cheapestPackage = getCheapestPackage()
  const cheapestPlan = getCheapestPlan()

  const getRecommendation = () => {
    if (!cheapestPackage || !cheapestPlan) {
      return {
        type: 'info' as const,
        title: 'Compare your options',
        description: 'Choose between one-time point purchases or recurring subscriptions',
      }
    }

    const packageValue = calculatePointsPerDollar(cheapestPackage.points, cheapestPackage.price)
    const planValue = calculatePointsPerDollar(
      // Assuming monthly plans grant points based on plan configs (this would need real data)
      cheapestPlan.price, // Using price as denominator for comparison
      cheapestPlan.price
    )

    if (packageValue > planValue * 1.2) {
      return {
        type: 'package' as const,
        title: 'Better Value: Points Package',
        description: `Get ${cheapestPackage.points.toLocaleString()} points for ${formatPrice(cheapestPackage.price, cheapestPackage.currency)} - great for one-time needs!`,
      }
    } else if (planValue > packageValue * 1.2) {
      return {
        type: 'subscription' as const,
        title: 'Better Value: Subscription',
        description: `Save more with recurring points delivery and premium features`,
      }
    } else {
      return {
        type: 'neutral' as const,
        title: 'Both Great Options',
        description: 'Choose based on your usage patterns and preferences',
      }
    }
  }

  const recommendation = getRecommendation()

  return (
    <div className="space-y-6" data-testid="package-vs-subscription-comparison">
      {/* Recommendation Banner */}
      <Card
        className={
          recommendation.type === 'package'
            ? 'border-green-500'
            : recommendation.type === 'subscription'
              ? 'border-blue-500'
              : ''
        }
      >
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Info className="h-5 w-5" />
            {recommendation.title}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">{recommendation.description}</p>
        </CardContent>
      </Card>

      {/* Comparison Table */}
      <div className="rounded-md border">
        <table className="w-full">
          <thead>
            <tr className="border-b bg-muted/50">
              <th className="p-4 text-left font-semibold">Feature</th>
              <th className="p-4 text-center font-semibold">Points Package</th>
              <th className="p-4 text-center font-semibold">Subscription</th>
            </tr>
          </thead>
          <tbody>
            <tr className="border-b">
              <td className="p-4 font-medium">Payment Type</td>
              <td className="p-4 text-center">
                <Badge variant="outline">One-time Payment</Badge>
              </td>
              <td className="p-4 text-center">
                <Badge variant="outline">Recurring Payment</Badge>
              </td>
            </tr>
            <tr className="border-b">
              <td className="p-4 font-medium">Points Delivery</td>
              <td className="p-4 text-center">
                <div className="flex items-center justify-center gap-2">
                  <Check className="h-4 w-4 text-green-600" />
                  <span>Immediate</span>
                </div>
              </td>
              <td className="p-4 text-center">
                <div className="flex items-center justify-center gap-2">
                  <Check className="h-4 w-4 text-green-600" />
                  <span>Periodic (Monthly/Weekly)</span>
                </div>
              </td>
            </tr>
            <tr className="border-b">
              <td className="p-4 font-medium">Cost Flexibility</td>
              <td className="p-4 text-center">
                <div className="flex items-center justify-center gap-2">
                  <Check className="h-4 w-4 text-green-600" />
                  <span>Pay as you need</span>
                </div>
              </td>
              <td className="p-4 text-center">
                <div className="flex items-center justify-center gap-2">
                  <X className="h-4 w-4 text-destructive" />
                  <span>Fixed commitment</span>
                </div>
              </td>
            </tr>
            <tr className="border-b">
              <td className="p-4 font-medium">Best For</td>
              <td className="p-4 text-center text-sm">
                Occasional users, one-time needs, try before committing
              </td>
              <td className="p-4 text-center text-sm">
                Active users, consistent usage, better long-term value
              </td>
            </tr>
            <tr className="border-b">
              <td className="p-4 font-medium">Cancellation</td>
              <td className="p-4 text-center">
                <div className="flex items-center justify-center gap-2">
                  <Check className="h-4 w-4 text-green-600" />
                  <span>Not applicable</span>
                </div>
              </td>
              <td className="p-4 text-center">
                <div className="flex items-center justify-center gap-2">
                  <Check className="h-4 w-4 text-green-600" />
                  <span>Cancel anytime</span>
                </div>
              </td>
            </tr>
            <tr>
              <td className="p-4 font-medium">Starting From</td>
              <td className="p-4 text-center font-semibold">
                {cheapestPackage
                  ? formatPrice(cheapestPackage.price, cheapestPackage.currency)
                  : 'N/A'}
              </td>
              <td className="p-4 text-center font-semibold">
                {cheapestPlan ? `$${cheapestPlan.price.toFixed(2)}/mo` : 'N/A'}
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      {/* Quick Comparison */}
      <div className="grid gap-4 md:grid-cols-2">
        {/* Points Package Card */}
        <Card
          className="cursor-pointer transition-shadow hover:shadow-md"
          onClick={() => cheapestPackage && onPackageClick?.(cheapestPackage)}
          data-testid="cheapest-package-card"
        >
          <CardHeader>
            <CardTitle className="text-lg">Points Package</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {cheapestPackage ? (
              <>
                <div className="text-center">
                  <div className="text-3xl font-bold">
                    {cheapestPackage.points.toLocaleString()}
                  </div>
                  <div className="text-sm text-muted-foreground">points</div>
                  <div className="mt-2 text-2xl font-semibold text-primary">
                    {formatPrice(cheapestPackage.price, cheapestPackage.currency)}
                  </div>
                  <div className="text-xs text-muted-foreground">one-time payment</div>
                </div>
                <div className="rounded-md bg-muted p-3">
                  <div className="text-sm font-medium">Value:</div>
                  <div className="text-lg font-bold">
                    {calculatePointsPerDollar(
                      cheapestPackage.points,
                      apiPriceToDisplayPrice(cheapestPackage.price, cheapestPackage.currency)
                    ).toFixed(1)}{' '}
                    points/$
                  </div>
                </div>
              </>
            ) : (
              <div className="text-center text-sm text-muted-foreground">No packages available</div>
            )}
          </CardContent>
        </Card>

        {/* Subscription Card */}
        <Card
          className="cursor-pointer transition-shadow hover:shadow-md"
          onClick={() => cheapestPlan && onPlanClick?.(cheapestPlan)}
          data-testid="cheapest-plan-card"
        >
          <CardHeader>
            <CardTitle className="text-lg">Subscription</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {cheapestPlan ? (
              <>
                <div className="text-center">
                  <div className="text-3xl font-bold">Recurring</div>
                  <div className="text-sm text-muted-foreground">points delivery</div>
                  <div className="mt-2 text-2xl font-semibold text-primary">
                    ${cheapestPlan.price.toFixed(2)}
                  </div>
                  <div className="text-xs text-muted-foreground">per month</div>
                </div>
                <div className="rounded-md bg-muted p-3">
                  <div className="text-sm font-medium">Features:</div>
                  <div className="mt-1 space-y-1 text-sm">
                    <div className="flex items-center gap-2">
                      <Check className="h-3 w-3 text-green-600" />
                      <span>Automatic renewal</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <Check className="h-3 w-3 text-green-600" />
                      <span>Periodic grants</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <Check className="h-3 w-3 text-green-600" />
                      <span>Premium features</span>
                    </div>
                  </div>
                </div>
              </>
            ) : (
              <div className="text-center text-sm text-muted-foreground">No plans available</div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
