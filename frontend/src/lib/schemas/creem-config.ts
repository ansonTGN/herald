import { z } from 'zod'

export const creemConfigSchema = z.object({
  enabled: z.boolean().default(false),
  apiKey: z
    .string()
    .regex(/^ck_(test|live)_/, 'API key must start with ck_test_ or ck_live_')
    .or(z.literal('')),
  timeout: z.number().min(1).max(120).default(30),
  webhookSecret: z
    .string()
    .regex(/^whsec_/, 'Webhook secret must start with whsec_')
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
