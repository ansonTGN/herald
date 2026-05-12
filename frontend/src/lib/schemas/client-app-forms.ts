import { z } from 'zod'

// URL 验证规则（参考后端 validation.rs）
const urlSchema = z
  .string()
  .url('Invalid URL format')
  .refine((url) => {
    // 禁止 javascript: 协议
    if (url.toLowerCase().startsWith('javascript:')) {
      return false
    }
    // 禁止协议相对 URL (//example.com)
    if (url.startsWith('//')) {
      return false
    }
    return true
  }, 'Invalid URL format: javascript: protocol and protocol-relative URLs are not allowed')

// Client ID 验证（小写字母、数字、连字符、点号）
const clientIdSchema = z
  .string()
  .min(3, 'Client ID must be at least 3 characters')
  .max(100, 'Client ID must be at most 100 characters')
  .regex(
    /^[a-z0-9.-]+$/,
    'Client ID must contain only lowercase letters, numbers, dots, and hyphens'
  )

// 创建 Client App Schema
export const createClientAppSchema = z.object({
  clientId: clientIdSchema,
  name: z
    .string()
    .min(2, 'Name must be at least 2 characters')
    .max(100, 'Name must be at most 100 characters'),
  description: z.string().max(500, 'Description must be at most 500 characters').optional(),
  redirectUris: z.array(urlSchema).min(1, 'At least one redirect URI is required'),
  iconUrl: z.string().url('Invalid icon URL').optional().or(z.literal('')),
  enabled: z.boolean().default(true),
  sessionTtlSeconds: z
    .number()
    .int('Session TTL must be an integer')
    .min(60, 'Session TTL must be at least 60 seconds')
    .max(86400, 'Session TTL must be at most 86400 seconds (24 hours)')
    .default(1800),
  sessionRenewalTtlSeconds: z
    .number()
    .int('Session renewal TTL must be an integer')
    .min(60, 'Session renewal TTL must be at least 60 seconds')
    .max(86400, 'Session renewal TTL must be at most 86400 seconds (24 hours)')
    .nullable()
    .optional(),
})

// 更新 Client App Schema（所有字段可选）
export const updateClientAppSchema = createClientAppSchema.partial().extend({
  regenerateSecret: z.boolean().optional(),
})

// 类型导出
export type CreateClientAppFormData = z.infer<typeof createClientAppSchema>
export type UpdateClientAppFormData = z.infer<typeof updateClientAppSchema>
