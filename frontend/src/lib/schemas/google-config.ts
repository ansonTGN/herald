import { z } from 'zod'

export const googlePlayConfigSchema = z.object({
  packageName: z.string(),
  // Sensitive: allow empty string so edits can leave it blank to keep existing.
  serviceAccountJson: z.string().or(z.literal('')),
})

export type GooglePlayConfigForm = z.infer<typeof googlePlayConfigSchema>

export function getGooglePlayConfigDefaults(
  initialValues?: Partial<GooglePlayConfigForm>
): GooglePlayConfigForm {
  return {
    packageName: initialValues?.packageName ?? '',
    serviceAccountJson: initialValues?.serviceAccountJson ?? '',
  }
}
