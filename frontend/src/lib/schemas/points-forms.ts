import { z } from 'zod'
import { m } from '@/paraglide/messages'

/**
 * Schema for transaction filters
 *
 * `bucketId` is the Credit Bucket dimension (design §4.4.2 / §4.2.3). It is
 * optional so the "all buckets" state is represented by `undefined`, not a
 * sentinel. The URL search-param form is {@link transactionBucketSearchSchema}.
 */
export const transactionFiltersSchema = z.object({
  transactionType: z.enum(['recharge', 'consume']).optional(),
  startTime: z.string().datetime().optional(),
  endTime: z.string().datetime().optional(),
  clientAppId: z.string().optional(),
  bucketId: z.string().uuid().optional(),
})

export type TransactionFilters = z.infer<typeof transactionFiltersSchema>

/**
 * URL search-param schema for the user points route's transaction-bucket
 * filter (design §4.2.3 `?bucketId=`). Kept separate from
 * {@link transactionFiltersSchema} because the URL form only carries the
 * shareable bucket dimension, not the ephemeral date/type filters.
 *
 * Consumed by the `/$realmId/user/points` route's `validateSearch`
 * (FE-D06); `bucketId` parsing and URL ↔ filter sync are covered by the
 * frontend/test slot.
 */
export const transactionBucketSearchSchema = z.object({
  bucketId: z.string().uuid().optional(),
})

export type TransactionBucketSearch = z.infer<typeof transactionBucketSearchSchema>

/**
 * Schema for account filters
 */
export const accountFiltersSchema = z.object({
  search: z.string().optional(),
  status: z.enum(['active', 'frozen', 'closed']).optional(),
})

export type AccountFilters = z.infer<typeof accountFiltersSchema>

/**
 * Schema for granting points to a user
 */
export const grantPointsSchema = z.object({
  userId: z.string().min(1, { error: () => m['points.validation_user_required']() }),
  amount: z
    .number()
    .int({ error: () => m['points.validation_amount_integer']() })
    .min(1, { error: () => m['points.validation_amount_min']() }),
  reason: z.string().min(1, { error: () => m['points.validation_reason_required']() }),
  // Target Credit Bucket (design §4.2.4 / §4.4). REQUIRED — no default — so a
  // grant without a bucket fails loud at the schema layer (FE-D07). The
  // backend independently rejects a missing/invalid bucketId with 400
  // `grant_bucket_required` as defense-in-depth.
  bucketId: z.string().min(1, { error: () => m['points.validation_bucket_required']() }),
  validityDays: z
    .number()
    .int({ error: () => m['points.validation_validity_days_integer']() })
    .min(1, { error: () => m['points.validation_validity_days_min']() })
    .nullable()
    .optional(),
})

export type GrantPointsFormData = z.infer<typeof grantPointsSchema>

/**
 * Schema for points default configuration
 *
 * Note: This schema matches the backend API response.
 * The backend uses freePeriodic* fields instead of dailyPoints* fields.
 * Use generated types from src/lib/api-generated/ as the source of truth.
 */
export const pointsDefaultConfigSchema = z
  .object({
    registrationBonusPoints: z
      .number()
      .int({ error: () => m['points.validation_registration_bonus_integer']() })
      .min(0, { error: () => m['points.validation_registration_bonus_non_negative']() }),
    freePeriodicPointsAmount: z
      .number()
      .int({ error: () => m['points.validation_periodic_amount_integer']() })
      .min(0, { error: () => m['points.validation_periodic_amount_non_negative']() }),
    freePeriodicGrantPeriodType: z.enum(['once', 'daily', 'weekly', 'monthly'], {
      error: () => m['points.validation_grant_period_type_invalid'](),
    }),
    freePeriodicValidityDays: z
      .number()
      .int({ error: () => m['points.validation_validity_days_integer']() })
      .min(0, { error: () => m['points.validation_validity_days_non_negative']() }),
  })
  .superRefine((data, ctx) => {
    if (data.freePeriodicGrantPeriodType !== 'once' && data.freePeriodicValidityDays < 1) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: m['points.validation_validity_days_for_periodic'](),
        path: ['freePeriodicValidityDays'],
      })
    }
  })

export type PointsDefaultConfigFormData = z.infer<typeof pointsDefaultConfigSchema>
