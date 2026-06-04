import { z } from 'zod'
import { m } from '@/paraglide/messages'

const urlSchema = z
  .string()
  .url({ error: () => m['client_apps.validation_url_invalid']() })
  .refine(
    (url) => {
      if (url.toLowerCase().startsWith('javascript:')) return false
      if (url.startsWith('//')) return false
      return true
    },
    { error: () => m['client_apps.validation_url_js_protocol']() }
  )

const sessionTtlSchema = z
  .number()
  .int({ error: () => m['client_apps.validation_session_ttl_integer']() })
  .min(60, { error: () => m['client_apps.validation_session_ttl_min']() })
  .max(86400, { error: () => m['client_apps.validation_session_ttl_max']() })

const sessionRenewalTtlSchema = z
  .number()
  .int({ error: () => m['client_apps.validation_session_renewal_ttl_integer']() })
  .min(60, { error: () => m['client_apps.validation_session_renewal_ttl_min']() })
  .max(604800, { error: () => m['client_apps.validation_session_renewal_ttl_max']() })
  .nullable()
  .optional()

export const createClientAppSchema = z
  .object({
    clientId: z
      .string()
      .min(3, { error: () => m['client_apps.validation_client_id_min_length']() })
      .max(36, { error: () => m['client_apps.validation_client_id_max_length']() })
      .regex(/^[a-zA-Z0-9-_]+$/, { error: () => m['client_apps.validation_client_id_format']() }),
    name: z
      .string()
      .min(1, { error: () => m['client_apps.validation_name_required']() })
      .max(100, { error: () => m['client_apps.validation_name_max_length_create']() }),
    description: z
      .string()
      .max(500, { error: () => m['client_apps.validation_description_max_length_create']() })
      .optional(),
    redirectUris: z
      .array(urlSchema)
      .min(1, { error: () => m['client_apps.validation_redirect_uris_required']() }),
    iconUrl: z
      .string()
      .url({ error: () => m['client_apps.validation_icon_url_invalid']() })
      .optional()
      .or(z.literal('')),
    enabled: z.boolean().default(true),
    sessionTtlSeconds: sessionTtlSchema.default(1800),
    sessionRenewalTtlSeconds: sessionRenewalTtlSchema,
    deviceCodeGrantEnabled: z.boolean().default(false),
  })
  .refine(
    (data) => {
      if (data.sessionRenewalTtlSeconds != null && data.sessionTtlSeconds != null) {
        return data.sessionRenewalTtlSeconds >= data.sessionTtlSeconds
      }
      return true
    },
    {
      error: () => m['client_apps.validation_session_renewal_gte'](),
      path: ['sessionRenewalTtlSeconds'],
    }
  )

export const updateClientAppSchema = z
  .object({
    name: z
      .string()
      .min(1, { error: () => m['client_apps.validation_name_required']() })
      .max(36, { error: () => m['client_apps.validation_name_max_length_update']() }),
    description: z
      .string()
      .max(255, { error: () => m['client_apps.validation_description_max_length_update']() })
      .optional(),
    redirectUris: z
      .array(urlSchema)
      .min(1, { error: () => m['client_apps.validation_redirect_uris_required']() }),
    iconUrl: z
      .string()
      .url({ error: () => m['client_apps.validation_icon_url_invalid']() })
      .optional()
      .or(z.literal('')),
    enabled: z.boolean(),
    sessionTtlSeconds: sessionTtlSchema,
    sessionRenewalTtlSeconds: sessionRenewalTtlSchema,
    deviceCodeGrantEnabled: z.boolean(),
    regenerateSecret: z.boolean().default(false),
  })
  .refine(
    (data) => {
      if (data.sessionRenewalTtlSeconds != null && data.sessionTtlSeconds != null) {
        return data.sessionRenewalTtlSeconds >= data.sessionTtlSeconds
      }
      return true
    },
    {
      error: () => m['client_apps.validation_session_renewal_gte'](),
      path: ['sessionRenewalTtlSeconds'],
    }
  )

export type CreateClientAppFormData = z.infer<typeof createClientAppSchema>
export type UpdateClientAppFormData = z.infer<typeof updateClientAppSchema>
