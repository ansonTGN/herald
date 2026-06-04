import { z } from 'zod'
import { m } from '@/paraglide/messages'

export const creemConfigSchema = z.object({
  enabled: z.boolean().default(false),
  apiKey: z
    .string()
    .regex(/^ck_(test|live)_/, { error: () => m['billing.creem_api_key_format']() })
    .or(z.literal('')),
  timeout: z.number().min(1).max(120).default(30),
  webhookSecret: z
    .string()
    .regex(/^whsec_/, { error: () => m['billing.creem_webhook_start_whsec']() })
    .optional()
    .or(z.literal('')),
})

export type CreemConfigForm = z.infer<typeof creemConfigSchema>

export function getCreemConfigDefaults(initialValues?: Partial<CreemConfigForm>): CreemConfigForm {
  return {
    enabled: initialValues?.enabled ?? false,
    apiKey: initialValues?.apiKey ?? '',
    timeout: initialValues?.timeout ?? 30,
    webhookSecret: initialValues?.webhookSecret ?? '',
  }
}
