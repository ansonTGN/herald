import { z } from 'zod'

// TOTP 配置 Schema
// ✅ 前端 Schema 使用 camelCase（符合 JavaScript 约定）
export const totpConfigSchema = z.object({
  enabled: z.boolean(), // Realm 是否启用 TOTP
  forceEnabled: z.boolean(), // ✅ camelCase：是否强制所有用户启用 TOTP
})

// Registration 配置 Schema
export const registrationConfigSchema = z.object({
  enabled: z.boolean(), // 是否允许注册
  requireEmailVerification: z.boolean(), // ✅ camelCase：是否需要邮箱验证
})

// Turnstile 配置 Schema
export const turnstileConfigSchema = z.object({
  siteKey: z.string(),
  secretKey: z.string(),
})

// Email 配置 Schema
export const emailConfigSchema = z.object({
  provider: z.enum(['resend', 'smtp']),
  fromAddress: z.string().email().or(z.literal('')),
  resendApiKey: z.string().optional(),
  smtpHost: z.string().optional(),
  smtpPort: z.string().default('587'),
  smtpUsername: z.string().optional(),
  smtpPassword: z.string().optional(),
  smtpEncryption: z.enum(['starttls', 'ssl']).default('starttls'),
})

// 类型导出
export type TOTPConfigForm = z.infer<typeof totpConfigSchema>
export type RegistrationConfigForm = z.infer<typeof registrationConfigSchema>
export type TurnstileConfigForm = z.infer<typeof turnstileConfigSchema>
export type EmailConfigForm = z.infer<typeof emailConfigSchema>
