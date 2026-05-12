import { z } from 'zod'

/**
 * Zod schema for Step 1: Basic Information
 * Validates app name, description, app type, and client type fields
 */
export const step1Schema = z.object({
  name: z
    .string()
    .min(1, 'App name is required')
    .max(100, 'App name must be less than 100 characters'),

  description: z
    .string()
    .max(500, 'Description must be less than 500 characters')
    .optional()
    .or(z.literal('')),

  appType: z.enum(['WEB', 'SERVICE', 'MOBILE', 'NATIVE'], {
    message: 'Please select an app type',
  }),

  clientType: z.enum(['PUBLIC', 'CONFIDENTIAL'], {
    message: 'Please select a client type',
  }),
})

/**
 * TypeScript type inferred from the Zod schema
 */
export type Step1FormData = z.infer<typeof step1Schema>

/**
 * App type options for the select dropdown
 */
export const APP_TYPE_OPTIONS = [
  { value: 'WEB', label: 'Web Application' },
  { value: 'SERVICE', label: 'Service/Backend' },
  { value: 'MOBILE', label: 'Mobile Application' },
  { value: 'NATIVE', label: 'Native Application' },
] as const

/**
 * Client type options for radio buttons
 */
export const CLIENT_TYPE_OPTIONS = [
  {
    value: 'CONFIDENTIAL',
    label: 'Confidential',
    description: 'For server-side applications that can securely store credentials',
  },
  {
    value: 'PUBLIC',
    label: 'Public',
    description: 'For SPAs, mobile apps, or other clients that cannot securely store secrets',
  },
] as const
