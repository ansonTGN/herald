import { z } from 'zod'

/**
 * Zod schema for Step 3: Security Settings
 * Validates session TTL and advanced security options
 */

/**
 * Schema for security settings
 * Note: Currently only session-related settings are available in the backend API
 * Authentication Flow and Token Endpoint Auth will be added when backend supports them
 */
export const step3Schema = z
  .object({
    // Session time-to-live in seconds
    sessionTtlSeconds: z
      .number()
      .min(60, 'Session TTL must be at least 60 seconds (1 minute)')
      .max(86400, 'Session TTL must not exceed 86400 seconds (24 hours)')
      .default(3600), // Default: 1 hour

    // Session renewal time-to-live in seconds (optional)
    sessionRenewalTtlSeconds: z
      .number()
      .max(604800, 'Session renewal TTL must not exceed 604800 seconds (7 days)')
      .optional(),
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
 * TypeScript type inferred from the Zod schema
 */
export type Step3FormData = z.infer<typeof step3Schema>

/**
 * Session TTL preset options for quick selection
 */
export const SESSION_TTL_PRESETS = [
  { value: 1800, label: '30 minutes', seconds: 1800 },
  { value: 3600, label: '1 hour', seconds: 3600 },
  { value: 7200, label: '2 hours', seconds: 7200 },
  { value: 14400, label: '4 hours', seconds: 14400 },
  { value: 28800, label: '8 hours', seconds: 28800 },
  { value: 43200, label: '12 hours', seconds: 43200 },
  { value: 86400, label: '24 hours', seconds: 86400 },
] as const

/**
 * Advanced security options (for future implementation)
 * These will be integrated when the backend API supports them
 */
export const ADVANCED_SECURITY_OPTIONS = [
  {
    id: 'requireProofKeyForCodeExchange',
    label: 'Require PKCE',
    description: 'Enforce Proof Key for Code Exchange for authorization code flow',
    type: 'boolean',
    default: false,
  },
  {
    id: 'implicitFlowEnabled',
    label: 'Enable Implicit Flow',
    description: 'Allow implicit flow (not recommended for new apps)',
    type: 'boolean',
    default: false,
  },
  {
    id: 'serviceAccountsEnabled',
    label: 'Enable Service Accounts',
    description: 'Allow service account authentication for this client',
    type: 'boolean',
    default: false,
  },
  {
    id: 'directAccessGrantsEnabled',
    label: 'Enable Direct Access Grants',
    description: 'Allow resource owner password credentials flow',
    type: 'boolean',
    default: false,
  },
  {
    id: 'standardFlowEnabled',
    label: 'Enable Authorization Code Flow',
    description: 'Allow standard authorization code flow',
    type: 'boolean',
    default: true,
  },
  {
    id: 'frontchannelLogoutEnabled',
    label: 'Enable Front-Channel Logout',
    description: 'Support front-channel single logout',
    type: 'boolean',
    default: false,
  },
  {
    id: 'backchannelLogoutEnabled',
    label: 'Enable Back-Channel Logout',
    description: 'Support back-channel single logout',
    type: 'boolean',
    default: false,
  },
  {
    id: 'consentRequired',
    label: 'Require User Consent',
    description: 'Always require user consent for authorization',
    type: 'boolean',
    default: true,
  },
] as const
