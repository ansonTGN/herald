import { z } from 'zod'

export const appleConfigSchema = z.object({
  bundleId: z.string(),
  issuerId: z.string(),
  keyId: z.string(),
  // Sensitive: allow empty string so edits can leave it blank to keep existing.
  privateKeyP8: z.string().or(z.literal('')),
  environment: z.enum(['sandbox', 'production']).default('production'),
})

export type AppleIapConfigForm = z.infer<typeof appleConfigSchema>

export function getAppleIapConfigDefaults(
  initialValues?: Partial<AppleIapConfigForm>
): AppleIapConfigForm {
  return {
    bundleId: initialValues?.bundleId ?? '',
    issuerId: initialValues?.issuerId ?? '',
    keyId: initialValues?.keyId ?? '',
    privateKeyP8: initialValues?.privateKeyP8 ?? '',
    environment: initialValues?.environment ?? 'production',
  }
}
