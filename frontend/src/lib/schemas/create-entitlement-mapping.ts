import { z } from 'zod'
import { BILLING_PERIODS, providerRequiresManualPrice } from '@/lib/billing-constants'
import { isValidCurrencyCode } from '@/lib/currency-utils'
import { m } from '@/paraglide/messages'
import { pointDistributionRulesSchema } from '@/lib/schemas/billing-forms'

/**
 *
 * Kept as its own file (NOT merged into `billing-forms.ts`) because the field
 * set (paymentProvider / externalProductId / entitlementKey / bucketId /
 * billingType / billingPeriod / grantedRoleIds ...) is a different schema
 * family from the `priceMappingUpdateSchema` / `batchEntitlementMappingsSchema`
 * (batch-update, price-granularity PATCH) family that lives there. Merging
 * would conflate single-create vs batch-update semantics.
 *
 * - `billingType === 'recurring'` ⇒ `billingPeriod` is required (monthly/yearly).
 * - `billingType === 'one_time'` ⇒ `validityDays` is optional-but-fillable;
 *   `billingPeriod` is dropped on submit.
 * - `billingType === 'non_renewing'` ⇒ `serviceDurationDays` is required (≥1),
 *   and `billingPeriod` is mutually exclusive (rejected at the billingPeriod
 *   path so the message can name the conflict, not the recurring-required wording).
 *
 * `externalPriceId` is optional: IAP (apple/google) and Creem leave it empty;
 * Stripe requires it (the backend enforces — we don't hard-fail client-side to
 * keep the form permissive for the IAP primary case).
 */

const billingPeriodValues = Object.values(BILLING_PERIODS) as [string, ...string[]]

/** Major-unit price shape the manual-price input accepts: "19", "19.9", "19.90". */
const MAJOR_UNIT_PRICE_PATTERN = /^\d+(\.\d{1,2})?$/

/**
 * Convert a major-unit price string ("19.9") to integer minor units (1990 —
 * the unit the API, the DB and the WeChat order protocol all use). String-split
 * parsing instead of float math so "19.9" cannot degrade to 1989.99….
 * Returns null for empty/invalid input (the schema gates validity first).
 */
export function majorUnitsToMinor(price: string | null | undefined): number | null {
  if (!price) return null
  if (!MAJOR_UNIT_PRICE_PATTERN.test(price)) return null
  const [whole, frac = ''] = price.split('.')
  return Number(whole) * 100 + Number(frac.padEnd(2, '0'))
}

/**
 * Inverse of {@link majorUnitsToMinor} for editing an existing stored price:
 * 1990 → "19.90". Integer math only; empty string for absent values.
 */
export function minorToMajorUnits(minor: number | null | undefined): string {
  if (minor == null) return ''
  const sign = minor < 0 ? '-' : ''
  const abs = Math.abs(minor)
  return `${sign}${Math.trunc(abs / 100)}.${String(abs % 100).padStart(2, '0')}`
}

export const createEntitlementMappingSchema = z
  .object({
    // Plain string (not z.enum) so the unselected Select state ('') is a valid
    // runtime form value while still failing the `.min(1)` required check.
    // Mirrors `batchEntitlementMappingsSchema.paymentProvider` in billing-forms.
    paymentProvider: z
      .string()
      .min(1, { error: () => m['billing.create_mapping_provider_required']() }),
    externalProductId: z
      .string()
      .min(1, { error: () => m['billing.create_mapping_external_product_id_required']() }),
    externalPriceId: z.string().nullable().optional(),
    entitlementKey: z
      .string()
      .min(1, { error: () => m['billing.create_mapping_entitlement_key_required']() }),
    billingType: z
      .string()
      .min(1, { error: () => m['billing.create_mapping_billing_type_required']() }),
    billingPeriod: z.enum(billingPeriodValues).nullable().optional(),
    // WeChat manual price, entered in major units ("19.9") and converted to
    // integer minor units on submit via `majorUnitsToMinor`.
    priceYuan: z.string().nullable().optional(),
    currency: z.string().nullable().optional(),
    pointRules: pointDistributionRulesSchema,
    // column, semantically isolated from `validityDays`). Same shape as
    // validityDays so the form-level value is always a number | null.
    serviceDurationDays: z.number().int().min(1).nullable().optional(),
    grantedRoleIds: z.array(z.string()).optional(),
    enabled: z.boolean().optional(),
  })
  .superRefine((data, ctx) => {
    if (data.billingType === 'recurring' && !data.billingPeriod) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['billingPeriod'],
        message: m['billing.create_mapping_billing_period_required'](),
      })
    }
    if (data.billingType === 'non_renewing' && !data.serviceDurationDays) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['serviceDurationDays'],
        message: m['billing.create_mapping_service_duration_days_required'](),
      })
    }
    // Non-renewing + billingPeriod is a billing-semantics conflict: a dedicated
    // mutually-exclusive key (NOT the recurring-required wording, which mentions
    // 'recurring' and reads as "missing required", not "not allowed").
    if (data.billingType === 'non_renewing' && data.billingPeriod) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['billingPeriod'],
        message: m['billing.create_mapping_billing_period_mutually_exclusive'](),
      })
    }
    // WeChat has no hosted catalog (manual price, PRD wechat-support §2.2) and
    // no auto-renewal in scope (PRD §8.1) — the backend rejects both, so stop
    // them at form validation with field-level messages.
    if (providerRequiresManualPrice(data.paymentProvider)) {
      if (!data.priceYuan || (majorUnitsToMinor(data.priceYuan) ?? 0) < 1) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['priceYuan'],
          message: m['billing.create_mapping_price_invalid'](),
        })
      }
      if (!data.currency || !isValidCurrencyCode(data.currency)) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['currency'],
          message: m['billing.create_mapping_currency_invalid'](),
        })
      }
      if (data.billingType === 'recurring') {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['billingType'],
          message: m['billing.create_mapping_wechat_recurring_forbidden'](),
        })
      }
    }
  })

export type CreateEntitlementMappingFormData = z.infer<typeof createEntitlementMappingSchema>

/**
 * Default form values for the create-mapping dialog. `billingPeriod` /
 * `externalPriceId` / credit fields start null/empty; the dialog reveals the
 * recurring/one_time/non_renewing-specific ones conditionally.
 */
export function getCreateEntitlementMappingDefaults(): CreateEntitlementMappingFormData {
  return {
    paymentProvider: '',
    externalProductId: '',
    externalPriceId: null,
    entitlementKey: '',
    billingType: '',
    billingPeriod: null,
    // WeChat prefills CNY (its native settlement currency); other providers
    // never submit these fields.
    priceYuan: '',
    currency: 'CNY',
    pointRules: [],
    serviceDurationDays: null,
    grantedRoleIds: [],
    enabled: true,
  }
}
