import { m } from '@/paraglide/messages'

/**
 * Companion TS type for the `providerProductInfo` JSONB column.
 *
 * The generated OpenAPI client types `providerProductInfo` as `unknown`
 * (see `frontend/src/lib/api-generated/types.gen.ts`). The structured shape
 * below is the frontend lens over that JSONB and is layered OVER the generated
 * `unknown` — the generated files are NOT modified.
 *
 * Backend JSONB keys are snake_case; this accessor maps them to camelCase TS
 * fields (the ONLY place that narrowing happens):
 *   name            → name
 *   description     → description
 *   price           → price           (integer cents; unified /100 on display)
 *   currency        → currency
 *   billing_type    → billingType
 *   billing_period  → billingPeriod
 *   product_metadata→ productMetadata
 *   price_metadata  → priceMetadata
 *
 * This file does not re-export anything from the generated client.
 */

/**
 * Structured view of the `provider_product_info` JSONB written by the
 * backend stores the union of what each provider exposes and any subset may
 * be absent for a given product/price.
 */
export interface ProviderProductInfo {
  name?: string | null
  description?: string | null
  /** Integer cents (Stripe `unit_amount` / Creem price); display divides by 100. */
  price?: number | null
  currency?: string | null
  billingType?: string | null
  billingPeriod?: string | null
  productMetadata?: Record<string, string> | null
  priceMetadata?: Record<string, string> | null
}

/**
 * Narrow the generated `unknown` JSONB into the structured
 * {@link ProviderProductInfo} view. Defensive: returns `{}` for null/non-object
 * input and shallow-reads only the known keys, mapping the snake_case backend
 * JSONB keys to camelCase TS fields. Never throws on unknown shapes.
 *
 * This is the single narrowing point — everywhere else imports this accessor
 * and the type; do NOT re-narrow `unknown` elsewhere.
 */
export function readProviderProductInfo(raw: unknown): ProviderProductInfo {
  if (raw === null || typeof raw !== 'object') {
    return {}
  }
  const r = raw as Record<string, unknown>
  const pick = <T>(key: string): T | undefined => {
    const v = r[key]
    // Treat `null` as an explicit absent value (kept as undefined here; callers
    // see the field simply as missing). The TS type still allows `null` so the
    // shape stays honest for future consumers that read the raw JSON directly.
    return v === null ? undefined : (v as T | undefined)
  }
  return {
    name: pick<string>('name'),
    description: pick<string>('description'),
    price: pick<number>('price'),
    currency: pick<string>('currency'),
    billingType: pick<string>('billing_type'),
    billingPeriod: pick<string>('billing_period'),
    productMetadata: pick<Record<string, string>>('product_metadata'),
    priceMetadata: pick<Record<string, string>>('price_metadata'),
  }
}

/**
 * The primary display label for a product row / detail head: prefer the synced
 * product name, fall back to the external product id, and finally return `''`
 * when both are absent (the caller renders an i18n placeholder in that case).
 *
 * Pure + unit-testable. The page composes this with the localized placeholder.
 */
export function primaryProductLabel(
  productName?: string | null,
  externalProductId?: string | null
): string {
  return productName ?? externalProductId ?? ''
}

/**
 * Authoritative rule for whether a mapping is a one-time (non-subscription)
 * purchase. Used by both the {@link PriceEditRow} form (to hide the four
 * {@link toPriceMappingUpdate} batch-payload mapper (to null those fields out
 * so no stale seeded value leaks onto the wire).
 *
 * `billingType === 'one_time'` → `true` (one-time → hide/null fields).
 * `'recurring'` / `'non_renewing'` / `null` / `undefined` / anything else →
 * `false` (render the full recurring field set; this is the recurring default
 * `grant_on_subscribe && points_per_period > 0` emits SubscriptionCredit +
 * role grant), differing only in that it shows `serviceDurationDays` instead
 *
 * Pure + unit-testable. This is the single decision point; do not re-test
 * `billingType === 'one_time'` inline elsewhere in the mappings page.
 */
export function isOneTimeMapping(billingType?: string | null): boolean {
  return billingType === 'one_time'
}

/**
 * Authoritative rule for whether a mapping is a non-renewing (fixed-term)
 * subscription. Non-renewing shares recurring distribution-rule triggers
 * while also carrying a fixed service duration.
 *
 * `billingType === 'non_renewing'` → `true`.
 * Anything else → `false`.
 *
 * Pure + unit-testable. This is the single decision point; do not re-test
 * `billingType === 'non_renewing'` inline elsewhere in the mappings page —
 * use this helper so a future billing-type string change surfaces in one place.
 */
export function isNonRenewingMapping(billingType?: string | null): boolean {
  return billingType === 'non_renewing'
}

/**
 * A selectable point-rule trigger source (rendered as a checkbox label by
 * {@link PointDistributionRuleEditor}). Defined here, next to the billing-type
 * decision helpers below, so the trigger taxonomy has one home.
 */
export interface PointRuleTriggerOption {
  value: string
  label: string
}

/**
 * Localized display label for a point-rule trigger source value. The single
 * place trigger labels are translated; the option lists below reuse it. Must run
 * at render time (not module scope) so it honors the active locale.
 */
export function pointRuleTriggerLabel(value: string): string {
  switch (value) {
    case 'topup':
      return m['points.rule_editor_trigger_topup']()
    case 'subscription_initial':
      return m['points.rule_editor_trigger_subscription_initial']()
    case 'subscription_renewal':
      return m['points.rule_editor_trigger_subscription_renewal']()
    case 'subscription_upgrade':
      return m['points.rule_editor_trigger_subscription_upgrade']()
    case 'registration':
      return m['points.rule_editor_trigger_registration']()
    case 'free_periodic_grant':
      return m['points.rule_editor_trigger_free_periodic_grant']()
    default:
      return value
  }
}

// Legal point-rule trigger sources per mapping billing type. One-time purchases
// grant on `topup`; non-renewing subscriptions grant only on `subscription_initial`;
// recurring subscriptions grant on initial, renewal, and upgrade.
const TOPUP_POINT_RULE_TRIGGERS = ['topup']
const NON_RENEWING_POINT_RULE_TRIGGERS = ['subscription_initial']
const SUBSCRIPTION_POINT_RULE_TRIGGERS = [
  'subscription_initial',
  'subscription_renewal',
  'subscription_upgrade',
]

/**
 * Resolve the legal point-rule trigger options for a mapping's billing type.
 * The single decision point — callers pass the result straight to
 * {@link PointDistributionRuleEditor}'s `triggers` prop instead of re-deriving
 * the list inline. Labels are localized at call time (this runs during render).
 */
export function pointRuleTriggersForBillingType(
  billingType?: string | null
): PointRuleTriggerOption[] {
  const values = isOneTimeMapping(billingType)
    ? TOPUP_POINT_RULE_TRIGGERS
    : isNonRenewingMapping(billingType)
      ? NON_RENEWING_POINT_RULE_TRIGGERS
      : SUBSCRIPTION_POINT_RULE_TRIGGERS
  return values.map((value) => ({ value, label: pointRuleTriggerLabel(value) }))
}

/**
 * Map a provider `billing_period` (Creem/Stripe raw string) to its localized
 * display label. Known variants:
 *   - 'every-month' | 'month'  → localized "Month"
 *   - 'every-year'  | 'year'   → localized "Year"
 * Any other non-empty string is returned verbatim (graceful fallback for
 * provider-specific variants). Empty/null/undefined → `''` (caller renders
 * the `—` placeholder).
 *
 * Pure + unit-testable.
 */
export function mapBillingPeriodLabel(period: string | null | undefined): string {
  if (!period) return ''
  switch (period) {
    case 'every-month':
    case 'month':
      return m['billing.billing_period_month']()
    case 'every-year':
    case 'year':
      return m['billing.billing_period_year']()
    default:
      return period
  }
}
