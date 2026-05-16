import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { cancelSubscriptionForClientApp } from '@/lib/api-generated'
import { queryKeys, subscriptionQueryOptions } from '@/data/query-options'
import { toast } from 'sonner'
import { getStatusBadgeVariant, getStatusMessage, type SubscriptionStatus } from '@/types/billing'
import { formatDate } from '@/lib/date-utils'
import { ConfirmDeleteDialog, PageHeader } from '@/components/shared'

interface SubscriptionManagementProps {
  realmId: string
  clientAppId: string
}

export function SubscriptionManagement({ realmId, clientAppId }: SubscriptionManagementProps) {
  const queryClient = useQueryClient()
  const { data: subscription, isLoading } = useQuery(subscriptionQueryOptions(realmId, clientAppId))
  const [cancelConfirmOpen, setCancelConfirmOpen] = useState(false)

  // Cancel subscription mutation
  const cancelSubscriptionMutation = useMutation({
    mutationFn: async () => {
      const response = await cancelSubscriptionForClientApp({
        path: { realmId, clientAppId },
        body: { cancelAtPeriodEnd: true },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      toast.success('Subscription canceled successfully')
      setCancelConfirmOpen(false)
      queryClient.invalidateQueries({ queryKey: queryKeys.subscription(realmId, clientAppId) })
    },
    onError: (error: Error) => {
      toast.error(`Failed to cancel subscription: ${error.message}`)
    },
  })

  function handleCancelSubscription() {
    setCancelConfirmOpen(true)
  }

  async function confirmCancelSubscription() {
    await cancelSubscriptionMutation.mutateAsync()
  }

  if (isLoading) {
    return <div>Loading...</div>
  }

  if (!subscription) {
    return (
      <Card>
        <CardContent className="py-8 text-center text-muted-foreground">
          No subscription found for this app.
        </CardContent>
      </Card>
    )
  }

  return (
    <div className="space-y-6" data-testid="subscription-management">
      <PageHeader title="Subscription" />

      <Card>
        <CardHeader>
          <CardTitle>Subscription Details</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">Status</span>
            <Badge
              variant={getStatusBadgeVariant(subscription.status as SubscriptionStatus)}
              data-testid="subscription-status-badge"
            >
              {subscription.status}
            </Badge>
          </div>
          {getStatusMessage(subscription.status as SubscriptionStatus) && (
            <div className="flex items-center justify-between">
              <span></span>
              <div className="text-sm text-muted-foreground">
                {getStatusMessage(subscription.status as SubscriptionStatus)}
              </div>
            </div>
          )}

          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">Plan</span>
            <span className="text-sm">{subscription.plan?.title || 'None'}</span>
          </div>

          {subscription.currentPeriodStart && (
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">Current Period Start</span>
              <span className="text-sm">{formatDate(subscription.currentPeriodStart)}</span>
            </div>
          )}

          {subscription.currentPeriodEnd && (
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">Current Period End</span>
              <span className="text-sm">{formatDate(subscription.currentPeriodEnd)}</span>
            </div>
          )}

          {subscription.cancelAt && (
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">Cancel At</span>
              <span className="text-sm">{formatDate(subscription.cancelAt)}</span>
            </div>
          )}

          {subscription.status === 'active' && (
            <Button
              variant="destructive"
              onClick={handleCancelSubscription}
              disabled={cancelSubscriptionMutation.isPending}
              data-testid="cancel-subscription-button"
            >
              {cancelSubscriptionMutation.isPending ? 'Canceling...' : 'Cancel Subscription'}
            </Button>
          )}
        </CardContent>
      </Card>

      {/* Cancel Confirmation Dialog */}
      <ConfirmDeleteDialog
        open={cancelConfirmOpen}
        onOpenChange={setCancelConfirmOpen}
        title="Cancel Subscription"
        description="Are you sure you want to cancel this subscription?"
        onConfirm={confirmCancelSubscription}
        confirmLabel="Cancel Subscription"
        isPending={cancelSubscriptionMutation.isPending}
      />
    </div>
  )
}
