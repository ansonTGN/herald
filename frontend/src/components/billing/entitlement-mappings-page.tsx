import { useState, useCallback, useMemo } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { format } from 'date-fns'
import { toast } from 'sonner'
import { Plug2, AlertTriangle } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { Skeleton } from '@/components/ui/skeleton'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { PageHeader } from '@/components/shared'
import { ProviderSyncButton } from '@/components/billing/provider-sync-button'
import { formatProviderName } from '@/components/billing/format-provider-name'
import { ProtectedPriceConfirmDialog } from '@/components/billing/entitlement-mapping-detail-dialog'
import {
  groupByProduct,
  groupByEntitlementKey,
  productKeyOf,
  isWebhookUnresolvedPrice,
  hasWebhookUnresolvedPrice,
} from '@/components/billing/entitlement-mapping-grouping'
import { deriveSharedKeyColor } from '@/components/billing/shared-key-color'
import { entitlementMappingsQueryOptions, queryKeys } from '@/data/query-options'
import { useBatchUpdateEntitlementMappings } from '@/data/entitlement-mapping-mutations'
import {
  batchEntitlementMappingsSchema,
  type PriceMappingUpdateFormData,
} from '@/lib/schemas/billing-forms'
import {
  isProtectedPriceError,
  extractActiveSubscriptions,
} from '@/data/entitlement-mapping-mutations'
import { usePermission } from '@/hooks/use-permission'
import { m } from '@/paraglide/messages'
import type {
  EntitlementMappingResponse,
  EntitlementMappingListResponse,
  BatchUpdateEntitlementMappingsRequest,
  PriceMappingUpdate,
} from '@/lib/api-generated'

const PROVIDER_FILTER_OPTIONS = [
  { value: 'all', label: 'All' },
  { value: 'stripe', label: 'Stripe' },
  { value: 'creem', label: 'Creem' },
] as const

interface EntitlementMappingsPageProps {
  realmId: string
  search: { page?: number; pageSize?: number; provider?: string }
}

export function EntitlementMappingsPage({ realmId, search }: EntitlementMappingsPageProps) {
  const queryClient = useQueryClient()
  const { hasPermission } = usePermission()
  const canManage = hasPermission('billing.manage')
  const canManagePoints = hasPermission('points.manage')

  const [providerFilter, setProviderFilter] = useState<string>(search.provider ?? 'all')
  const [productFilter, setProductFilter] = useState<string>('all')
  const [entitlementKeyFilter, setEntitlementKeyFilter] = useState<string>('all')
  const [selectedProductKey, setSelectedProductKey] = useState<string | null>(null)

  // Protected-price 409 confirmation state: surfaces the active-sub count for
  // the rejected batch (the backend rolls back the whole transaction; there is
  // no "force" action, so only the count is retained).
  const [protectedConfirm, setProtectedConfirm] = useState<{
    activeSubscriptions: number
  } | null>(null)

  const filters = {
    paymentProvider: providerFilter !== 'all' ? providerFilter : undefined,
  }

  const { data, isLoading } = useQuery({
    ...entitlementMappingsQueryOptions(realmId, filters),
    select: (rawData) => rawData as EntitlementMappingListResponse | undefined,
  })

  const allMappings: EntitlementMappingResponse[] = useMemo(() => data?.items ?? [], [data])

  const productGroups = useMemo(() => groupByProduct(allMappings), [allMappings])

  // Unique product id list for the product filter dropdown.
  const productFilterOptions = useMemo(() => {
    const seen = new Map<string, { value: string; label: string }>()
    for (const g of productGroups) {
      const value = productKeyOf(g)
      if (!seen.has(value)) {
        seen.set(value, {
          value,
          label: `${formatProviderName(g.paymentProvider)} · ${g.externalProductId}`,
        })
      }
    }
    return Array.from(seen.values())
  }, [productGroups])

  // Unique entitlement key list (across all loaded mappings) for the key filter.
  const entitlementKeyOptions = useMemo(() => {
    const seen = new Set<string>()
    for (const item of allMappings) seen.add(item.entitlementKey)
    return Array.from(seen).sort()
  }, [allMappings])

  // Apply provider (server) + product + entitlement_key (client) filters.
  const filteredGroups = useMemo(() => {
    let groups = productGroups
    if (productFilter !== 'all') {
      groups = groups.filter((g) => productKeyOf(g) === productFilter)
    }
    if (entitlementKeyFilter !== 'all') {
      groups = groups
        .map((g) => ({
          ...g,
          prices: g.prices.filter((p) => p.entitlementKey === entitlementKeyFilter),
        }))
        .filter((g) => g.prices.length > 0)
    }
    return groups
  }, [productGroups, productFilter, entitlementKeyFilter])

  // Derive the EFFECTIVE selected product key without a setState effect:
  // if the user's selection still exists in the filtered list, keep it;
  // otherwise fall back to the first product (or null when empty). This
  // avoids cascading renders from a setState-in-effect.
  const effectiveSelectedProductKey: string | null = useMemo(() => {
    if (filteredGroups.length === 0) return null
    const exists = filteredGroups.some((g) => productKeyOf(g) === selectedProductKey)
    return exists ? selectedProductKey : productKeyOf(filteredGroups[0])
  }, [filteredGroups, selectedProductKey])

  const selectedGroup = useMemo(
    () => filteredGroups.find((g) => productKeyOf(g) === effectiveSelectedProductKey) ?? null,
    [filteredGroups, effectiveSelectedProductKey]
  )

  const handleSyncComplete = useCallback(() => {
    queryClient.invalidateQueries({
      queryKey: queryKeys.entitlementMappings(realmId, {}),
    })
  }, [queryClient, realmId])

  const showWebhookBanner = hasWebhookUnresolvedPrice(allMappings)

  return (
    <div className="space-y-6" data-testid="entitlement-mappings-page">
      <PageHeader
        title={m['billing.entitlement_mappings_title']()}
        subtitle={m['billing.entitlement_mappings_subtitle']()}
        headingTestId="entitlement-mappings-heading"
      />

      {!canManage && (
        <div
          className="rounded-md border border-yellow-500/50 bg-yellow-50 px-4 py-3 text-sm text-yellow-800"
          data-testid="readonly-perm-banner"
          role="alert"
        >
          {m['billing.readonly_banner']()}
        </div>
      )}

      {showWebhookBanner && (
        <div
          className="rounded-md border border-destructive/50 bg-destructive/10 px-4 py-3 text-sm text-destructive"
          data-testid="webhook-price-unresolved-banner"
          role="alert"
        >
          <AlertTriangle className="mr-2 inline h-4 w-4 align-text-bottom" />
          <span className="font-medium">{m['billing.webhook_price_unresolved_title']()}</span>
          {' — '}
          {m['billing.webhook_price_unresolved_body']()}
        </div>
      )}

      {/* Toolbar */}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-2">
          <Select value={providerFilter} onValueChange={setProviderFilter}>
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

          <Select value={productFilter} onValueChange={setProductFilter}>
            <SelectTrigger className="w-[220px]" data-testid="product-filter-select">
              <SelectValue placeholder="All Products" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Products</SelectItem>
              {productFilterOptions.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Select value={entitlementKeyFilter} onValueChange={setEntitlementKeyFilter}>
            <SelectTrigger className="w-[180px]" data-testid="entitlement-key-filter-select">
              <SelectValue placeholder="All Keys" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Keys</SelectItem>
              {entitlementKeyOptions.map((k) => (
                <SelectItem key={k} value={k}>
                  {k}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <ProviderSyncButton realmId={realmId} onSyncComplete={handleSyncComplete} />
      </div>

      {isLoading ? (
        <LoadingSkeleton />
      ) : allMappings.length === 0 ? (
        <EmptyState onSync={() => undefined} canTriggerSync={canManage} />
      ) : (
        <div className="grid grid-cols-1 gap-6 lg:grid-cols-[320px_1fr]">
          {/* Master: product list */}
          <Card>
            <CardHeader>
              <CardTitle>Products</CardTitle>
            </CardHeader>
            <CardContent className="p-0">
              <ul className="divide-y" data-testid="mapping-product-list">
                {filteredGroups.map((g) => {
                  const key = productKeyOf(g)
                  const isSelected = key === effectiveSelectedProductKey
                  const primaryColor = deriveSharedKeyColor(g.prices[0]?.entitlementKey ?? '')
                  return (
                    <li key={key}>
                      <button
                        type="button"
                        onClick={() => setSelectedProductKey(key)}
                        className={`flex w-full items-center justify-between gap-2 px-4 py-3 text-left hover:bg-accent ${
                          isSelected ? 'bg-accent' : ''
                        }`}
                        data-testid={`mapping-product-row-${g.externalProductId}`}
                        aria-current={isSelected ? 'true' : undefined}
                      >
                        <span className="flex items-center gap-2 truncate">
                          {primaryColor.hue !== 0 && (
                            <span
                              className="inline-block h-2 w-2 shrink-0 rounded-full"
                              style={{ backgroundColor: `hsl(${primaryColor.hue} 70% 50%)` }}
                              aria-hidden
                            />
                          )}
                          <span className="truncate text-sm font-medium">
                            {g.externalProductId}
                          </span>
                          <Badge variant="secondary" className="font-mono text-xs">
                            {formatProviderName(g.paymentProvider)}
                          </Badge>
                        </span>
                        <Badge variant="outline">{g.prices.length}</Badge>
                      </button>
                    </li>
                  )
                })}
              </ul>
            </CardContent>
          </Card>

          {/* Detail: same-screen multi-price editor.
              The `key` resets the panel's local edit state when the selected
              product changes (React-recommended alternative to a sync-seed
              effect, avoids cascading renders). */}
          {selectedGroup && (
            <DetailPanel
              key={productKeyOf(selectedGroup)}
              realmId={realmId}
              group={selectedGroup}
              canManage={canManage}
              canManagePoints={canManagePoints}
              onProtectedPriceError={(activeSubscriptions) =>
                setProtectedConfirm({ activeSubscriptions })
              }
            />
          )}
        </div>
      )}

      <ProtectedPriceConfirmDialog
        open={protectedConfirm !== null}
        activeSubscriptions={protectedConfirm?.activeSubscriptions ?? null}
        onOpenChange={(open) => {
          if (!open) setProtectedConfirm(null)
        }}
      />
    </div>
  )
}

// ==================== Detail panel (multi-price editor) ====================

interface DetailPanelProps {
  realmId: string
  group: {
    paymentProvider: string
    externalProductId: string
    prices: EntitlementMappingResponse[]
  }
  canManage: boolean
  canManagePoints: boolean
  onProtectedPriceError: (activeSubscriptions: number) => void
}

function DetailPanel({
  realmId,
  group,
  canManage,
  canManagePoints,
  onProtectedPriceError,
}: DetailPanelProps) {
  const batchMutation = useBatchUpdateEntitlementMappings(realmId)

  // Controlled field-array state: one entry per price row, seeded from the
  // loaded group. The parent remounts this panel via `key` when the selected
  // product changes, so the lazy initializer re-runs fresh (no sync effect).
  const [rows, setRows] = useState<PriceMappingUpdateFormData[]>(() => seedRows(group.prices))

  const keyGroups = useMemo(() => groupByEntitlementKey(group.prices), [group.prices])

  const updateRow = useCallback((mappingId: string, patch: Partial<PriceMappingUpdateFormData>) => {
    setRows((prev) => prev.map((r) => (r.mappingId === mappingId ? { ...r, ...patch } : r)))
  }, [])

  const handleSave = () => {
    const request: BatchUpdateEntitlementMappingsRequest = {
      paymentProvider: group.paymentProvider,
      externalProductId: group.externalProductId,
      updates: rows.map(toPriceMappingUpdate),
    }
    // Validate the whole batch before submit.
    const parsed = batchEntitlementMappingsSchema.safeParse(request)
    if (!parsed.success) {
      // Surface the first validation issue via toast through the mutation's
      // generic error path is not appropriate; alert is simplest for now.
      // The backend will re-validate authoritatively on submit.
      const firstError = parsed.error.issues[0]
      toast.error(firstError ? String(firstError.message) : 'Validation failed')
      return
    }
    batchMutation.mutate(request, {
      onError: (error) => {
        if (isProtectedPriceError(error)) {
          const count = extractActiveSubscriptions(error)
          if (count != null) {
            onProtectedPriceError(count)
          }
        }
        // Non-409 errors are already toasted by the mutation.
      },
    })
  }

  const syncedAt = latestSyncedAt(group.prices)

  return (
    <Card data-testid="mapping-detail-panel">
      <CardHeader>
        <div className="flex items-center justify-between gap-2" data-testid="detail-head">
          <div className="flex items-center gap-2">
            <span className="text-base font-semibold">{group.externalProductId}</span>
            <Badge variant="secondary" className="font-mono text-xs">
              {formatProviderName(group.paymentProvider)}
            </Badge>
          </div>
          <span className="text-xs text-muted-foreground">
            {syncedAt ? format(new Date(syncedAt), 'PP') : '---'}
          </span>
        </div>
      </CardHeader>
      <CardContent className="space-y-6">
        {keyGroups.map((kg) => {
          const color = deriveSharedKeyColor(kg.entitlementKey)
          return (
            <div
              key={kg.entitlementKey}
              className="rounded-md border-l-4 pl-4"
              style={{ borderLeftColor: color.hue !== 0 ? `hsl(${color.hue} 70% 50%)` : undefined }}
            >
              <div className="mb-3 flex items-center gap-2">
                <Badge variant="outline" data-testid={`shared-key-chip-${kg.entitlementKey}`}>
                  {color.hue !== 0 && (
                    <span
                      className="mr-1 inline-block h-2 w-2 rounded-full align-middle"
                      style={{ backgroundColor: `hsl(${color.hue} 70% 50%)` }}
                      aria-hidden
                    />
                  )}
                  <span className="font-mono">{kg.entitlementKey}</span>
                </Badge>
                {kg.prices.length > 1 && (
                  <span className="text-xs text-muted-foreground">
                    {m['billing.shared_across_n_prices']({ count: kg.prices.length })}
                  </span>
                )}
              </div>

              <div className="space-y-4">
                {kg.prices.map((price) => {
                  const row = rows.find((r) => r.mappingId === price.id)
                  if (!row) return null
                  return (
                    <PriceEditRow
                      key={price.id}
                      price={price}
                      row={row}
                      canManage={canManage}
                      canManagePoints={canManagePoints}
                      isUnresolved={isWebhookUnresolvedPrice(price)}
                      onChange={(patch) => updateRow(price.id, patch)}
                    />
                  )
                })}
              </div>
            </div>
          )
        })}

        {canManage && (
          <div className="flex justify-end">
            <Button
              onClick={handleSave}
              disabled={batchMutation.isPending}
              data-testid="save-mapping-button"
            >
              {batchMutation.isPending ? 'Saving...' : m['shared.save_changes']()}
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  )
}

// ==================== Single price edit row ====================

interface PriceEditRowProps {
  price: EntitlementMappingResponse
  row: PriceMappingUpdateFormData
  canManage: boolean
  canManagePoints: boolean
  isUnresolved: boolean
  onChange: (patch: Partial<PriceMappingUpdateFormData>) => void
}

function PriceEditRow({
  price,
  row,
  canManage,
  canManagePoints,
  isUnresolved,
  onChange,
}: PriceEditRowProps) {
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const editDisabled = !canManage
  const pointsDisabled = !canManage || !canManagePoints

  return (
    <div
      className={`rounded-md border p-3 ${
        isUnresolved ? 'border-destructive/60 bg-destructive/5' : ''
      }`}
      data-testid={`price-edit-row-${price.externalPriceId ?? price.id}`}
    >
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <Field label={m['billing.field_price_id']()}>
          <Input value={price.externalPriceId ?? price.id} readOnly className="font-mono text-sm" />
        </Field>

        <Field label="Entitlement Key">
          <Input
            value={row.entitlementKey}
            onChange={(e) => onChange({ entitlementKey: e.target.value })}
            disabled={editDisabled}
            className="font-mono text-sm"
          />
          <p className="text-xs text-muted-foreground">
            {m['billing.editing_key_renames_group']()}
          </p>
        </Field>

        <Field
          label={m['billing.field_billing_type']()}
          required={isUnresolved && !row.billingType}
        >
          <Select
            value={row.billingType ?? ''}
            onValueChange={(v) => onChange({ billingType: v || null })}
            disabled={editDisabled}
          >
            <SelectTrigger className={isUnresolved && !row.billingType ? 'border-destructive' : ''}>
              <SelectValue placeholder="—" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="recurring">{m['billing.billing_type_recurring']()}</SelectItem>
              <SelectItem value="one_time">{m['billing.billing_type_one_time']()}</SelectItem>
            </SelectContent>
          </Select>
        </Field>

        <Field label={m['billing.field_period']()}>
          <Input
            value={row.billingPeriod ?? ''}
            onChange={(e) => onChange({ billingPeriod: e.target.value || null })}
            disabled={editDisabled}
            placeholder="month, year, …"
            className="text-sm"
          />
        </Field>

        <Field
          label={m['billing.field_points_per_period']()}
          required={isUnresolved && row.pointsPerPeriod == null}
        >
          <Input
            type="number"
            min={0}
            value={row.pointsPerPeriod ?? ''}
            onChange={(e) =>
              onChange({
                pointsPerPeriod: e.target.value === '' ? null : Number(e.target.value),
              })
            }
            disabled={pointsDisabled}
            placeholder="1000"
            className={isUnresolved && row.pointsPerPeriod == null ? 'border-destructive' : ''}
          />
        </Field>

        <Field label="Enabled">
          <div className="flex items-center gap-2">
            <Switch
              checked={row.enabled ?? false}
              onCheckedChange={(checked: boolean) => onChange({ enabled: checked })}
              disabled={editDisabled}
              data-testid={`price-enabled-toggle-${price.externalPriceId ?? price.id}`}
              aria-label={
                row.enabled ? m['billing.price_enabled']() : m['billing.price_disabled']()
              }
            />
            <span className="text-sm text-muted-foreground">
              {row.enabled ? m['billing.price_enabled']() : m['billing.price_disabled']()}
            </span>
          </div>
          {/* Protected-price active-subs note: surfaced only after a 409 (the
              backend is the source of truth for the active-subscription lock);
              no client field pre-disables the toggle. */}
        </Field>
      </div>

      <Collapsible open={advancedOpen} onOpenChange={setAdvancedOpen}>
        <CollapsibleTrigger asChild>
          <Button variant="ghost" size="sm" disabled={editDisabled} className="mt-2">
            {advancedOpen ? 'Hide advanced' : 'Advanced'}
          </Button>
        </CollapsibleTrigger>
        <CollapsibleContent>
          <div className="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2">
            <Field label={m['billing.field_grant_period_type']()}>
              <Select
                value={row.grantPeriodType ?? ''}
                onValueChange={(v) =>
                  onChange({
                    grantPeriodType: (v || null) as 'once' | 'daily' | 'weekly' | 'monthly' | null,
                  })
                }
                disabled={pointsDisabled}
              >
                <SelectTrigger>
                  <SelectValue placeholder="—" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="once">Once</SelectItem>
                  <SelectItem value="daily">Daily</SelectItem>
                  <SelectItem value="weekly">Weekly</SelectItem>
                  <SelectItem value="monthly">Monthly</SelectItem>
                </SelectContent>
              </Select>
            </Field>

            <Field label={m['billing.field_validity_days']()}>
              <Input
                type="number"
                min={1}
                value={row.validityDays ?? ''}
                onChange={(e) =>
                  onChange({
                    validityDays: e.target.value === '' ? null : Number(e.target.value),
                  })
                }
                disabled={pointsDisabled}
                placeholder="30"
              />
            </Field>

            <Field label={m['billing.field_max_periods']()}>
              <Input
                type="number"
                min={1}
                value={row.maxPeriods ?? ''}
                onChange={(e) =>
                  onChange({
                    maxPeriods: e.target.value === '' ? null : Number(e.target.value),
                  })
                }
                disabled={pointsDisabled}
                placeholder="12"
              />
            </Field>

            <Field label={m['billing.field_grant_on_subscribe']()}>
              <div className="flex items-center gap-2">
                <Switch
                  checked={row.grantOnSubscribe ?? false}
                  onCheckedChange={(checked: boolean) => onChange({ grantOnSubscribe: checked })}
                  disabled={pointsDisabled}
                />
              </div>
            </Field>
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  )
}

function Field({
  label,
  required,
  children,
}: {
  label: string
  required?: boolean
  children: React.ReactNode
}) {
  return (
    <div className="space-y-1">
      <Label className="text-xs font-medium text-muted-foreground">
        {label}
        {required && <span className="ml-0.5 text-destructive">*</span>}
      </Label>
      {children}
    </div>
  )
}

// ==================== Empty state ====================

function EmptyState({ onSync, canTriggerSync }: { onSync: () => void; canTriggerSync: boolean }) {
  return (
    <Card className="border-dashed" data-testid="entitlement-mappings-empty-state">
      <CardContent className="flex flex-col items-center justify-center py-12">
        <Plug2 className="mb-4 h-12 w-12 text-muted-foreground" />
        <p className="text-center text-sm font-medium text-muted-foreground">
          {m['billing.empty_state_title']()}
        </p>
        <p className="mt-1 text-center text-sm text-muted-foreground">
          {m['billing.empty_state_body']()}
        </p>
        {canTriggerSync && (
          <Button onClick={onSync} className="mt-4" data-testid="empty-sync-button">
            {m['billing.empty_sync_button']()}
          </Button>
        )}
      </CardContent>
    </Card>
  )
}

function LoadingSkeleton() {
  return (
    <div className="grid grid-cols-1 gap-6 lg:grid-cols-[320px_1fr]">
      <Card>
        <CardHeader>
          <Skeleton className="h-6 w-32" />
        </CardHeader>
        <CardContent className="space-y-3">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-10 w-full" />
          ))}
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <Skeleton className="h-6 w-48" />
        </CardHeader>
        <CardContent className="space-y-4">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-24 w-full" />
          ))}
        </CardContent>
      </Card>
    </div>
  )
}

// ==================== helpers ====================

function seedRows(prices: EntitlementMappingResponse[]): PriceMappingUpdateFormData[] {
  return prices.map((p) => ({
    mappingId: p.id,
    entitlementKey: p.entitlementKey,
    billingType: p.billingType ?? null,
    billingPeriod: p.billingPeriod ?? null,
    enabled: p.enabled,
    pointsPerPeriod: p.pointsPerPeriod ?? null,
    grantPeriodType: (p.grantPeriodType as 'once' | 'daily' | 'weekly' | 'monthly' | null) ?? null,
    validityDays: p.validityDays ?? null,
    grantOnSubscribe: p.grantOnSubscribe,
    maxPeriods: p.maxPeriods ?? null,
  }))
}

function toPriceMappingUpdate(row: PriceMappingUpdateFormData): PriceMappingUpdate {
  return {
    mappingId: row.mappingId,
    entitlementKey: row.entitlementKey,
    billingType: row.billingType ?? undefined,
    billingPeriod: row.billingPeriod ?? undefined,
    enabled: row.enabled ?? undefined,
    pointsPerPeriod: row.pointsPerPeriod ?? undefined,
    grantPeriodType: row.grantPeriodType ?? undefined,
    validityDays: row.validityDays ?? undefined,
    grantOnSubscribe: row.grantOnSubscribe ?? undefined,
    maxPeriods: row.maxPeriods ?? undefined,
  }
}

function latestSyncedAt(prices: EntitlementMappingResponse[]): string | null {
  let best: string | null = null
  for (const p of prices) {
    if (p.syncedAt) {
      if (best === null || p.syncedAt > best) best = p.syncedAt
    }
  }
  return best
}
