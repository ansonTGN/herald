import { z } from 'zod'
import type { UriItem } from '../redirect-uris-input'

/**
 * Zod schema for Step 2: Redirect URIs
 * Validates redirect URIs, post logout URIs, and web origins
 */

// Helper function to validate URI format
const uriSchema = z.string().refine(
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

/**
 * Schema for redirect URIs with detailed validation
 */
export const step2Schema = z.object({
  // Valid redirect URIs - at least one required
  redirectUris: z
    .array(uriSchema)
    .min(1, 'At least one redirect URI is required')
    .max(100, 'Cannot exceed 100 redirect URIs'),

  // Valid post logout redirect URIs - optional
  postLogoutUris: z.array(uriSchema).max(50, 'Cannot exceed 50 post logout URIs').default([]),

  // Web origins for CORS - optional
  webOrigins: z.array(uriSchema).max(50, 'Cannot exceed 50 web origins').default([]),
})

/**
 * TypeScript type inferred from the Zod schema
 */
export type Step2FormData = z.infer<typeof step2Schema>

/**
 * Transform redirect URIs from string array to UriItem array
 */
export function transformToUriItems(uris: string[]): UriItem[] {
  return uris.map((uri, index) => ({
    id: `init-${index}-${Date.now()}`,
    value: uri,
    isValid: true, // Assuming stored URIs are valid
  }))
}

/**
 * Transform UriItem array back to string array
 */
export function transformFromUriItems(items: UriItem[]): string[] {
  return items.filter((item) => item.isValid).map((item) => item.value)
}
