import { FileText } from 'lucide-react'
import { useQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { invoiceApplyEligibilityQueryOptions } from '@/data/invoice-query-options'
import { getProviderLabel } from '@/lib/invoice-utils'
import { m } from '@/paraglide/messages'

type ReferenceType = 'payment_attempt' | 'subscription'

interface InvoiceApplyRowButtonProps {
  realmId: string
  referenceType: ReferenceType
  referenceId: string
  /**
   * Called only when the resource is eligible for a manual Herald invoice
   * (route === 'manual_fallback', canApply === true).
   */
  onApply: () => void
  /** Label shown next to the FileText icon (omitted for icon-only buttons). */
  label?: string
  /** Screen-reader-only label. */
  srLabel?: string
  /** Variant for the enabled (manual_fallback) button. */
  variant?: 'ghost' | 'outline'
  /** Test id prefix; the disabled reason span uses `${prefix}-reason`. */
  testIdPrefix: string
}

/**
 * Per-row Invoice button that reflects the apply-eligibility verdict BEFORE
 * submit (P1-4):
 *  - `manual_fallback` (`canApply=true`): enabled; clicking invokes onApply.
 *  - `disabled`: disabled; surfaces the backend `reason` inline.
 *  - `external_provider`: disabled; surfaces "Managed by {provider} — see My Invoices".
 *
 * The eligibility query fires per rendered row (lists are paginated/small);
 * react-query dedupes/caches by [realmId, referenceType, referenceId]. Callers
 * gate rendering on realm-level `invoicesVisible` (outer gate) and only mount
 * this button when that gate is open, so the query stays off otherwise.
 */
export function InvoiceApplyRowButton({
  realmId,
  referenceType,
  referenceId,
  onApply,
  label,
  srLabel,
  variant = 'ghost',
  testIdPrefix,
}: InvoiceApplyRowButtonProps) {
  const { data: eligibility } = useQuery(
    invoiceApplyEligibilityQueryOptions(realmId, referenceType, referenceId)
  )

  const route = eligibility?.route
  const disabled = !eligibility || !(route === 'manual_fallback' && eligibility.canApply === true)

  let disabledReason: string | undefined
  if (eligibility && disabled) {
    if (route === 'external_provider') {
      disabledReason = m['billing.invoice_apply_managed_external']({
        provider: getProviderLabel(eligibility.provider ?? ''),
      })
    } else {
      // `disabled` route: prefer backend reason, fall back to contact-admin.
      disabledReason = eligibility.reason ?? m['billing.invoice_apply_disabled_contact_admin']()
    }
  }

  return (
    <div className="flex flex-col items-start gap-0.5">
      <Button
        type="button"
        variant={variant}
        size="sm"
        disabled={disabled}
        onClick={onApply}
        data-testid={testIdPrefix}
      >
        <FileText className={variant === 'ghost' ? 'h-4 w-4' : 'mr-2 h-4 w-4'} />
        {label}
        {srLabel && <span className="sr-only">{srLabel}</span>}
      </Button>
      {disabledReason && (
        <span className="text-xs text-muted-foreground" data-testid={`${testIdPrefix}-reason`}>
          {disabledReason}
        </span>
      )}
    </div>
  )
}
