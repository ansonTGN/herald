import { z } from 'zod'
import { m } from '@/paraglide/messages'

/**
 * Schema for transaction filters
 *
 * `bucketId` is the Credit Bucket dimension. It is
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
 * filter (`?bucketId=`). Kept separate from
 * {@link transactionFiltersSchema} because the URL form only carries the
 * shareable bucket dimension, not the ephemeral date/type filters.
 *
 * Consumed by the `/$realmId/user/points` route's `validateSearch`
 * (`validateSearch`); `bucketId` parsing and URL ↔ filter sync are covered by the
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
  // Target Credit Bucket. REQUIRED — no default — so a
  // grant without a bucket fails loud at the schema layer. The
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
 * Schema for a single quota window.
 *
 * Mirrors {@link QuotaWindowInput}: `windowSeconds` is the sliding window
 * length in seconds (must be > 0) and `limit` is the quota cap (>= 0; 0 is a
 * valid "grants nothing" edge case). Used by `MultiWindowQuotaEditor`.
 */
export const quotaWindowSchema = z.object({
  windowSeconds: z
    .number()
    .int({ error: () => m['points.quota_window_seconds_min']() })
    .min(1, { error: () => m['points.quota_window_seconds_min']() }),
  limit: z
    .number()
    .int({ error: () => m['points.quota_window_limit_min']() })
    .min(0, { error: () => m['points.quota_window_limit_min']() }),
})

export type QuotaWindowFormData = z.infer<typeof quotaWindowSchema>

/**
 * Schema for the full multi-window quota array.
 *
 * Capped at 8 windows. Each element must satisfy
 * {@link quotaWindowSchema}. The count cap is enforced here via `max(8)` so
 * pages embedding `MultiWindowQuotaEditor` can rely on the editor's local
 * validation to disable the add button at the same threshold.
 */
export const quotaWindowsSchema = z
  .array(quotaWindowSchema)
  .max(8, { error: () => m['points.quota_window_max']() })

export type QuotaWindowsFormData = z.infer<typeof quotaWindowsSchema>
