import { useMemo, useState } from 'react'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent } from '@/components/ui/card'
import { Check } from 'lucide-react'
import type { PurchaseOptionView } from '@/lib/api-generated'
import { m } from '@/paraglide/messages'
import { formatInvoiceAmount } from '@/lib/invoice-utils'
import { deriveSharedKeyColor } from '@/components/billing/shared-key-color'
import {
  isCurrencySwitchable,
  isPreferredCurrencyUnavailable,
  resolveEffectivePreferredCurrency,
  resolveHighlightedCurrency,
  type EntitlementOptionGroup,
} from '@/components/billing/currency-utils'

/**
 * Reason a price card is not purchasable, or null when it is purchasable.
 *
 * A card is disabled when the mapping is not enabled for purchase, or when no
 * payment provider is wired to it (the price exists but cannot be checked out).
 * A gated one-time+role card is also disabled when the current user already
 * owns it — `grantsRole` is true only for `one_time` + non-empty
 * `granted_role_ids` (points/subscriptions are never gated), so the already-owned
 * branch is naturally scoped without the frontend re-checking billing_type.
 * Returns the matching message key so the caller renders the canonical copy via
 * Paraglide; returns null for purchasable cards so the caller can skip rendering
 * a reason row.
 */
// eslint-disable-next-line react-refresh/only-export-components -- exported for unit testing
export function disabledReason(
  option: PurchaseOptionView
): { key: 'purchase.not_enabled_reason' | 'purchase.already_owned_reason' } | null {
  if (!option.enabled || !option.paymentProvider) {
    return { key: 'purchase.not_enabled_reason' }
  }
  if (option.grantsRole && option.alreadyOwned) {
    return { key: 'purchase.already_owned_reason' }
  }
  return null
}

export function PriceCard({
  option,
  isSelected,
  onSelect,
}: {
  option: PurchaseOptionView
  isSelected: boolean
  onSelect: () => void
}) {
  const reason = disabledReason(option)
  const isDisabled = reason !== null
  const color = deriveSharedKeyColor(option.entitlementKey)
  // priceId falls back to mappingId for price-less providers (Creem) so the
  // testid is always stable and non-empty.
  const priceId = option.externalPriceId ?? option.mappingId

  // Billing-type badge + period suffix. one_time renders an "One-time" badge +
  // `once` suffix; recurring renders "Subscription" + a period suffix derived
  // from billingPeriod.
  const isOneTime = option.billingType !== 'recurring'
  const periodSuffixKey = isOneTime
    ? 'purchase.period_suffix_once'
    : option.billingPeriod === 'year'
      ? 'purchase.period_suffix_year'
      : 'purchase.period_suffix_month'

  return (
    <Card
      className={`cursor-pointer transition-all ${
        isSelected
          ? 'border-primary ring-2 ring-primary'
          : 'border-muted-foreground/25 hover:border-muted-foreground/50'
      } ${isDisabled ? 'opacity-60' : ''}`}
      onClick={isDisabled ? undefined : onSelect}
      data-testid={`purchase-price-card-${priceId}`}
    >
      <CardContent className="p-4">
        <div className="flex w-full items-start justify-between gap-3">
          <div className="flex-1 space-y-1">
            <div className="flex items-center gap-2">
              <span
                className="inline-block h-3 w-3 rounded-full"
                style={
                  color.hue !== 0 ? { backgroundColor: `hsl(${color.hue} 70% 50%)` } : undefined
                }
                aria-hidden
              />
              <div className="font-medium">{option.displayName || option.entitlementKey}</div>
            </div>
            <Badge variant="secondary" data-testid={`price-card-billing-type-${priceId}`}>
              {isOneTime
                ? m['purchase.billing_type_one_time']()
                : m['purchase.billing_type_subscription']()}
            </Badge>
            {option.pointRules.length > 0 && (
              <div className="space-y-1 text-sm text-muted-foreground">
                {option.pointRules.map((rule) => (
                  <div
                    key={rule.id}
                    data-testid={`purchase-point-rule-${rule.id}`}
                    className="rounded border px-2 py-1"
                  >
                    <span className="font-mono text-xs">{rule.bucketId}</span>
                    {rule.grantMode === 'fixed' ? (
                      <span> · {rule.pointsAmount?.toLocaleString() ?? 0} points</span>
                    ) : (
                      <span>
                        {' '}
                        ·{' '}
                        {(rule.quotaWindows ?? [])
                          .map(
                            (window) =>
                              `${window.limit.toLocaleString()} / ${window.windowSeconds}s`
                          )
                          .join(', ')}
                      </span>
                    )}
                  </div>
                ))}
              </div>
            )}
            {option.amount != null && option.currency ? (
              <div className="text-sm font-medium">
                {formatInvoiceAmount(option.amount, option.currency)}{' '}
                <span className="text-muted-foreground">{m[periodSuffixKey]()}</span>
              </div>
            ) : (
              <div className="text-sm text-muted-foreground">{m['purchase.unavailable']()}</div>
            )}
            {isDisabled && reason && (
              <div
                className="text-xs text-muted-foreground"
                data-testid={`purchase-price-card-${priceId}-reason`}
              >
                {m[reason.key]()}
              </div>
            )}
          </div>
          {isSelected && (
            <div className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-primary">
              <Check className="h-4 w-4 text-primary-foreground" />
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  )
}

/** Kebab-case slug of an entitlement key, for per-entitlement testids. */
function entitlementSlug(entitlementKey: string): string {
  return entitlementKey.toLowerCase().replace(/[^a-z0-9]+/g, '-')
}

/**
 * One entitlement's purchase block: a currency switcher plus the price rows of
 * the active currency. The switcher only renders when every row is
 * Stripe-priced and spans multiple currencies; store-priced providers (Creem /
 * Apple / Google / WeChat) and single-currency products degrade to the plain
 * price list with no switcher, and the preferred currency never filters what
 * the user may buy — it only picks the initially highlighted currency.
 */
export function CurrencyPurchaseGroup({
  group,
  userPreferredCurrency,
  realmDefaultCurrency,
  selectedMappingId,
  onSelect,
}: {
  group: EntitlementOptionGroup
  userPreferredCurrency: string | null | undefined
  realmDefaultCurrency: string | null | undefined
  selectedMappingId: string | null
  onSelect: (mappingId: string) => void
}) {
  const slug = entitlementSlug(group.entitlementKey)
  const switchable = isCurrencySwitchable(group)

  // Highlight chain: user preferred → realm default → first available. A
  // manual pick overrides it until the picked currency disappears from the
  // refreshed options, at which point the chain takes over again.
  const highlightedCurrency = useMemo(
    () => resolveHighlightedCurrency(group.currencies, userPreferredCurrency, realmDefaultCurrency),
    [group.currencies, userPreferredCurrency, realmDefaultCurrency]
  )
  const [manualCurrency, setManualCurrency] = useState<string | null>(null)
  const activeCurrency =
    manualCurrency && group.currencies.includes(manualCurrency)
      ? manualCurrency
      : (highlightedCurrency ?? group.currencies[0])

  // Switchable groups render only the active currency's rows; degraded groups
  // render everything (single currency or store-priced rows carry no switch).
  const visibleOptions = switchable
    ? (group.currencyGroups.find((g) => g.currency === activeCurrency)?.options ?? [])
    : group.options

  const effectivePreferred = resolveEffectivePreferredCurrency(
    userPreferredCurrency,
    realmDefaultCurrency
  )
  const preferredUnavailable =
    switchable &&
    isPreferredCurrencyUnavailable(group.currencies, userPreferredCurrency, realmDefaultCurrency)

  return (
    <div className="space-y-3" data-testid={`purchase-entitlement-${slug}`}>
      {switchable && (
        <div
          className="flex flex-wrap items-center gap-2"
          data-testid={`purchase-currency-switch-${slug}`}
        >
          {group.currencies.map((currency) => (
            <button
              key={currency}
              type="button"
              onClick={() => setManualCurrency(currency)}
              data-testid={`purchase-currency-option-${slug}-${currency.toLowerCase()}`}
              className={`rounded-md border px-3 py-1 text-sm font-medium transition-colors ${
                currency === activeCurrency
                  ? 'border-primary bg-primary text-primary-foreground'
                  : 'border-muted-foreground/25 text-muted-foreground hover:border-muted-foreground/50'
              }`}
            >
              {currency}
            </button>
          ))}
          {/* Base-currency annotation: with Stripe Adaptive Pricing enabled the
              checkout page may convert the charge, so the displayed (base)
              currency is not a promise of the final charge currency. */}
          <span
            className="text-xs text-muted-foreground"
            data-testid={`purchase-adaptive-pricing-note-${slug}`}
          >
            {m['purchase.adaptive_pricing_note']()}
          </span>
        </div>
      )}
      {preferredUnavailable && effectivePreferred && (
        <p
          className="text-sm text-muted-foreground"
          data-testid={`purchase-currency-preferred-unavailable-${slug}`}
        >
          {m['purchase.currency_preferred_unavailable']({ currency: effectivePreferred })}
        </p>
      )}
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
        {visibleOptions.map((option) => (
          <PriceCard
            key={option.mappingId}
            option={option}
            isSelected={selectedMappingId === option.mappingId}
            onSelect={() => onSelect(option.mappingId)}
          />
        ))}
      </div>
    </div>
  )
}
