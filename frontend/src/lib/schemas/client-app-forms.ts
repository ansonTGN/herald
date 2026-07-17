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

/**
 * Validates an allowed CORS origin: an exact https:// origin, or an
 * http://localhost variant for local development (no path/query/wildcard).
 * Matches the backend `validate_redirect_uri`-based origin normalization.
 */
const allowedOriginSchema = z
  .string()
  .trim()
  .refine(
    (origin) => {
      try {
        const url = new URL(origin)
        if (url.protocol !== 'https:' && url.protocol !== 'http:') return false
        // Origins are scheme + host + port only; no path/query/hash.
        if (url.pathname !== '/' || url.search !== '' || url.hash !== '') return false
        // http:// is only permitted for localhost development.
        if (url.protocol === 'http:' && url.hostname !== 'localhost') return false
        return true
      } catch {
        return false
      }
    },
    { error: () => m['client_apps.validation_allowed_origins_origin']() }
  )

const browserRefreshTtlSchema = z
  .number()
  .int({ error: () => m['client_apps.validation_browser_refresh_ttl_integer']() })
  .min(86400, { error: () => m['client_apps.validation_browser_refresh_ttl_min']() })
  .max(7776000, { error: () => m['client_apps.validation_browser_refresh_ttl_max']() })

// Cloudflare Turnstile public site key (D-PROTECT-01). Shared by the create
// and update schemas.
const turnstileSiteKeySchema = z.string().trim().optional().or(z.literal(''))

/** Default browser refresh token family absolute TTL: 30 days (design §4.3.2). */
export const DEFAULT_BROWSER_REFRESH_TTL_SECONDS = 2592000

export const createClientAppSchema = z.object({
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
  browserRefreshAbsoluteTtlSeconds: browserRefreshTtlSchema.default(
    DEFAULT_BROWSER_REFRESH_TTL_SECONDS
  ),
  allowedOrigins: z.array(allowedOriginSchema).default([]),
  deviceCodeGrantEnabled: z.boolean().default(false),
  // Cloudflare Turnstile (D-PROTECT-01): per-Client-App human-verification.
  // The secret is write-only: an empty value means "do not set" on create.
  turnstileEnabled: z.boolean().default(false),
  turnstileSiteKey: turnstileSiteKeySchema,
  turnstileSecretKey: z.string().optional(),
})

export const updateClientAppSchema = z.object({
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
  browserRefreshAbsoluteTtlSeconds: browserRefreshTtlSchema,
  allowedOrigins: z.array(allowedOriginSchema),
  deviceCodeGrantEnabled: z.boolean(),
  regenerateSecret: z.boolean().default(false),
  // Cloudflare Turnstile (D-PROTECT-01): per-Client-App human-verification.
  // The secret is write-only and NEVER echoed back (ClientAppItem omits it),
  // so an empty/omitted value here means "leave the stored secret untouched";
  // a non-empty value replaces it. Mirrors how `regenerateSecret` and the
  // OAuth mutation (`oauth-mutations.ts`) treat `clientSecret`.
  turnstileEnabled: z.boolean().default(false),
  turnstileSiteKey: turnstileSiteKeySchema,
  turnstileSecretKey: z.string().optional(),
})

export type CreateClientAppFormData = z.infer<typeof createClientAppSchema>
export type UpdateClientAppFormData = z.infer<typeof updateClientAppSchema>
