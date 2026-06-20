import { z } from 'zod'
import { m } from '@/paraglide/messages'

// ==================== Entitlement Mapping Update Schema ====================

const ENTITLEMENT_KEY_REGEX = /^[a-z0-9-]{1,64}$/

export const entitlementMappingUpdateSchema = z.object({
  entitlementKey: z
    .string()
    .min(1, { error: () => m['billing.entitlement_key_required']() })
    .regex(ENTITLEMENT_KEY_REGEX, { error: () => m['billing.entitlement_key_format']() }),

  enabled: z.boolean().default(false),

  pointsPerPeriod: z.number().int().min(0).optional().nullable(),

  grantPeriodType: z.enum(['once', 'daily', 'weekly', 'monthly']).optional().nullable(),

  validityDays: z.number().int().min(1).optional().nullable(),

  grantOnSubscribe: z.boolean().default(false),

  maxPeriods: z.number().int().min(1).optional().nullable(),

  // Bound Credit Bucket (design §4.2.1). Triple-state at the PATCH boundary:
  //   undefined  -> leave unchanged (omit from body)
  //   null       -> clear (unassign)
  //   "<uuid>"   -> set/reassign
  // The toggle-enabled path (MappingRow) omits this to preserve attribution;
  // the detail dialog always supplies the full intended value.
  bucketId: z.string().nullable().optional(),
})

export type EntitlementMappingUpdateFormData = z.infer<typeof entitlementMappingUpdateSchema>

export function getEntitlementMappingUpdateDefaults(
  config?: Partial<EntitlementMappingUpdateFormData>
): EntitlementMappingUpdateFormData {
  return {
    entitlementKey: '',
    enabled: false,
    pointsPerPeriod: null,
    grantPeriodType: null,
    validityDays: null,
    grantOnSubscribe: false,
    maxPeriods: null,
    bucketId: null,
    ...config,
  } as EntitlementMappingUpdateFormData
}
