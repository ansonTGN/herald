import { Card, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import type { ClientAppItem, SubscriptionDetailResponse } from '@/lib/api-generated'
import {
  getStatusBadgeVariant,
  getSubscriptionStatusLabels,
  type SubscriptionStatus,
} from '@/types/billing'
import { formatDate } from '@/lib/date-utils'
import { FileText } from 'lucide-react'
import { m } from '@/paraglide/messages'

interface SubscriptionSelectorProps {
  subscriptions: Array<{
    clientApp: ClientAppItem
    subscription: SubscriptionDetailResponse | null
  }>
  selectedId?: string
  onSelect: (subscriptionId: string) => void
  onApplyInvoice?: (subscriptionId: string) => void
}

export function SubscriptionSelector({
  subscriptions,
  selectedId,
  onSelect,
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
                        {subscription.plan?.title || m['billing.subscription_none']()}
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
                    {onApplyInvoice && (
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={(event) => {
                          event.stopPropagation()
                          onApplyInvoice(subscription.id)
                        }}
                        data-testid={`subscription-invoice-button-${subscription.id}`}
                      >
                        <FileText className="mr-2 h-4 w-4" />
                        {m['billing.subscription_invoice_button']()}
                      </Button>
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
