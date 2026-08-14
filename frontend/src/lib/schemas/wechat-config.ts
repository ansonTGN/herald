import { z } from 'zod'

export const wechatConfigSchema = z.object({
  appId: z.string(),
  mchId: z.string(),
  // Sensitive: allow empty string so edits can leave it blank to keep existing.
  privateKey: z.string().or(z.literal('')),
  serialNo: z.string(),
  v3Key: z.string().or(z.literal('')),
  notifyUrl: z.string(),
  platformPublicKey: z.string().or(z.literal('')),
})

export type WechatConfigForm = z.infer<typeof wechatConfigSchema>

export function getWechatConfigDefaults(
  initialValues?: Partial<WechatConfigForm>
): WechatConfigForm {
  return {
    appId: initialValues?.appId ?? '',
    mchId: initialValues?.mchId ?? '',
    privateKey: initialValues?.privateKey ?? '',
    serialNo: initialValues?.serialNo ?? '',
    v3Key: initialValues?.v3Key ?? '',
    notifyUrl: initialValues?.notifyUrl ?? '',
    platformPublicKey: initialValues?.platformPublicKey ?? '',
  }
}
