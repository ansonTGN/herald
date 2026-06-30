import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { cancelSubscriptionForClientApp } from '@/lib/api-generated'
import { queryKeys, subscriptionQueryOptions } from '@/data/query-options'
import { toast } from 'sonner'
import {
  getStatusBadgeVariant,
  getStatusMessage,
  getSubscriptionStatusLabels,
  type SubscriptionStatus,
} from '@/types/billing'
import { formatDate } from '@/lib/date-utils'
import { ConfirmDialog, PageHeader } from '@/components/shared'
import { m } from '@/paraglide/messages'

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
      toast.success(m['billing.subscription_canceled_success']())
      setCancelConfirmOpen(false)
      queryClient.invalidateQueries({ queryKey: queryKeys.subscription(realmId, clientAppId) })
    },
    onError: (error: Error) => {
      toast.error(m['billing.subscription_cancel_failed']({ message: error.message }))
    },
  })

  function handleCancelSubscription() {
    setCancelConfirmOpen(true)
  }

  async function confirmCancelSubscription() {
    // The mutation's `onError` surfaces the failure toast and logs the error.
    // Swallow the rejected promise so it doesn't propagate as an unhandled
    // rejection (per vitest config, rejections are expected to be handled in
    // components). Sibling forms share this latent leak; see FE-T06 handoff.
    await cancelSubscriptionMutation.mutateAsync().catch(() => {})
  }

  if (isLoading) {
    return <div>{m['billing.subscription_info_loading']()}</div>
  }

  if (!subscription) {
    return (
      <Card>
        <CardContent className="py-8 text-center text-muted-foreground">
          {m['billing.subscription_no_subscription']()}
        </CardContent>
      </Card>
    )
  }

  return (
    <div className="space-y-6" data-testid="subscription-management">
      <PageHeader title={m['billing.subscription_page_title']()} />

      <Card>
        <CardHeader>
          <CardTitle>{m['billing.subscription_details']()}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">{m['billing.subscription_status']()}</span>
            <Badge
              variant={getStatusBadgeVariant(subscription.status as SubscriptionStatus)}
              data-testid="subscription-status-badge"
            >
              {getSubscriptionStatusLabels()[subscription.status as SubscriptionStatus]}
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
            <span className="text-sm font-medium">
              {m['billing.subscription_entitlement_key']()}
            </span>
            <span className="text-sm">
              {subscription.entitlementKey || m['billing.subscription_none']()}
            </span>
          </div>

          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">
              {m['billing.subscription_payment_provider']()}
            </span>
            <span className="text-sm">
              {subscription.paymentProvider || m['billing.subscription_none']()}
            </span>
          </div>

          {subscription.syncedAt && (
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">{m['billing.subscription_synced_at']()}</span>
              <span className="text-sm">{formatDate(subscription.syncedAt)}</span>
            </div>
          )}

          {subscription.currentPeriodStart && (
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">
                {m['billing.subscription_period_start']()}
              </span>
              <span className="text-sm">{formatDate(subscription.currentPeriodStart)}</span>
            </div>
          )}

          {subscription.currentPeriodEnd && (
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">{m['billing.subscription_period_end']()}</span>
              <span className="text-sm">{formatDate(subscription.currentPeriodEnd)}</span>
            </div>
          )}

          {subscription.cancelAt && (
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">{m['billing.subscription_cancel_at']()}</span>
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
              {cancelSubscriptionMutation.isPending
                ? m['billing.subscription_canceling']()
                : m['billing.subscription_cancel_button']()}
            </Button>
          )}
        </CardContent>
      </Card>

      {/* Cancel Confirmation Dialog */}
      <ConfirmDialog
        open={cancelConfirmOpen}
        onOpenChange={setCancelConfirmOpen}
        title={m['billing.subscription_cancel_confirm_title']()}
        description={m['billing.subscription_cancel_confirm_description']()}
        onConfirm={confirmCancelSubscription}
        confirmLabel={m['billing.subscription_cancel_button']()}
        isPending={cancelSubscriptionMutation.isPending}
      />
    </div>
  )
}
