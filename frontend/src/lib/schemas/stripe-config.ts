import { z } from 'zod'

/**
 * Stripe 配置 Schema
 * 用于 Realm Admin 配置 Stripe 支付平台
 */
export const stripeConfigSchema = z.object({
  enabled: z.boolean().default(false),
  publishableKey: z
    .string()
    .min(1, 'Publishable key is required')
    .regex(/^pk_/, 'Publishable key must start with pk_'),
  secretKey: z
    .string()
    .min(1, 'Secret key is required')
    .regex(/^sk_/, 'Secret key must start with sk_'),
  webhookSecret: z
    .string()
    .regex(/^whsec_/, 'Webhook secret must start with whsec_')
    .optional()
    .or(z.literal('')),
})

export type StripeConfigForm = z.infer<typeof stripeConfigSchema>

/**
 * Get default values for Stripe configuration form
 */
export function getStripeConfigDefaults(): StripeConfigForm {
  return {
    enabled: false,
    publishableKey: '',
    secretKey: '',
    webhookSecret: '',
  }
}
