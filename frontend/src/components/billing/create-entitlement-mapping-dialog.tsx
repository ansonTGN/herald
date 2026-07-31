import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { RoleSelector } from '@/components/shared/role-selector'
import { PointDistributionRuleEditor } from '@/components/billing/point-distribution-rule-editor'
import { formatProviderName } from '@/components/billing/format-provider-name'
import { pointRuleTriggersForBillingType } from '@/components/billing/provider-product-info'
import { creditBucketsListQueryOptions, adminRolesQueryOptions } from '@/data/query-options'
import {
  useCreateEntitlementMapping,
  isCreateMappingDuplicateError,
  isCreateMappingConfigError,
} from '@/data/entitlement-mapping-mutations'
import {
  createEntitlementMappingSchema,
  getCreateEntitlementMappingDefaults,
  type CreateEntitlementMappingFormData,
} from '@/lib/schemas/create-entitlement-mapping'
import { PAYMENT_PROVIDERS, BILLING_PERIODS } from '@/lib/billing-constants'
import type { CreateEntitlementMappingRequest } from '@/lib/api-generated'
import { m } from '@/paraglide/messages'

interface CreateEntitlementMappingDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  realmId: string
  /** Whether the caller may edit credit-strategy fields (points.manage). */
  canManagePoints: boolean
}

/**
 * Shared label + required marker + field-error scaffolding for the dialog's
 * standard fields. The dialog drives its inputs with local `useState` (not a
 * tanstack form instance), so it can't reuse the shared `TextField`/`PasswordField`
 * wrappers — this local wrapper centralizes the markup those fields repeat.
 */
function Field({
  label,
  htmlFor,
  required = false,
  error,
  children,
}: {
  label: string
  htmlFor?: string
  required?: boolean
  error?: string
  children: ReactNode
}) {
  return (
    <div className="space-y-1.5">
      <Label htmlFor={htmlFor}>
        {label}
        {required && <span className="ml-0.5 text-destructive">*</span>}
      </Label>
      {children}
      {error && (
        <p className="text-xs text-destructive" role="alert">
          {error}
        </p>
      )}
    </div>
  )
}

/**
 *
 * Hand-builds a single entitlement mapping and POSTs it via
 * `useCreateEntitlementMapping`. On success it closes itself (the mutation
 * already invalidates `['entitlement-mappings']` so the list refreshes).
 *
 * - 409 → `billing.create_mapping_duplicate` ("product id already exists").
 * - 23514 / non-4xx → `billing.create_mapping_config_error`
 *   ("configuration error"; DB CHECK / server defense).
 * - other (incl. 400/401/403) → generic toast owned by the mutation.
 */
export function CreateEntitlementMappingDialog({
  open,
  onOpenChange,
  realmId,
  canManagePoints,
}: CreateEntitlementMappingDialogProps) {
  const createMutation = useCreateEntitlementMapping(realmId)

  const [values, setValues] = useState<CreateEntitlementMappingFormData>(
    getCreateEntitlementMappingDefaults
  )
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({})
  const [submitError, setSubmitError] = useState<string | null>(null)

  // Reset the form whenever the dialog opens (fresh create each time).
  useEffect(() => {
    if (open) {
      setValues(getCreateEntitlementMappingDefaults())
      setFieldErrors({})
      setSubmitError(null)
    }
  }, [open])

  const { data: buckets } = useQuery(creditBucketsListQueryOptions(realmId))
  const { data: roles } = useQuery(adminRolesQueryOptions(realmId))
  const assignableRoles = useMemo(() => (roles ?? []).filter((r) => !r.isBuiltin), [roles])

  const isRecurring = values.billingType === 'recurring'
  const isOneTime = values.billingType === 'one_time'
  const isNonRenewing = values.billingType === 'non_renewing'

  const update = <K extends keyof CreateEntitlementMappingFormData>(
    key: K,
    next: CreateEntitlementMappingFormData[K]
  ) => {
    setValues((prev) => ({ ...prev, [key]: next }))
    // Clear a field-level error as soon as the field is edited.
    setFieldErrors((prev) => {
      if (!prev[key as string]) return prev
      const { [key as string]: _removed, ...rest } = prev
      return rest
    })
  }

  const handleSubmit = () => {
    const parsed = createEntitlementMappingSchema.safeParse(values)
    if (!parsed.success) {
      const errors: Record<string, string> = {}
      for (const issue of parsed.error.issues) {
        const path = issue.path[0]
        if (typeof path === 'string' && !errors[path]) {
          errors[path] = issue.message
        }
      }
      setFieldErrors(errors)
      return
    }

    const v = parsed.data
    const body: CreateEntitlementMappingRequest = {
      paymentProvider: v.paymentProvider,
      externalProductId: v.externalProductId,
      entitlementKey: v.entitlementKey,
      billingType: v.billingType,
      // externalPriceId: optional; IAP/Creem leave it empty.
      externalPriceId: v.externalPriceId ? v.externalPriceId : null,
      billingPeriod: isRecurring ? (v.billingPeriod ?? null) : null,
      serviceDurationDays: isNonRenewing ? (v.serviceDurationDays ?? null) : null,
      ...(canManagePoints ? { pointRules: v.pointRules } : {}),
      grantedRoleIds: v.grantedRoleIds,
      enabled: v.enabled,
    }

    createMutation.mutate(body, {
      onSuccess: () => {
        toast.success(m['billing.create_mapping_success']())
        onOpenChange(false)
      },
      onError: (error) => {
        // 409 duplicate: the only fix is editing the inputs (provider/product/
        // price), so surface on the externalProductId field AND as a toast.
        if (isCreateMappingDuplicateError(error)) {
          const msg = m['billing.create_mapping_duplicate']()
          setSubmitError(msg)
          toast.error(msg)
          return
        }
        // 23514 / non-4xx: DB CHECK / server defense — a configuration error,
        if (isCreateMappingConfigError(error)) {
          const msg = m['billing.create_mapping_config_error']()
          setSubmitError(msg)
          toast.error(msg)
          return
        }
        // Other errors (400 validation, 401/403 authz) are toasted by the
        // mutation's generic onError. Surface inline too for visibility.
        setSubmitError(m['billing.create_mapping_failed']())
      },
    })
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl" data-testid="create-entitlement-mapping-dialog">
        <DialogHeader>
          <DialogTitle>{m['billing.create_mapping_title']()}</DialogTitle>
          <DialogDescription>{m['billing.create_mapping_description']()}</DialogDescription>
        </DialogHeader>

        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <Field
            label={m['billing.create_mapping_provider']()}
            htmlFor="create-mapping-provider"
            required
            error={fieldErrors.paymentProvider}
          >
            <Select
              value={values.paymentProvider ?? ''}
              onValueChange={(v) => update('paymentProvider', v)}
            >
              <SelectTrigger
                id="create-mapping-provider"
                data-testid="create-mapping-provider-select"
              >
                <SelectValue placeholder={m['billing.create_mapping_provider_placeholder']()} />
              </SelectTrigger>
              <SelectContent>
                {Object.values(PAYMENT_PROVIDERS).map((p) => (
                  <SelectItem key={p} value={p}>
                    {formatProviderName(p)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>

          <Field
            label={m['billing.create_mapping_external_product_id']()}
            htmlFor="create-mapping-external-product-id"
            required
            error={fieldErrors.externalProductId}
          >
            <Input
              id="create-mapping-external-product-id"
              value={values.externalProductId}
              onChange={(e) => update('externalProductId', e.target.value)}
              data-testid="create-mapping-external-product-id-input"
            />
          </Field>

          {/* External Price ID (optional; Stripe only) */}
          <Field
            label={m['billing.create_mapping_external_price_id']()}
            htmlFor="create-mapping-external-price-id"
          >
            <Input
              id="create-mapping-external-price-id"
              value={values.externalPriceId ?? ''}
              onChange={(e) =>
                update('externalPriceId', e.target.value === '' ? null : e.target.value)
              }
              data-testid="create-mapping-external-price-id-input"
            />
          </Field>

          <Field
            label={m['billing.create_mapping_entitlement_key']()}
            htmlFor="create-mapping-entitlement-key"
            required
            error={fieldErrors.entitlementKey}
          >
            <Input
              id="create-mapping-entitlement-key"
              value={values.entitlementKey}
              onChange={(e) => update('entitlementKey', e.target.value)}
              data-testid="create-mapping-entitlement-key-input"
            />
          </Field>

          <Field
            label={m['billing.create_mapping_billing_type']()}
            htmlFor="create-mapping-billing-type"
            required
            error={fieldErrors.billingType}
          >
            <Select
              value={values.billingType ?? ''}
              onValueChange={(v) =>
                update('billingType', v as 'recurring' | 'one_time' | 'non_renewing')
              }
            >
              <SelectTrigger
                id="create-mapping-billing-type"
                data-testid="create-mapping-billing-type-select"
              >
                <SelectValue placeholder={m['billing.create_mapping_billing_type_placeholder']()} />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="recurring">{m['billing.billing_type_recurring']()}</SelectItem>
                <SelectItem value="one_time">{m['billing.billing_type_one_time']()}</SelectItem>
                <SelectItem value="non_renewing">
                  {m['billing.billing_type_non_renewing']()}
                </SelectItem>
              </SelectContent>
            </Select>
          </Field>

          {isRecurring && (
            <Field
              label={m['billing.create_mapping_billing_period']()}
              htmlFor="create-mapping-billing-period"
              required
              error={fieldErrors.billingPeriod}
            >
              <Select
                value={values.billingPeriod ?? ''}
                onValueChange={(v) => update('billingPeriod', v)}
              >
                <SelectTrigger
                  id="create-mapping-billing-period"
                  data-testid="create-mapping-billing-period-select"
                >
                  <SelectValue
                    placeholder={m['billing.create_mapping_billing_period_placeholder']()}
                  />
                </SelectTrigger>
                <SelectContent>
                  {Object.values(BILLING_PERIODS).map((p) => (
                    <SelectItem key={p} value={p}>
                      {p === BILLING_PERIODS.MONTHLY
                        ? m['billing.billing_period_month']()
                        : m['billing.billing_period_year']()}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
          )}

          {isNonRenewing && (
            <Field
              label={m['billing.create_mapping_service_duration_days']()}
              htmlFor="create-mapping-service-duration-days"
              required
              error={fieldErrors.serviceDurationDays}
            >
              <Input
                id="create-mapping-service-duration-days"
                type="number"
                min={1}
                value={values.serviceDurationDays ?? ''}
                onChange={(e) =>
                  update(
                    'serviceDurationDays',
                    e.target.value === '' ? null : Number(e.target.value)
                  )
                }
                data-testid="create-mapping-service-duration-days-input"
              />
            </Field>
          )}

          <div className="space-y-1.5 sm:col-span-2" data-testid="create-mapping-granted-roles">
            <Label>{m['billing.create_mapping_granted_roles']()}</Label>
            <RoleSelector
              roles={assignableRoles}
              selectedRoleIds={values.grantedRoleIds ?? []}
              onChange={(ids) => update('grantedRoleIds', ids)}
              placeholder={m['billing.help_granted_roles']()}
            />
          </div>

          {canManagePoints && values.billingType && (
            <div className="space-y-2 sm:col-span-2">
              <Label>Points distribution rules</Label>
              <PointDistributionRuleEditor
                value={values.pointRules}
                onChange={(pointRules) => update('pointRules', pointRules)}
                buckets={buckets ?? []}
                triggers={pointRuleTriggersForBillingType(values.billingType)}
                allowQuota={!isOneTime}
              />
            </div>
          )}
        </div>

        {submitError && (
          <p
            className="text-sm text-destructive"
            role="alert"
            data-testid="create-mapping-submit-error"
          >
            {submitError}
          </p>
        )}

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            data-testid="create-mapping-cancel-button"
          >
            {m['billing.create_mapping_cancel']()}
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={createMutation.isPending}
            data-testid="create-mapping-submit-button"
          >
            {createMutation.isPending
              ? m['billing.create_mapping_submitting']()
              : m['billing.create_mapping_submit']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
