import { useState, useCallback } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { format } from 'date-fns'
import { Plug2 } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
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
import { Switch } from '@/components/ui/switch'
import { Skeleton } from '@/components/ui/skeleton'
import { PageHeader, ListPagination } from '@/components/shared'
import { ProviderSyncButton } from '@/components/billing/provider-sync-button'
import { EntitlementMappingDetailDialog } from '@/components/billing/entitlement-mapping-detail-dialog'
import { entitlementMappingsQueryOptions, queryKeys } from '@/data/query-options'
import { useUpdateEntitlementMapping } from '@/data/entitlement-mapping-mutations'
import type {
  EntitlementMappingResponse,
  EntitlementMappingListResponse,
} from '@/lib/api-generated'

const PAGE_SIZE = 20

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

interface EntitlementMappingsPageProps {
  realmId: string
  search: { page?: number; pageSize?: number; provider?: string }
}

export function EntitlementMappingsPage({ realmId, search }: EntitlementMappingsPageProps) {
  const queryClient = useQueryClient()
  const [providerFilter, setProviderFilter] = useState<string>(search.provider ?? 'all')
  const [page, setPage] = useState(search.page ?? 0)
  const [selectedMappingId, setSelectedMappingId] = useState<string | null>(null)
  const [detailOpen, setDetailOpen] = useState(false)

  const filters = {
    paymentProvider: providerFilter !== 'all' ? providerFilter : undefined,
    page,
    pageSize: search.pageSize ?? PAGE_SIZE,
  }

  const { data, isLoading } = useQuery({
    ...entitlementMappingsQueryOptions(realmId, filters),
    select: (rawData) => rawData as EntitlementMappingListResponse | undefined,
  })

  const mappings = data?.items ?? []
  const total = data?.total ?? 0

  const handleProviderFilterChange = useCallback((value: string) => {
    setProviderFilter(value)
    setPage(0)
  }, [])

  const handleRowClick = useCallback((mappingId: string) => {
    setSelectedMappingId(mappingId)
    setDetailOpen(true)
  }, [])

  const handleSyncComplete = useCallback(() => {
    queryClient.invalidateQueries({
      queryKey: [queryKeys.entitlementMappings(realmId, {})],
    })
  }, [queryClient, realmId])

  return (
    <div className="space-y-6" data-testid="entitlement-mappings-page">
      <PageHeader title="Entitlement Mappings" headingTestId="entitlement-mappings-heading" />

      {/* Filter bar + sync button */}
      <div className="flex items-center justify-between gap-4">
        <Select value={providerFilter} onValueChange={handleProviderFilterChange}>
          <SelectTrigger className="w-[160px]" data-testid="provider-filter-select">
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

        <ProviderSyncButton realmId={realmId} onSyncComplete={handleSyncComplete} />
      </div>

      {/* Table or empty state */}
      {isLoading ? (
        <LoadingSkeleton />
      ) : mappings.length === 0 ? (
        <Card className="border-dashed" data-testid="entitlement-mappings-empty-state">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <Plug2 className="h-12 w-12 text-muted-foreground mb-4" />
            <p className="text-sm text-muted-foreground text-center">
              No provider products synced yet. Sync provider products to see available mappings.
            </p>
          </CardContent>
        </Card>
      ) : (
        <>
          <Card>
            <CardHeader>
              <CardTitle>Mappings</CardTitle>
            </CardHeader>
            <CardContent>
              <Table data-testid="entitlement-mappings-table">
                <TableHeader>
                  <TableRow>
                    <TableHead>Payment Provider</TableHead>
                    <TableHead>External Product ID</TableHead>
                    <TableHead>External Price ID</TableHead>
                    <TableHead>Entitlement Key</TableHead>
                    <TableHead>Subscription Points</TableHead>
                    <TableHead>Synced At</TableHead>
                    <TableHead>Enabled</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {mappings.map((mapping) => (
                    <MappingRow
                      key={mapping.id}
                      mapping={mapping}
                      realmId={realmId}
                      onClick={() => handleRowClick(mapping.id)}
                    />
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
              testIdPrefix="entitlement-mappings-pagination"
            />
          )}
        </>
      )}

      <EntitlementMappingDetailDialog
        realmId={realmId}
        mappingId={selectedMappingId}
        open={detailOpen}
        onOpenChange={setDetailOpen}
      />
    </div>
  )
}

function MappingRow({
  mapping,
  realmId,
  onClick,
}: {
  mapping: EntitlementMappingResponse
  realmId: string
  onClick: () => void
}) {
  const updateMutation = useUpdateEntitlementMapping(realmId, mapping.id)

  const handleToggleEnabled = () => {
    updateMutation.mutate({
      entitlementKey: mapping.entitlementKey,
      enabled: !mapping.enabled,
      pointsPerPeriod: mapping.pointsPerPeriod ?? null,
      grantPeriodType: mapping.grantPeriodType as 'once' | 'daily' | 'weekly' | 'monthly' | null,
      validityDays: mapping.validityDays ?? null,
      grantOnSubscribe: mapping.grantOnSubscribe,
      maxPeriods: mapping.maxPeriods ?? null,
    })
  }

  const pointsPolicyLabel = mapping.pointsPerPeriod != null ? 'Grant Configured' : 'No Grant'

  return (
    <TableRow
      className="cursor-pointer"
      onClick={onClick}
      data-testid={`mapping-row-${mapping.id}`}
    >
      <TableCell className="font-medium">{formatProviderName(mapping.paymentProvider)}</TableCell>
      <TableCell className="font-mono text-sm">{mapping.externalProductId}</TableCell>
      <TableCell className="font-mono text-sm">{mapping.externalPriceId ?? '---'}</TableCell>
      <TableCell className="font-mono text-sm">{mapping.entitlementKey}</TableCell>
      <TableCell>
        <span
          className={
            mapping.pointsPerPeriod != null
              ? 'text-sm text-green-700'
              : 'text-sm text-muted-foreground'
          }
        >
          {pointsPolicyLabel}
        </span>
      </TableCell>
      <TableCell className="text-sm">
        {mapping.syncedAt ? format(new Date(mapping.syncedAt), 'PP') : '---'}
      </TableCell>
      <TableCell>
        <Switch
          checked={mapping.enabled}
          onCheckedChange={handleToggleEnabled}
          disabled={updateMutation.isPending}
          data-testid={`mapping-enabled-toggle-${mapping.id}`}
          onClick={(e: React.MouseEvent) => e.stopPropagation()}
        />
      </TableCell>
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
