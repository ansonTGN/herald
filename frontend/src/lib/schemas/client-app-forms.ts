import { z } from 'zod'

const urlSchema = z
  .string()
  .url('Invalid URL format')
  .refine((url) => {
    if (url.toLowerCase().startsWith('javascript:')) return false
    if (url.startsWith('//')) return false
    return true
  }, 'Invalid URL format: javascript: protocol and protocol-relative URLs are not allowed')

const sessionTtlSchema = z
  .number()
  .int('Session TTL must be an integer')
  .min(60, 'Session TTL must be at least 60 seconds')
  .max(86400, 'Session TTL must be at most 86400 seconds (24 hours)')

const sessionRenewalTtlSchema = z
  .number()
  .int('Session renewal TTL must be an integer')
  .min(60, 'Session renewal TTL must be at least 60 seconds')
  .max(604800, 'Session renewal TTL must be at most 604800 seconds (7 days)')
  .nullable()
  .optional()

export const createClientAppSchema = z
  .object({
    clientId: z
      .string()
      .min(3, 'Client ID must be at least 3 characters')
      .max(36, 'Client ID must be at most 36 characters')
      .regex(
        /^[a-zA-Z0-9-_]+$/,
        'Client ID must contain only letters, numbers, hyphens, and underscores'
      ),
    name: z.string().min(1, 'Name is required').max(100, 'Name must be at most 100 characters'),
    description: z.string().max(500, 'Description must be at most 500 characters').optional(),
    redirectUris: z.array(urlSchema).min(1, 'At least one redirect URI is required'),
    iconUrl: z.string().url('Invalid icon URL').optional().or(z.literal('')),
    enabled: z.boolean().default(true),
    sessionTtlSeconds: sessionTtlSchema.default(1800),
    sessionRenewalTtlSeconds: sessionRenewalTtlSchema,
    deviceCodeGrantEnabled: z.boolean().default(false),
  })
  .refine(
    (data) => {
      if (data.sessionRenewalTtlSeconds != null && data.sessionTtlSeconds != null) {
        return data.sessionRenewalTtlSeconds > data.sessionTtlSeconds
      }
      return true
    },
    {
      message: 'Session renewal TTL must be greater than session TTL',
      path: ['sessionRenewalTtlSeconds'],
    }
  )

export const updateClientAppSchema = z
  .object({
    name: z.string().min(1, 'Name is required').max(36, 'Name must be at most 36 characters'),
    description: z.string().max(255, 'Description must be at most 255 characters').optional(),
    redirectUris: z.array(urlSchema).min(1, 'At least one redirect URI is required'),
    iconUrl: z.string().url('Invalid icon URL').optional().or(z.literal('')),
    enabled: z.boolean(),
    sessionTtlSeconds: sessionTtlSchema,
    sessionRenewalTtlSeconds: sessionRenewalTtlSchema,
    deviceCodeGrantEnabled: z.boolean(),
    regenerateSecret: z.boolean().default(false),
  })
  .refine(
    (data) => {
      if (data.sessionRenewalTtlSeconds != null && data.sessionTtlSeconds != null) {
        return data.sessionRenewalTtlSeconds > data.sessionTtlSeconds
      }
      return true
    },
    {
      message: 'Session renewal TTL must be greater than session TTL',
      path: ['sessionRenewalTtlSeconds'],
    }
  )

export type CreateClientAppFormData = z.infer<typeof createClientAppSchema>
export type UpdateClientAppFormData = z.infer<typeof updateClientAppSchema>
