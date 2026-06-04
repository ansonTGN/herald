import { z } from 'zod'
import { PROVIDER_TYPES } from '@/lib/oauth-provider-constants'
import { m } from '@/paraglide/messages'

/**
 * Create OAuth Config Schema
 */
export const createOAuthConfigSchema = z.object({
  providerType: z.enum(PROVIDER_TYPES, {
    error: () => m['oauth.validation_provider_type_required'](),
  }),
  clientId: z.string().min(1, { error: () => m['oauth.validation_client_id_required']() }),
  clientSecret: z.string().min(1, { error: () => m['oauth.validation_client_secret_required']() }),
  scopes: z.array(z.string()).optional(),
  enabled: z.boolean().default(true),
})

/**
 * Update OAuth Config Schema
 * clientSecret is optional for updates (leave empty to keep existing)
 */
export const updateOAuthConfigSchema = createOAuthConfigSchema.partial().extend({
  clientSecret: z.string().optional(),
})

/**
 * Get dynamic schema for form based on edit mode
 * When editing, clientSecret is optional (leave empty to keep existing)
 */
export function getOAuthConfigSchema(isEditing: boolean = false) {
  if (isEditing) {
    return createOAuthConfigSchema.extend({
      clientSecret: z.string().optional(),
    })
  }
  return createOAuthConfigSchema
}

export type CreateOAuthConfigFormData = z.infer<typeof createOAuthConfigSchema>
export type UpdateOAuthConfigFormData = z.infer<typeof updateOAuthConfigSchema>
export type OAuthConfigFormData = z.infer<ReturnType<typeof getOAuthConfigSchema>>
