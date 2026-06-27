import { z } from 'zod'
import { m } from '@/paraglide/messages'

// ==================== Price-Level Batch Save Schema ====================
//
// Mirrors the generated `PriceMappingUpdate` / `BatchUpdateEntitlementMappingsRequest`.
// One price row per entry; `entitlementKey` is shared across the product's prices.

const ENTITLEMENT_KEY_REGEX = /^[a-z0-9-]{1,64}$/

export const priceMappingUpdateSchema = z.object({
  mappingId: z.string().min(1),

  entitlementKey: z
    .string()
    .min(1, { error: () => m['billing.entitlement_key_required']() })
    .regex(ENTITLEMENT_KEY_REGEX, { error: () => m['billing.entitlement_key_format']() }),

  billingType: z.string().nullable().optional(),

  billingPeriod: z.string().nullable().optional(),

  enabled: z.boolean().nullable().optional(),

  pointsPerPeriod: z.number().int().min(0).nullable().optional(),

  grantPeriodType: z.enum(['once', 'daily', 'weekly', 'monthly']).nullable().optional(),

  validityDays: z.number().int().min(1).nullable().optional(),

  grantOnSubscribe: z.boolean().nullable().optional(),

  maxPeriods: z.number().int().min(1).nullable().optional(),
})

export type PriceMappingUpdateFormData = z.infer<typeof priceMappingUpdateSchema>

export const batchEntitlementMappingsSchema = z.object({
  paymentProvider: z.string().min(1),
  externalProductId: z.string().min(1),
  updates: z.array(priceMappingUpdateSchema).min(1),
})

export type BatchEntitlementMappingsFormData = z.infer<typeof batchEntitlementMappingsSchema>

/**
 * Build form defaults for the price-level batch editor from the product's
 * current price rows. Each input row must already carry its `mappingId`;
 * the entitlement key is seeded from the row (the editor renames it group-wide).
 */
export function getBatchEntitlementMappingsDefaults(
  config?: Partial<BatchEntitlementMappingsFormData>
): BatchEntitlementMappingsFormData {
  return {
    paymentProvider: '',
    externalProductId: '',
    updates: [],
    ...config,
  } as BatchEntitlementMappingsFormData
}
