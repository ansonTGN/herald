import { z } from 'zod'
import { m } from '@/paraglide/messages'

export type AsyncPointsStrategy = 'conservative' | 'eager'

export const stripeConfigSchema = z.object({
  enabled: z.boolean().default(false),
  publishableKey: z
    .string()
    .regex(/^pk_/, { error: () => m['billing.stripe_key_start_pk']() })
    .or(z.literal('')),
  secretKey: z
    .string()
    .regex(/^sk_/, { error: () => m['billing.stripe_key_start_sk']() })
    .or(z.literal('')),
  webhookSecret: z
    .string()
    .regex(/^whsec_/, { error: () => m['billing.stripe_webhook_start_whsec']() })
    .optional()
    .or(z.literal('')),
  asyncPointsStrategy: z.enum(['conservative', 'eager']).default('conservative'),
})

export type StripeConfigForm = z.infer<typeof stripeConfigSchema>

export function getStripeConfigDefaults(
  initialValues?: Partial<StripeConfigForm>
): StripeConfigForm {
  return {
    enabled: false,
    publishableKey: '',
    secretKey: '',
    webhookSecret: '',
    asyncPointsStrategy: 'conservative',
    ...initialValues,
  }
}
