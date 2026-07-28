import { z } from 'zod'
import { BILLING_PERIODS } from '@/lib/billing-constants'
import { m } from '@/paraglide/messages'

/**
 * Create-entitlement-mapping form schema (design support-iap §4.4.2 / §4.2.2).
 *
 * Kept as its own file (NOT merged into `billing-forms.ts`) because the field
 * set (paymentProvider / externalProductId / entitlementKey / bucketId /
 * billingType / billingPeriod / grantedRoleIds ...) is a different schema
 * family from the `priceMappingUpdateSchema` / `batchEntitlementMappingsSchema`
 * (batch-update, price-granularity PATCH) family that lives there. Merging
 * would conflate single-create vs batch-update semantics.
 *
 * Refinements (mirror the backend constraints in §4.2.2):
 * - `billingType === 'recurring'` ⇒ `billingPeriod` is required (monthly/yearly).
 * - `billingType === 'one_time'` ⇒ `validityDays` is optional-but-fillable;
 *   `billingPeriod` is dropped on submit.
 *
 * `externalPriceId` is optional: IAP (apple/google) and Creem leave it empty;
 * Stripe requires it (the backend enforces — we don't hard-fail client-side to
 * keep the form permissive for the IAP primary case).
 */

const billingPeriodValues = Object.values(BILLING_PERIODS) as [string, ...string[]]

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
    bucketId: z.string().min(1, { error: () => m['billing.create_mapping_bucket_required']() }),
    billingType: z
      .string()
      .min(1, { error: () => m['billing.create_mapping_billing_type_required']() }),
    billingPeriod: z.enum(billingPeriodValues).nullable().optional(),
    // Credit-strategy fields (require points.manage; gated by the dialog).
    pointsPerPeriod: z.number().int().min(0).nullable().optional(),
    grantOnSubscribe: z.boolean().nullable().optional(),
    validityDays: z.number().int().min(1).nullable().optional(),
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
  })

export type CreateEntitlementMappingFormData = z.infer<typeof createEntitlementMappingSchema>

/**
 * Default form values for the create-mapping dialog. `billingPeriod` /
 * `externalPriceId` / credit fields start null/empty; the dialog reveals the
 * recurring/one_time-specific ones conditionally.
 */
export function getCreateEntitlementMappingDefaults(): CreateEntitlementMappingFormData {
  return {
    paymentProvider: '',
    externalProductId: '',
    externalPriceId: null,
    entitlementKey: '',
    bucketId: '',
    billingType: '',
    billingPeriod: null,
    pointsPerPeriod: null,
    grantOnSubscribe: false,
    validityDays: null,
    grantedRoleIds: [],
    enabled: true,
  }
}
