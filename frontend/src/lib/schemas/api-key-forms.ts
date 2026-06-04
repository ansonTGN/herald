import { z } from 'zod'
import { m } from '@/paraglide/messages'

/**
 * Converts a datetime-local input value (e.g. "2026-12-31T23:59") to RFC 3339
 * format (e.g. "2026-12-31T23:59:00.000Z").
 * Returns undefined for empty strings so the field is omitted from the request.
 */
function normalizeExpiresAt(value: string): string | undefined {
  if (!value) return undefined
  // datetime-local gives "YYYY-MM-DDTHH:mm" — append seconds and UTC zone
  return new Date(value).toISOString()
}

export const createApiKeySchema = z.object({
  name: z
    .string()
    .min(1, { error: () => m['api_keys.name_required']() })
    .max(100, { error: () => m['api_keys.name_max_length']() }),
  clientAppId: z.string().uuid().optional(),
  expiresAt: z
    .string()
    .optional()
    .transform((val) => (val === undefined || val === '' ? undefined : normalizeExpiresAt(val))),
})

export const updateApiKeySchema = z.object({
  name: z
    .string()
    .min(1, { error: () => m['api_keys.name_required']() })
    .max(100, { error: () => m['api_keys.name_max_length']() }),
  enabled: z.boolean(),
  expiresAt: z
    .string()
    .nullable()
    .transform((val) => (val === null || val === '' ? null : val ? normalizeExpiresAt(val) : null)),
})

export type CreateApiKeyFormData = z.infer<typeof createApiKeySchema>
export type UpdateApiKeyFormData = z.infer<typeof updateApiKeySchema>
