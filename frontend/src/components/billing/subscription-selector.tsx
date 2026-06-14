import { Card, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import type { ClientAppItem, SubscriptionDetailResponse } from '@/lib/api-generated'
import {
  getStatusBadgeVariant,
  getSubscriptionStatusLabels,
  type SubscriptionStatus,
} from '@/types/billing'
import { formatDate } from '@/lib/date-utils'
import { m } from '@/paraglide/messages'
import { InvoiceApplyRowButton } from '@/components/billing/invoices/invoice-apply-row-button'

interface SubscriptionSelectorProps {
  subscriptions: Array<{
    clientApp: ClientAppItem
    subscription: SubscriptionDetailResponse | null
  }>
  selectedId?: string
  onSelect: (subscriptionId: string) => void
  realmId?: string
  onApplyInvoice?: (subscriptionId: string) => void
}

export function SubscriptionSelector({
  subscriptions,
  selectedId,
  onSelect,
  realmId,
  onApplyInvoice,
}: SubscriptionSelectorProps) {
  if (subscriptions.length === 0) {
    return (
      <div
        className="text-center py-8 text-muted-foreground"
        data-testid="subscription-selector-empty"
      >
        {m['billing.subscription_not_found']()}
      </div>
    )
  }

  return (
    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3" data-testid="subscription-selector">
      {subscriptions.map(({ clientApp, subscription }) => {
        const subscriptionId = subscription?.id || clientApp.id
        const isSelected = selectedId === subscriptionId

        return (
          <Card
            key={clientApp.id}
            className={`cursor-pointer transition-all hover:shadow-md ${
              isSelected ? 'border-primary ring-2 ring-primary ring-offset-2' : 'border-border'
            }`}
            onClick={() => onSelect(subscriptionId)}
            data-testid={`subscription-card-${clientApp.id}`}
          >
            <CardContent className="pt-6">
              <div className="space-y-3">
                <div className="flex items-start justify-between">
                  <div>
                    <h3 className="font-semibold text-sm">{clientApp.name}</h3>
                    <p className="text-xs text-muted-foreground mt-0.5">{clientApp.clientId}</p>
                  </div>
                  {subscription && (
                    <Badge
                      variant={getStatusBadgeVariant(subscription.status as SubscriptionStatus)}
                      className="text-xs"
                    >
                      {getSubscriptionStatusLabels()[subscription.status as SubscriptionStatus]}
                    </Badge>
                  )}
                </div>

                {subscription ? (
                  <div className="space-y-1.5">
                    <div className="text-sm">
                      <span className="text-muted-foreground">
                        {m['billing.subscription_plan_label_colon']()}
                      </span>
                      <span className="font-medium">
                        {subscription.entitlementKey || m['billing.subscription_none']()}
                      </span>
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {subscription.currentPeriodEnd && (
                        <>
                          {m['billing.subscription_expires']({
                            date: formatDate(subscription.currentPeriodEnd),
                          })}
                        </>
                      )}
                    </div>
                    {onApplyInvoice && realmId && (
                      <div onClick={(event) => event.stopPropagation()}>
                        <InvoiceApplyRowButton
                          realmId={realmId}
                          referenceType="subscription"
                          referenceId={subscription.id}
                          onApply={() => onApplyInvoice(subscription.id)}
                          label={m['billing.subscription_invoice_button']()}
                          variant="outline"
                          testIdPrefix={`subscription-invoice-button-${subscription.id}`}
                        />
                      </div>
                    )}
                  </div>
                ) : (
                  <div className="text-sm text-muted-foreground">
                    {m['billing.subscription_no_sub']()}
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
