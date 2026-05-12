import { z } from 'zod'

// 保留词列表
const RESERVED_WORDS = ['admin', 'system', 'api', 'www'] as const

// Realm ID 验证（必填）
// 注意：后端只验证长度 (3-64)，没有格式限制
// 前端验证可以更宽松，允许连字符和下划线
export const realmIdSchema = z
  .string()
  .min(3, '3-64 characters')
  .max(64, '3-64 characters')
  .regex(/^[a-zA-Z0-9_-]+$/, 'must be alphanumeric (letters, numbers, hyphens, underscores)')
  .refine((val) => !RESERVED_WORDS.some((word) => word === val), {
    message: 'cannot be a reserved word',
  })

// 管理员用户信息
const adminUserSchema = z.object({
  email: z.string().email('Invalid email address'),
  password: z.string().min(8, 'at least 8 characters'),
})

// 创建 Realm Schema
// 注意：字段名使用 camelCase 匹配后端 API 类型（CreateRealmValidator.adminUser）
// 后端 CreateRealmValidator.id 是 Optional<String>，所以前端也设为可选
export const createRealmSchema = z.object({
  id: realmIdSchema.optional(), // ID is optional - backend will auto-generate if not provided
  name: z.string().min(1, 'Name is required').max(50, 'Name must be at most 50 characters'),
  adminUser: adminUserSchema, // camelCase 匹配 API
})

// 更新 Realm Schema（仅 name 字段）
export const updateRealmSchema = z.object({
  name: z.string().min(1, 'Name is required').max(50, 'Name must be at most 50 characters'),
})

// 类型导出
export type CreateRealmFormData = z.infer<typeof createRealmSchema>
export type UpdateRealmFormData = z.infer<typeof updateRealmSchema>
