import { z } from 'zod'
import { m } from '@/paraglide/messages'

/**
 * bucketKey format: lowercase alphanumeric + hyphen, 1-64 chars (design §4.2.2).
 * Matches backend `^[a-z0-9-]{1,64}$`.
 */
export const bucketKeyRegex = /^[a-z0-9-]{1,64}$/

/**
 * UUID-ish string validator. We keep it permissive (`min(1)`) rather than a
 * strict uuid regex because the backend is the authority on UUID shape; the
 * fail-loud concern here is "at least one" (coverage set non-empty), not the
 * exact uuid grammar. The generated request type already constrains to
 * `Array<string>`.
 */
const idField = z.string().min(1)

/**
 * Create schema for a Credit Bucket (design §4.2.2).
 *
 * Coverage set (`clientAppIds`) MUST be non-empty — fail loud at the schema
 * layer with `.min(1)` so the form never submits an empty coverage set. NO
 * `isDefault` field anywhere (design A4).
 */
export const createCreditBucketSchema = z.object({
  bucketKey: z
    .string()
    .min(1, { error: () => m['credit_buckets.validation_bucket_key_required']() })
    .regex(bucketKeyRegex, {
      error: () => m['credit_buckets.validation_bucket_key_format'](),
    }),
  name: z.string().min(1, { error: () => m['credit_buckets.validation_name_required']() }),
  description: z.string().nullable().optional(),
  displayOrder: z.number().int().nullable().optional(),
  enabled: z.boolean(),
  receivesRegistrationCredits: z.boolean(),
  clientAppIds: z
    .array(idField)
    .min(1, { error: () => m['credit_buckets.validation_coverage_required']() }),
  entitlementMappingIds: z.array(idField).optional(),
})

export type CreateCreditBucketFormData = z.infer<typeof createCreditBucketSchema>

/**
 * Update schema for a Credit Bucket (design §4.2.3 PUT).
 *
 * `bucketKey` is NOT editable on update (immutable identity) — hence absent.
 * Coverage set still must be non-empty (backend rejects empty with 400).
 */
export const updateCreditBucketSchema = z.object({
  name: z.string().min(1, { error: () => m['credit_buckets.validation_name_required']() }),
  description: z.string().nullable().optional(),
  displayOrder: z.number().int().nullable().optional(),
  enabled: z.boolean(),
  receivesRegistrationCredits: z.boolean(),
  clientAppIds: z
    .array(idField)
    .min(1, { error: () => m['credit_buckets.validation_coverage_required']() }),
  entitlementMappingIds: z.array(idField).optional(),
})

export type UpdateCreditBucketFormData = z.infer<typeof updateCreditBucketSchema>
