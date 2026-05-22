import { z } from 'zod'

export const createApiKeySchema = z.object({
  name: z.string().min(1, 'Name is required').max(100, 'Name must be at most 100 characters'),
  expiresAt: z.string().optional(),
})

export const updateApiKeySchema = z.object({
  name: z.string().min(1, 'Name is required').max(100, 'Name must be at most 100 characters'),
  enabled: z.boolean(),
  expiresAt: z.string().nullable(),
})

export type CreateApiKeyFormData = z.infer<typeof createApiKeySchema>
export type UpdateApiKeyFormData = z.infer<typeof updateApiKeySchema>
