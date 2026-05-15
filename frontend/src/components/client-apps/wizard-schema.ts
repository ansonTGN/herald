import { z } from 'zod'
import type { ClientAppItem } from '@/lib/api-generated'

/**
 * Combined Zod schema for all wizard steps
 *
 * This schema consolidates validation rules from all individual step schemas
 * into a single, comprehensive schema for the entire wizard.
 *
 * Step 1: Basic Information
 * - name (required): App name (1-100 characters)
 * - description (optional): App description (max 500 characters)
 * - appType (required): Application type (WEB/SERVICE/MOBILE/NATIVE)
 * - clientType (required): Client type (PUBLIC/CONFIDENTIAL)
 *
 * Step 2: Redirect URIs
 * - redirectUris (required): At least one valid HTTPS/HTTP URL
 * - postLogoutUris (optional): Post-logout redirect URLs
 * - webOrigins (optional): CORS allowed origins
 *
 * Step 3: Security Settings
 * - sessionTtlSeconds (required): Session timeout (60-86400 seconds)
 * - sessionRenewalTtlSeconds (optional): Session renewal window (must exceed session TTL)
 * - deviceCodeGrantEnabled (optional): Enable Device Authorization Grant (RFC 8628)
 */
export const wizardSchema = z
  .object({
    // Step 1: Basic Information
    name: z
      .string()
      .min(1, 'App name is required')
      .max(100, 'App name must be less than 100 characters'),

    description: z
      .string()
      .max(500, 'Description must be less than 500 characters')
      .optional()
      .or(z.literal('')),

    appType: z
      .enum(['WEB', 'SERVICE', 'MOBILE', 'NATIVE'], {
        message: 'Please select an app type',
      })
      .optional()
      .refine((val) => val !== undefined, {
        message: 'Please select an app type',
      }),

    clientType: z
      .enum(['PUBLIC', 'CONFIDENTIAL'], {
        message: 'Please select a client type',
      })
      .optional()
      .refine((val) => val !== undefined, {
        message: 'Please select a client type',
      }),

    // Step 2: Redirect URIs
    redirectUris: z
      .array(
        z.string().refine(
          (val) => {
            if (!val.trim()) return false
            try {
              const url = new URL(val)
              return url.protocol === 'https:' || url.protocol === 'http:'
            } catch {
              return false
            }
          },
          {
            message: 'Must be a valid URL starting with https:// or http://',
          }
        )
      )
      .min(1, 'At least one redirect URI is required')
      .max(100, 'Cannot exceed 100 redirect URIs'),

    postLogoutUris: z
      .array(
        z.string().refine(
          (val) => {
            if (!val.trim()) return false
            try {
              const url = new URL(val)
              return url.protocol === 'https:' || url.protocol === 'http:'
            } catch {
              return false
            }
          },
          {
            message: 'Must be a valid URL starting with https:// or http://',
          }
        )
      )
      .max(50, 'Cannot exceed 50 post logout URIs')
      .default([]),

    webOrigins: z
      .array(
        z.string().refine(
          (val) => {
            if (!val.trim()) return false
            try {
              const url = new URL(val)
              return url.protocol === 'https:' || url.protocol === 'http:'
            } catch {
              return false
            }
          },
          {
            message: 'Must be a valid URL starting with https:// or http://',
          }
        )
      )
      .max(50, 'Cannot exceed 50 web origins')
      .default([]),

    // Step 3: Security Settings
    sessionTtlSeconds: z
      .number()
      .min(60, 'Session TTL must be at least 60 seconds (1 minute)')
      .max(86400, 'Session TTL must not exceed 86400 seconds (24 hours)')
      .default(3600), // Default: 1 hour

    sessionRenewalTtlSeconds: z
      .number()
      .max(604800, 'Session renewal TTL must not exceed 604800 seconds (7 days)')
      .optional(),

    deviceCodeGrantEnabled: z.boolean().default(false),
  })
  .refine(
    (data) => {
      // If renewal TTL is set, it must be greater than session TTL
      if (data.sessionRenewalTtlSeconds && data.sessionTtlSeconds) {
        return data.sessionRenewalTtlSeconds > data.sessionTtlSeconds
      }
      return true
    },
    {
      message: 'Session renewal TTL must be greater than session TTL',
      path: ['sessionRenewalTtlSeconds'],
    }
  )

/**
 * TypeScript type inferred from the wizard schema
 * This represents the complete form data structure for all wizard steps
 */
export type WizardFormData = z.infer<typeof wizardSchema>

/**
 * Map API data to form default values
 *
 * Converts ClientAppItem (API type) to WizardFormData (form type)
 * Used to initialize the form in edit mode
 *
 * @param data - Optional API data from existing client app
 * @returns Form default values matching the wizard schema
 *
 * @example
 * ```ts
 * const form = useAppForm({
 *   schema: wizardSchema,
 *   defaultValues: mapInitialData(initialData),
 * })
 * ```
 */
export function mapInitialData(data?: ClientAppItem): WizardFormData {
  return {
    // Step 1: Basic Information
    name: data?.name ?? '',
    description: data?.description ?? '',
    appType: undefined,
    clientType: undefined,

    // Step 2: Redirect URIs
    redirectUris: data?.redirectUris ?? [],
    postLogoutUris: [], // Not currently in API type
    webOrigins: [], // Not currently in API type

    // Step 3: Security Settings
    sessionTtlSeconds: data?.sessionTtlSeconds ?? 3600, // Default: 1 hour
    sessionRenewalTtlSeconds: data?.sessionRenewalTtlSeconds ?? undefined,
    deviceCodeGrantEnabled: data?.deviceCodeGrantEnabled ?? false,
  }
}

// FormApi type removed - not needed for the current implementation
// The form instance is automatically typed by useAppForm
