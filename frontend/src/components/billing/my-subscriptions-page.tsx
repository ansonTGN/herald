import { useMemo, useState } from 'react'
import { Link } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { PageHeader } from '@/components/shared/page-header'
import { ClaimSubscriptionDialog } from './ClaimSubscriptionDialog'
import { UnclaimedSubscriptionBanner } from './UnclaimedSubscriptionBanner'
import { claimShopifySubscriptions } from '@/lib/api-generated'
import { clientAppsQueryOptions, userSubscriptionsQueryOptions } from '@/data/query-options'
import {
  getSubscriptionForClientApp,
  type ClientAppItem,
  type SubscriptionDetailResponse,
} from '@/lib/api-generated'
import type { ClaimSubscriptionForm } from '@/lib/schemas/billing-forms'
import { m } from '@/paraglide/messages'

interface MySubscriptionsPageProps {
  realmId: string
}

type SubscriptionWithClientApp = {
  clientApp: ClientAppItem
  subscription: SubscriptionDetailResponse
}

export function MySubscriptionsPage({ realmId }: MySubscriptionsPageProps) {
  const queryClient = useQueryClient()
  const [isClaimDialogOpen, setIsClaimDialogOpen] = useState(false)
  const [showBanner, setShowBanner] = useState(true)

  const { data: clientAppsResponse, isLoading: isLoadingApps } = useQuery(
    clientAppsQueryOptions(realmId, { page: 0, pageSize: 100 })
  )

  const clientApps = useMemo(() => clientAppsResponse?.items ?? [], [clientAppsResponse?.items])

  const clientAppIds = useMemo(
    () =>
      clientApps
        .map((app) => app.id)
        .sort()
        .join(','),
    [clientApps]
  )

  // Query for subscriptions
  const { data: subscriptions = [], isLoading: isLoadingSubscriptions } = useQuery({
    ...userSubscriptionsQueryOptions<SubscriptionWithClientApp[]>(
      realmId,
      clientAppIds,
      async () => {
        if (clientApps.length === 0) {
          return []
        }

        const results = await Promise.all(
          clientApps.map(async (clientApp) => {
            try {
              const response = await getSubscriptionForClientApp({
                path: { realmId, clientAppId: clientApp.id },
              })

              if (response.error || !response.data) {
                return null
              }

              return {
                clientApp,
                subscription: response.data,
              }
            } catch {
              return null
            }
          })
        )

        return results.filter((result): result is SubscriptionWithClientApp => result !== null)
      }
    ),
    enabled: clientApps.length > 0,
  })

  // Mutation for claiming subscriptions
  const claimMutation = useMutation({
    mutationFn: async (data: ClaimSubscriptionForm) => {
      const response = await claimShopifySubscriptions({
        path: { realmId },
        body: {
          shopifyCustomerId: data.shopifyCustomerId || null,
          contractId: data.contractId || null,
          orderId: null,
        },
      })

      if (response.error) {
        throw response.error
      }

      return response.data
    },
    onSuccess: (data) => {
      // Close dialog
      setIsClaimDialogOpen(false)

      // Invalidate and refetch subscriptions
      queryClient.invalidateQueries({
        queryKey: ['user-subscriptions', realmId, clientAppIds],
      })

      // Show success message
      const grantedCount = data.grantedSubscriptionIds?.length || 0
      const claimedCount = data.claimedSubscriptionIds?.length || 0

      if (grantedCount > 0) {
        alert(
          claimedCount > 1
            ? m['billing.my_subscriptions_claimed_success_with_points_plural']({
                claimedCount,
                grantedCount,
              })
            : m['billing.my_subscriptions_claimed_success_with_points']({
                claimedCount,
                grantedCount,
              })
        )
      } else {
        alert(m['billing.my_subscriptions_claimed_success']({ count: claimedCount }))
      }

      // Hide banner after successful claim
      setShowBanner(false)
    },
    onError: (error: { status?: number; message?: string }) => {
      // Handle error responses
      if (error?.status === 404) {
        alert(m['billing.my_subscriptions_no_subscription_found']())
      } else if (error?.status === 409) {
        alert(m['billing.my_subscriptions_already_claimed']())
      } else {
        alert(
          m['billing.my_subscriptions_claim_failed']({ message: error?.message || 'Unknown error' })
        )
      }
    },
  })

  const handleClaimSubmit = (data: ClaimSubscriptionForm) => {
    claimMutation.mutate(data)
  }

  const isLoading = isLoadingApps || isLoadingSubscriptions

  const getStatusBadge = (status: string) => {
    switch (status.toLowerCase()) {
      case 'active':
        return (
          <Badge className="bg-green-100 text-green-800">
            {m['billing.subscription_status_label_active']()}
          </Badge>
        )
      case 'past_due':
        return (
          <Badge variant="destructive">{m['billing.subscription_status_label_past_due']()}</Badge>
        )
      case 'canceled':
        return (
          <Badge variant="secondary">{m['billing.subscription_status_label_canceled']()}</Badge>
        )
      case 'scheduled_cancel':
        return (
          <Badge className="bg-yellow-100 text-yellow-800">
            {m['billing.subscription_status_label_scheduled_cancel']()}
          </Badge>
        )
      default:
        return <Badge variant="outline">{status}</Badge>
    }
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">{m['billing.my_subscriptions_loading']()}</div>
      </div>
    )
  }

  return (
    <div className="space-y-6" data-testid="my-subscriptions-page">
      <PageHeader title={m['billing.my_subscriptions_title']()} />

      {/* Unclaimed Subscription Banner */}
      {showBanner && (
        <UnclaimedSubscriptionBanner
          count={1}
          onClaimClick={() => setIsClaimDialogOpen(true)}
          onClose={() => setShowBanner(false)}
        />
      )}

      {/* Claim Subscription Dialog */}
      <ClaimSubscriptionDialog
        open={isClaimDialogOpen}
        onOpenChange={setIsClaimDialogOpen}
        onSubmit={handleClaimSubmit}
        isSubmitting={claimMutation.isPending}
      />

      {subscriptions.length === 0 ? (
        <Card className="border-dashed">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <div className="text-4xl mb-4">📦</div>
            <h3 className="text-lg font-semibold mb-2">
              {m['billing.my_subscriptions_empty_title']()}
            </h3>
            <p className="text-sm text-muted-foreground text-center">
              {m['billing.my_subscriptions_empty_description']()}
            </p>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3" data-testid="subscription-list">
          {subscriptions.map(({ clientApp, subscription }) => (
            <Card key={subscription.id}>
              <CardHeader>
                <div className="flex items-start justify-between">
                  <div>
                    <CardTitle className="text-lg">
                      {subscription.plan?.title ?? subscription.plan?.name ?? 'Subscription'}
                    </CardTitle>
                    <CardDescription className="mt-1">{clientApp.name}</CardDescription>
                  </div>
                  {getStatusBadge(subscription.status)}
                </div>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <div className="text-sm text-muted-foreground">
                    {m['billing.my_subscriptions_current_period']()}
                  </div>
                  <div className="text-sm font-medium">
                    {subscription.currentPeriodStart && subscription.currentPeriodEnd
                      ? `${new Date(subscription.currentPeriodStart).toLocaleDateString()} - ${new Date(subscription.currentPeriodEnd).toLocaleDateString()}`
                      : m['billing.my_subscriptions_not_available']()}
                  </div>
                </div>

                <div>
                  <div className="text-sm text-muted-foreground">
                    {m['billing.my_subscriptions_billing_period']()}
                  </div>
                  <div className="text-2xl font-bold">{subscription.billingPeriod}</div>
                </div>

                <Button asChild variant="outline" size="sm" className="w-full">
                  <Link
                    to="/$realmId/subscription/$subscriptionId/history"
                    params={{ realmId, subscriptionId: subscription.id }}
                  >
                    {m['billing.my_subscriptions_view_history']()}
                  </Link>
                </Button>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  )
}
