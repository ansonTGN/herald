import { z } from 'zod'
import { quotaWindowSchema } from '@/lib/schemas/points-forms'

// ==================== Price-Level Batch Save Schema ====================
//
// Mirrors the generated `PriceMappingUpdate` / `BatchUpdateEntitlementMappingsRequest`.
// One price row per entry; `entitlementKey` is provider-sync owned and not editable here.

export const priceMappingUpdateSchema = z.object({
  mappingId: z.string().min(1),

  billingType: z.string().nullable().optional(),

  billingPeriod: z.string().nullable().optional(),

  enabled: z.boolean().nullable().optional(),

  pointsPerPeriod: z.number().int().min(0).nullable().optional(),

  validityDays: z.number().int().min(1).nullable().optional(),

  grantOnSubscribe: z.boolean().nullable().optional(),

  // Role-grant dimension (design §4.4 / §5.2). Mirrors the generated
  // `PriceMappingUpdate.grantedRoleIds` three-state contract: `null`/undefined
  // ⟺ leave unchanged, `[]` ⟺ clear (no role grant), non-empty ⟺ set. Orthogonal
  // to billing_type and points strategy (empty points + roles = pure entitlement;
  // empty roles + points = pure credit pack; both empty = payment record only).
  grantedRoleIds: z.array(z.string()).nullable().optional(),

  // Per-price quota windows (design §3.2 / §4.3.2). Mirrors
  // `PriceMappingUpdate.quotaWindows`: `null`/undefined ⟺ leave unchanged,
  // `[]` ⟺ clear. Capped at 8 windows (PRD §4). Validation rules per window
  // are shared with the realm-default editor via `quotaWindowSchema`.
  quotaWindows: z.array(quotaWindowSchema).max(8).nullable().optional(),
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
 * current price rows. Each input row must already carry its `mappingId`.
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
