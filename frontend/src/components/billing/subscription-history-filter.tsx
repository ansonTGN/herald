import { z } from 'zod'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Calendar } from 'lucide-react'
import type {
  HistoryFilters,
  SubscriptionHistoryEventType,
  SubscriptionStatus,
} from '@/types/billing'
import { getEventTypeLabels, getSubscriptionStatusLabels } from '@/types/billing'
import { optionalStringEnum } from '@/lib/form-utils'
import { toDateInputValue, toUtcDateRangeBoundary } from '@/lib/date-utils'
import { m } from '@/paraglide/messages'

const ALL_FILTER_VALUE = '__all__'

const eventTypeEnum = z.enum([
  'created',
  'upgraded',
  'downgraded',
  'canceled',
  'expired',
  'renewed',
  'reactivated',
  'billing_period_changed',
])

const subscriptionStatusEnum = z.enum([
  'active',
  'trialing',
  'past_due',
  'canceled',
  'incomplete',
  'expired',
  'paused',
  'disputed',
  'scheduled_cancel',
])

const sortOrderEnum = z.enum(['asc', 'desc'])

const historyFilterSchema = z.object({
  userId: z.string().optional(),
  entitlementKey: z.string().optional(),
  eventType: optionalStringEnum(eventTypeEnum) as z.ZodType<
    SubscriptionHistoryEventType | undefined
  >,
  subscriptionStatus: optionalStringEnum(subscriptionStatusEnum) as z.ZodType<
    SubscriptionStatus | undefined
  >,
  fromDate: z.string().optional(),
  toDate: z.string().optional(),
  sortBy: z.string().optional(),
  sortOrder: optionalStringEnum(sortOrderEnum) as z.ZodType<'asc' | 'desc' | undefined>,
})

interface SubscriptionHistoryFilterProps {
  filters: HistoryFilters
  onFiltersChange: (filters: HistoryFilters) => void
  onReset: () => void
  loading?: boolean
  className?: string
}

export function SubscriptionHistoryFilter({
  filters,
  onFiltersChange,
  onReset,
  loading,
  className,
}: SubscriptionHistoryFilterProps) {
  const form = useAppForm({
    schema: historyFilterSchema,
    defaultValues: {
      userId: filters.userId || '',
      entitlementKey: filters.entitlementKey || '',
      eventType: filters.eventType,
      subscriptionStatus: filters.subscriptionStatus,
      fromDate: filters.fromDate || '',
      toDate: filters.toDate || '',
      sortBy: filters.sortBy || 'timestamp',
      sortOrder: filters.sortOrder || 'desc',
    },
    onSubmit: async ({ value }) => {
      const newFilters: HistoryFilters = {
        userId: value.userId || undefined,
        entitlementKey: value.entitlementKey || undefined,
        eventType: value.eventType,
        subscriptionStatus: value.subscriptionStatus,
        fromDate: value.fromDate ? toUtcDateRangeBoundary(value.fromDate, 'start') : undefined,
        toDate: value.toDate ? toUtcDateRangeBoundary(value.toDate, 'end') : undefined,
        sortBy: value.sortBy,
        sortOrder: value.sortOrder,
      }
      onFiltersChange(newFilters)
    },
  })

  function handleReset() {
    form.reset()
    onReset()
  }

  return (
    <Card className={className} data-testid="subscription-history-filter">
      <CardContent className="p-4">
        <form
          onSubmit={(e) => {
            e.preventDefault()
            e.stopPropagation()
            form.handleSubmit()
          }}
        >
          <AppForm>
            <div className="space-y-3">
              <div className="grid gap-3 md:grid-cols-3 lg:grid-cols-4">
                {/* User ID */}
                <div className="space-y-1">
                  <form.Field name="userId">
                    {(field) => (
                      <>
                        <Label htmlFor={field.name}>
                          {m['billing.subscription_filter_user_id']()}
                        </Label>
                        <Input
                          id={field.name}
                          type="text"
                          value={field.state.value ?? ''}
                          onChange={(e) => field.handleChange(e.target.value)}
                          placeholder={m['billing.subscription_filter_user_id_placeholder']()}
                          data-testid="filter-user-id"
                        />
                      </>
                    )}
                  </form.Field>
                </div>

                {/* Entitlement Key */}
                <div className="space-y-1">
                  <form.Field name="entitlementKey">
                    {(field) => (
                      <>
                        <Label htmlFor={field.name}>
                          {m['billing.subscription_filter_entitlement_key']()}
                        </Label>
                        <Input
                          id={field.name}
                          type="text"
                          value={field.state.value ?? ''}
                          onChange={(e) => field.handleChange(e.target.value)}
                          placeholder={m[
                            'billing.subscription_filter_entitlement_key_placeholder'
                          ]()}
                          data-testid="filter-entitlement-key"
                        />
                      </>
                    )}
                  </form.Field>
                </div>

                {/* Event Type */}
                <div className="space-y-1">
                  <form.Field name="eventType">
                    {(field) => (
                      <>
                        <Label htmlFor={field.name}>
                          {m['billing.subscription_filter_event_type']()}
                        </Label>
                        <Select
                          value={field.state.value ?? ALL_FILTER_VALUE}
                          onValueChange={(value) =>
                            field.handleChange(
                              value === ALL_FILTER_VALUE
                                ? undefined
                                : (value as SubscriptionHistoryEventType)
                            )
                          }
                          data-testid="filter-event-type"
                        >
                          <SelectTrigger id={field.name}>
                            <SelectValue
                              placeholder={m['billing.subscription_filter_select_event_type']()}
                            />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value={ALL_FILTER_VALUE}>
                              {m['billing.subscription_filter_all_event_types']()}
                            </SelectItem>
                            {(
                              Object.entries(getEventTypeLabels()) as [
                                SubscriptionHistoryEventType,
                                string,
                              ][]
                            ).map(([type, label]) => (
                              <SelectItem key={type} value={type}>
                                {label}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </>
                    )}
                  </form.Field>
                </div>

                {/* Subscription Status */}
                <div className="space-y-1">
                  <form.Field name="subscriptionStatus">
                    {(field) => (
                      <>
                        <Label htmlFor={field.name}>
                          {m['billing.subscription_filter_status']()}
                        </Label>
                        <Select
                          value={field.state.value ?? ALL_FILTER_VALUE}
                          onValueChange={(value) =>
                            field.handleChange(
                              value === ALL_FILTER_VALUE ? undefined : (value as SubscriptionStatus)
                            )
                          }
                          data-testid="filter-status"
                        >
                          <SelectTrigger id={field.name}>
                            <SelectValue
                              placeholder={m['billing.subscription_filter_select_status']()}
                            />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value={ALL_FILTER_VALUE}>
                              {m['billing.subscription_filter_all_statuses']()}
                            </SelectItem>
                            {(
                              Object.entries(getSubscriptionStatusLabels()) as [
                                SubscriptionStatus,
                                string,
                              ][]
                            ).map(([status, label]) => (
                              <SelectItem key={status} value={status}>
                                {label}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </>
                    )}
                  </form.Field>
                </div>

                {/* From Date */}
                <div className="space-y-1">
                  <form.Field name="fromDate">
                    {(field) => (
                      <>
                        <Label htmlFor={field.name}>
                          {m['billing.subscription_filter_from_date']()}
                        </Label>
                        <div className="relative">
                          <Input
                            id={field.name}
                            type="date"
                            value={toDateInputValue(field.state.value)}
                            onChange={(e) => field.handleChange(e.target.value)}
                            data-testid="filter-from-date"
                          />
                          <Calendar className="absolute right-3 top-2.5 h-4 w-4 text-muted-foreground" />
                        </div>
                      </>
                    )}
                  </form.Field>
                </div>

                {/* To Date */}
                <div className="space-y-1">
                  <form.Field name="toDate">
                    {(field) => (
                      <>
                        <Label htmlFor={field.name}>
                          {m['billing.subscription_filter_to_date']()}
                        </Label>
                        <div className="relative">
                          <Input
                            id={field.name}
                            type="date"
                            value={toDateInputValue(field.state.value)}
                            onChange={(e) => field.handleChange(e.target.value)}
                            data-testid="filter-to-date"
                          />
                          <Calendar className="absolute right-3 top-2.5 h-4 w-4 text-muted-foreground" />
                        </div>
                      </>
                    )}
                  </form.Field>
                </div>

                {/* Sort By */}
                <div className="space-y-1">
                  <form.Field name="sortBy">
                    {(field) => (
                      <>
                        <Label htmlFor={field.name}>
                          {m['billing.subscription_filter_sort_by']()}
                        </Label>
                        <Select
                          value={field.state.value}
                          onValueChange={(value) => field.handleChange(value)}
                          data-testid="filter-sort-by"
                        >
                          <SelectTrigger id={field.name}>
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="timestamp">
                              {m['billing.subscription_filter_sort_timestamp']()}
                            </SelectItem>
                            <SelectItem value="eventType">
                              {m['billing.subscription_filter_sort_event_type']()}
                            </SelectItem>
                            <SelectItem value="subscriptionId">
                              {m['billing.subscription_filter_sort_subscription_id']()}
                            </SelectItem>
                          </SelectContent>
                        </Select>
                      </>
                    )}
                  </form.Field>
                </div>

                {/* Sort Order */}
                <div className="space-y-1">
                  <form.Field name="sortOrder">
                    {(field) => (
                      <>
                        <Label htmlFor={field.name}>
                          {m['billing.subscription_filter_sort_order']()}
                        </Label>
                        <Select
                          value={field.state.value ?? ''}
                          onValueChange={(value) =>
                            field.handleChange(value === '' ? undefined : (value as 'asc' | 'desc'))
                          }
                          data-testid="filter-sort-order"
                        >
                          <SelectTrigger id={field.name}>
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="desc">
                              {m['billing.subscription_filter_descending']()}
                            </SelectItem>
                            <SelectItem value="asc">
                              {m['billing.subscription_filter_ascending']()}
                            </SelectItem>
                          </SelectContent>
                        </Select>
                      </>
                    )}
                  </form.Field>
                </div>
              </div>

              {/* Action Buttons */}
              <div className="flex items-center justify-end gap-2 pt-2">
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={handleReset}
                  disabled={loading}
                  data-testid="reset-filter-button"
                >
                  {m['billing.subscription_filter_reset']()}
                </Button>
                <Button
                  type="submit"
                  size="sm"
                  disabled={loading}
                  data-testid="apply-filter-button"
                >
                  {loading
                    ? m['billing.subscription_filter_applying']()
                    : m['billing.subscription_filter_apply']()}
                </Button>
              </div>
            </div>
          </AppForm>
        </form>
      </CardContent>
    </Card>
  )
}
