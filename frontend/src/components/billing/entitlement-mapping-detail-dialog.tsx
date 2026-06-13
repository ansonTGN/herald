import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { format } from 'date-fns'
import { formatInvoiceAmount, extractProviderPrice } from '@/lib/invoice-utils'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { entitlementMappingQueryOptions } from '@/data/query-options'
import { useUpdateEntitlementMapping } from '@/data/entitlement-mapping-mutations'
import {
  entitlementMappingUpdateSchema,
  getEntitlementMappingUpdateDefaults,
} from '@/lib/schemas/billing-forms'
import type { EntitlementMappingResponse } from '@/lib/api-generated'

interface EntitlementMappingDetailDialogProps {
  realmId: string
  mappingId: string | null
  open: boolean
  onOpenChange: (open: boolean) => void
}

function formatProviderName(provider: string): string {
  const names: Record<string, string> = {
    stripe: 'Stripe',
    creem: 'Creem',
    wechat: 'WeChat Pay',
    shopify: 'Shopify',
  }
  return names[provider] ?? provider
}

function getProviderProductName(providerProductInfo: unknown): string | undefined {
  if (!providerProductInfo || typeof providerProductInfo !== 'object') return undefined
  const info = providerProductInfo as Record<string, unknown>
  return typeof info.name === 'string' ? info.name : undefined
}

export function EntitlementMappingDetailDialog({
  realmId,
  mappingId,
  open,
  onOpenChange,
}: EntitlementMappingDetailDialogProps) {
  const { data: mapping, isLoading } = useQuery({
    ...entitlementMappingQueryOptions(realmId, mappingId ?? ''),
    enabled: open && !!mappingId,
    select: (data) => data as EntitlementMappingResponse | undefined,
  })

  const updateMutation = useUpdateEntitlementMapping(realmId, mappingId ?? '')

  const formDefaults = useMemo(
    () =>
      mapping
        ? getEntitlementMappingUpdateDefaults({
            entitlementKey: mapping.entitlementKey,
            enabled: mapping.enabled,
            pointsPerPeriod: mapping.pointsPerPeriod ?? null,
            grantPeriodType: mapping.grantPeriodType as
              | 'once'
              | 'daily'
              | 'weekly'
              | 'monthly'
              | null,
            validityDays: mapping.validityDays ?? null,
            grantOnSubscribe: mapping.grantOnSubscribe,
            maxPeriods: mapping.maxPeriods ?? null,
          })
        : getEntitlementMappingUpdateDefaults(),
    [mapping]
  )

  const form = useAppForm({
    schema: entitlementMappingUpdateSchema,
    defaultValues: formDefaults,
    onSubmit: async ({ value }) => {
      await updateMutation.mutateAsync(value)
    },
  })

  if (!open) return null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="max-w-2xl max-h-[90vh] overflow-y-auto"
        data-testid="entitlement-mapping-detail-dialog"
      >
        <DialogHeader>
          {isLoading ? (
            <>
              <Skeleton className="h-6 w-48" />
              <Skeleton className="h-4 w-32" />
            </>
          ) : mapping ? (
            <>
              <DialogTitle>Entitlement Mapping Detail</DialogTitle>
              <DialogDescription>
                View provider info and configure entitlement settings.
              </DialogDescription>
            </>
          ) : (
            <>
              <DialogTitle>Mapping Not Found</DialogTitle>
              <DialogDescription>The requested mapping could not be loaded.</DialogDescription>
            </>
          )}
        </DialogHeader>

        {isLoading ? (
          <LoadingSkeleton />
        ) : mapping ? (
          <div className="space-y-6">
            {/* Read-only section */}
            <div className="space-y-4">
              <h3 className="text-sm font-semibold text-muted-foreground">Provider Information</h3>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <p className="text-xs font-medium text-muted-foreground">Payment Provider</p>
                  <p className="text-sm">{formatProviderName(mapping.paymentProvider)}</p>
                </div>
                <div>
                  <p className="text-xs font-medium text-muted-foreground">External Product ID</p>
                  <p className="text-sm font-mono">{mapping.externalProductId}</p>
                </div>
                <div>
                  <p className="text-xs font-medium text-muted-foreground">External Price ID</p>
                  <p className="text-sm font-mono">{mapping.externalPriceId ?? '---'}</p>
                </div>
                <div>
                  <p className="text-xs font-medium text-muted-foreground">Synced At</p>
                  <p className="text-sm">
                    {mapping.syncedAt ? format(new Date(mapping.syncedAt), 'PPp') : '---'}
                  </p>
                </div>
              </div>

              {mapping.providerProductInfo != null &&
                typeof mapping.providerProductInfo === 'object' && (
                  <ProviderProductInfoCard info={mapping.providerProductInfo} />
                )}
            </div>

            <hr className="border-t" />

            {/* Editable section */}
            <form
              onSubmit={(e) => {
                e.preventDefault()
                form.handleSubmit()
              }}
            >
              <AppForm>
                <div className="space-y-4">
                  <h3 className="text-sm font-semibold text-muted-foreground">
                    Entitlement Configuration
                  </h3>

                  {/* Entitlement Key */}
                  <form.Field name="entitlementKey">
                    {(field) => (
                      <div className="space-y-2">
                        <Label htmlFor={field.name}>Entitlement Key *</Label>
                        <Input
                          id={field.name}
                          value={field.state.value}
                          onBlur={field.handleBlur}
                          onChange={(e) => field.handleChange(e.target.value)}
                          placeholder="e.g. pro-plan"
                          data-testid="entitlement-key-input"
                        />
                        {(field.state.meta.isTouched || form.state.isSubmitted) &&
                          field.state.meta.errors.length > 0 && (
                            <p className="text-sm text-destructive" role="alert">
                              {String(field.state.meta.errors[0])}
                            </p>
                          )}
                      </div>
                    )}
                  </form.Field>

                  {/* Subscription Points Per Period */}
                  <form.Field name="pointsPerPeriod">
                    {(field) => (
                      <div className="space-y-2">
                        <Label htmlFor={field.name}>Subscription Points Per Period</Label>
                        <Input
                          id={field.name}
                          type="number"
                          min="0"
                          value={field.state.value ?? ''}
                          onBlur={field.handleBlur}
                          onChange={(e) =>
                            field.handleChange(e.target.value ? Number(e.target.value) : null)
                          }
                          placeholder="1000"
                          data-testid="points-per-period-input"
                        />
                      </div>
                    )}
                  </form.Field>

                  {/* Grant Period Type */}
                  <form.Field name="grantPeriodType">
                    {(field) => (
                      <div className="space-y-2">
                        <Label htmlFor={field.name}>Grant Period Type</Label>
                        <Select
                          value={field.state.value ?? ''}
                          onValueChange={(value) =>
                            field.handleChange(
                              (value || null) as 'once' | 'daily' | 'weekly' | 'monthly' | null
                            )
                          }
                        >
                          <SelectTrigger id={field.name} data-testid="grant-period-type-select">
                            <SelectValue placeholder="Select period type" />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="once">Once</SelectItem>
                            <SelectItem value="daily">Daily</SelectItem>
                            <SelectItem value="weekly">Weekly</SelectItem>
                            <SelectItem value="monthly">Monthly</SelectItem>
                          </SelectContent>
                        </Select>
                      </div>
                    )}
                  </form.Field>

                  {/* Validity Days */}
                  <form.Field name="validityDays">
                    {(field) => (
                      <div className="space-y-2">
                        <Label htmlFor={field.name}>Validity Days</Label>
                        <Input
                          id={field.name}
                          type="number"
                          min="1"
                          value={field.state.value ?? ''}
                          onBlur={field.handleBlur}
                          onChange={(e) =>
                            field.handleChange(e.target.value ? Number(e.target.value) : null)
                          }
                          placeholder="30"
                          data-testid="validity-days-input"
                        />
                      </div>
                    )}
                  </form.Field>

                  {/* Grant On Subscribe */}
                  <form.Field name="grantOnSubscribe">
                    {(field) => (
                      <div className="flex items-center justify-between">
                        <div className="space-y-0.5">
                          <Label htmlFor="grant-on-subscribe">Grant On Subscribe</Label>
                          <p className="text-sm text-muted-foreground">
                            Grant points on first subscription
                          </p>
                        </div>
                        <Switch
                          id="grant-on-subscribe"
                          checked={field.state.value}
                          onCheckedChange={(checked: boolean) => field.handleChange(checked)}
                          data-testid="grant-on-subscribe-switch"
                        />
                      </div>
                    )}
                  </form.Field>

                  {/* Max Periods */}
                  <form.Field name="maxPeriods">
                    {(field) => (
                      <div className="space-y-2">
                        <Label htmlFor={field.name}>Max Periods</Label>
                        <Input
                          id={field.name}
                          type="number"
                          min="1"
                          value={field.state.value ?? ''}
                          onBlur={field.handleBlur}
                          onChange={(e) =>
                            field.handleChange(e.target.value ? Number(e.target.value) : null)
                          }
                          placeholder="12"
                          data-testid="max-periods-input"
                        />
                      </div>
                    )}
                  </form.Field>

                  {/* Enabled */}
                  <form.Field name="enabled">
                    {(field) => (
                      <div className="flex items-center justify-between">
                        <div className="space-y-0.5">
                          <Label htmlFor="mapping-enabled">Enabled</Label>
                          <p className="text-sm text-muted-foreground">
                            Enable this mapping for subscription processing
                          </p>
                        </div>
                        <Switch
                          id="mapping-enabled"
                          checked={field.state.value}
                          onCheckedChange={(checked: boolean) => field.handleChange(checked)}
                          data-testid="mapping-enabled-switch"
                        />
                      </div>
                    )}
                  </form.Field>
                </div>
              </AppForm>
            </form>
          </div>
        ) : null}

        <DialogFooter showCloseButton>
          {mapping && (
            <Button
              type="submit"
              disabled={updateMutation.isPending}
              onClick={() => form.handleSubmit()}
              data-testid="save-mapping-button"
            >
              {updateMutation.isPending ? 'Saving...' : 'Save Changes'}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function ProviderProductInfoCard({ info }: { info: unknown }) {
  const name = getProviderProductName(info)
  const priceInfo = extractProviderPrice(info)
  if (!name && !priceInfo) return null

  return (
    <div className="rounded-md border p-3 space-y-1" data-testid="provider-product-info-card">
      <p className="text-xs font-medium text-muted-foreground">Provider Product Info</p>
      <div className="flex gap-4 text-sm">
        {name && <span>{name}</span>}
        {priceInfo && (
          <span className="font-mono">
            {formatInvoiceAmount(priceInfo.amount, priceInfo.currency)}
          </span>
        )}
      </div>
    </div>
  )
}

function LoadingSkeleton() {
  return (
    <div className="space-y-6">
      <div className="grid grid-cols-2 gap-4">
        <Skeleton className="h-12 w-full" />
        <Skeleton className="h-12 w-full" />
        <Skeleton className="h-12 w-full" />
        <Skeleton className="h-12 w-full" />
      </div>
      <Skeleton className="h-px w-full" />
      <div className="space-y-4">
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-10 w-full" />
      </div>
    </div>
  )
}
