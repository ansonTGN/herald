import { z } from 'zod'

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

export function getStripeConfigDefaults(initialValues?: Partial<StripeConfigForm>): StripeConfigForm {
  return {
    enabled: false,
    publishableKey: '',
    secretKey: '',
    webhookSecret: '',
    ...initialValues,
  }
}
