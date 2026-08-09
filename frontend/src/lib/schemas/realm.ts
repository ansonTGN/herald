import { z } from 'zod'
import { m } from '@/paraglide/messages'

// Reserved realm identifiers (shared by realm id + self-service signup slug validation).
export const RESERVED_WORDS = ['admin', 'system', 'api', 'www'] as const

// Realm ID 验证（必填）
// 注意：后端只验证长度 (3-64)，没有格式限制
// 前端验证可以更宽松，允许连字符和下划线
export const realmIdSchema = z
  .string()
  .min(3, { error: () => m['realms.validation_id_length']() })
  .max(64, { error: () => m['realms.validation_id_length']() })
  .regex(/^[a-zA-Z0-9_-]+$/, { error: () => m['realms.validation_id_format']() })
  .refine((val) => !RESERVED_WORDS.some((word) => word === val), {
    error: () => m['realms.validation_id_reserved'](),
  })

// 管理员用户信息
const adminUserSchema = z.object({
  email: z.string().email({ error: () => m['realms.validation_admin_email_invalid']() }),
  password: z.string().min(8, { error: () => m['realms.validation_admin_password_min_length']() }),
})

// 创建 Realm Schema
// 注意：字段名使用 camelCase 匹配后端 API 类型（CreateRealmValidator.adminUser）
// 后端 CreateRealmValidator.id 是 Optional<String>，所以前端也设为可选
export const createRealmSchema = z.object({
  id: realmIdSchema.optional(), // ID is optional - backend will auto-generate if not provided
  name: z
    .string()
    .min(1, { error: () => m['realms.validation_name_required']() })
    .max(50, { error: () => m['realms.validation_name_max_length']() }),
  description: z.string().optional(),
  adminUser: adminUserSchema, // camelCase 匹配 API
})

// 更新 Realm Schema
export const updateRealmSchema = z.object({
  name: z
    .string()
    .min(1, { error: () => m['realms.validation_name_required']() })
    .max(50, { error: () => m['realms.validation_name_max_length']() }),
  description: z.string().optional(),
})

// 类型导出
export type CreateRealmFormData = z.infer<typeof createRealmSchema>
export type UpdateRealmFormData = z.infer<typeof updateRealmSchema>
