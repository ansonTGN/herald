import { useState, useCallback } from 'react'
import { useQuery } from '@tanstack/react-query'
import { format } from 'date-fns'
import { CreditCard } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Skeleton } from '@/components/ui/skeleton'
import { PageHeader, ListPagination } from '@/components/shared'
import { subscriptionsQueryOptions } from '@/data/query-options'
import { m } from '@/paraglide/messages'
import type { SubscriptionListItemResponse, SubscriptionListResponse } from '@/lib/api-generated'

const PAGE_SIZE = 20

const STATUS_FILTER_OPTIONS = [
  { value: 'all', label: 'All' },
  { value: 'active', label: 'Active' },
  { value: 'past_due', label: 'Past Due' },
  { value: 'canceled', label: 'Canceled' },
  { value: 'expired', label: 'Expired' },
] as const

const PROVIDER_FILTER_OPTIONS = [
  { value: 'all', label: 'All' },
  { value: 'stripe', label: 'Stripe' },
  { value: 'creem', label: 'Creem' },
] as const

function formatProviderName(provider: string): string {
  const names: Record<string, string> = {
    stripe: 'Stripe',
    creem: 'Creem',
    wechat: 'WeChat Pay',
    shopify: 'Shopify',
  }
  return names[provider] ?? provider
}

function getStatusBadgeClass(status: string): string {
  switch (status) {
    case 'active':
      return 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400'
    case 'past_due':
      return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400'
    case 'canceled':
      return 'bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-400'
    case 'expired':
      return 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400'
    default:
      return 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400'
  }
}

function formatStatusLabel(status: string): string {
  const labels: Record<string, string> = {
    active: m['billing.subscription_status_label_active'](),
    past_due: m['billing.subscription_status_label_past_due'](),
    canceled: m['billing.subscription_status_label_canceled'](),
    expired: m['billing.subscription_status_label_expired'](),
    trialing: m['billing.subscription_status_label_trialing'](),
    incomplete: m['billing.subscription_status_label_incomplete'](),
    paused: m['billing.subscription_status_label_paused'](),
    disputed: m['billing.subscription_status_label_disputed'](),
  }
  return labels[status] ?? status
}

interface AdminSubscriptionListPageProps {
  realmId: string
  search: {
    page?: number
    pageSize?: number
    entitlementKey?: string
    status?: string
    paymentProvider?: string
  }
}

export function AdminSubscriptionListPage({ realmId, search }: AdminSubscriptionListPageProps) {
  const [entitlementKeyFilter, setEntitlementKeyFilter] = useState<string>(
    search.entitlementKey ?? ''
  )
  const [statusFilter, setStatusFilter] = useState<string>(search.status ?? 'all')
  const [providerFilter, setProviderFilter] = useState<string>(search.paymentProvider ?? 'all')
  const [page, setPage] = useState(search.page ?? 0)

  const filters = {
    entitlementKey: entitlementKeyFilter || undefined,
    status: statusFilter !== 'all' ? statusFilter : undefined,
    paymentProvider: providerFilter !== 'all' ? providerFilter : undefined,
    page,
    pageSize: search.pageSize ?? PAGE_SIZE,
  }

  const { data, isLoading } = useQuery({
    ...subscriptionsQueryOptions(realmId, filters),
    select: (rawData) => rawData as SubscriptionListResponse | undefined,
  })

  const subscriptions = data?.items ?? []
  const total = data?.total ?? 0

  const handleStatusFilterChange = useCallback((value: string) => {
    setStatusFilter(value)
    setPage(0)
  }, [])

  const handleProviderFilterChange = useCallback((value: string) => {
    setProviderFilter(value)
    setPage(0)
  }, [])

  const handleEntitlementKeyChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setEntitlementKeyFilter(e.target.value)
    setPage(0)
  }, [])

  const hasFilters =
    entitlementKeyFilter !== '' || statusFilter !== 'all' || providerFilter !== 'all'

  return (
    <div className="space-y-6" data-testid="admin-subscription-list-page">
      <PageHeader
        title={m['billing.subscription_list_title']()}
        headingTestId="admin-subscription-list-heading"
      />

      {/* Filter bar */}
      <div className="flex flex-wrap items-center gap-4">
        <Input
          placeholder={m['billing.subscription_filter_entitlement_key_placeholder']()}
          value={entitlementKeyFilter}
          onChange={handleEntitlementKeyChange}
          className="w-[220px]"
          data-testid="entitlement-key-filter-input"
        />

        <Select value={statusFilter} onValueChange={handleStatusFilterChange}>
          <SelectTrigger className="w-[160px]" data-testid="status-filter-select">
            <SelectValue placeholder="All Statuses" />
          </SelectTrigger>
          <SelectContent>
            {STATUS_FILTER_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.value === 'all' ? m['billing.subscription_filter_all_statuses']() : opt.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <Select value={providerFilter} onValueChange={handleProviderFilterChange}>
          <SelectTrigger className="w-[160px]" data-testid="payment-provider-filter-select">
            <SelectValue placeholder="All Providers" />
          </SelectTrigger>
          <SelectContent>
            {PROVIDER_FILTER_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* Table or empty state */}
      {isLoading ? (
        <LoadingSkeleton />
      ) : subscriptions.length === 0 ? (
        <Card className="border-dashed" data-testid="admin-subscription-list-empty-state">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <CreditCard className="h-12 w-12 text-muted-foreground mb-4" />
            <p className="text-sm text-muted-foreground text-center">
              {hasFilters
                ? m['billing.subscription_list_no_match']()
                : m['billing.subscription_list_empty']()}
            </p>
          </CardContent>
        </Card>
      ) : (
        <>
          <Card>
            <CardHeader>
              <CardTitle>{m['billing.subscription_list_title']()}</CardTitle>
            </CardHeader>
            <CardContent>
              <Table data-testid="admin-subscription-list-table">
                <TableHeader>
                  <TableRow>
                    <TableHead>{m['billing.subscription_entitlement_key']()}</TableHead>
                    <TableHead>{m['billing.subscription_payment_provider']()}</TableHead>
                    <TableHead>{m['billing.subscription_external_price_id']()}</TableHead>
                    <TableHead>{m['billing.subscription_synced_at']()}</TableHead>
                    <TableHead>{m['common.status']()}</TableHead>
                    <TableHead>Client App</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {subscriptions.map((sub) => (
                    <SubscriptionRow key={sub.id} subscription={sub} />
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>

          {total > 0 && (
            <ListPagination
              page={page}
              pageSize={search.pageSize ?? PAGE_SIZE}
              total={total}
              onPageChange={setPage}
              testIdPrefix="admin-subscription-list-pagination"
            />
          )}
        </>
      )}
    </div>
  )
}

function SubscriptionRow({ subscription }: { subscription: SubscriptionListItemResponse }) {
  return (
    <TableRow data-testid={`subscription-row-${subscription.id}`}>
      <TableCell className="font-mono text-sm">{subscription.entitlementKey}</TableCell>
      <TableCell className="font-medium">
        {formatProviderName(subscription.paymentProvider)}
      </TableCell>
      <TableCell className="font-mono text-sm">{subscription.externalPriceId ?? '---'}</TableCell>
      <TableCell className="text-sm">
        {subscription.syncedAt ? format(new Date(subscription.syncedAt), 'PP') : '---'}
      </TableCell>
      <TableCell>
        <span
          className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ${getStatusBadgeClass(subscription.status)}`}
        >
          {formatStatusLabel(subscription.status)}
        </span>
      </TableCell>
      <TableCell className="text-sm">{subscription.clientAppId ?? '---'}</TableCell>
    </TableRow>
  )
}

function LoadingSkeleton() {
  return (
    <Card>
      <CardHeader>
        <Skeleton className="h-6 w-32" />
      </CardHeader>
      <CardContent className="space-y-4">
        {Array.from({ length: 5 }).map((_, i) => (
          <Skeleton key={i} className="h-10 w-full" />
        ))}
      </CardContent>
    </Card>
  )
}
