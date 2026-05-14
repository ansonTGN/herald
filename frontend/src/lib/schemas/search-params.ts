import { z } from 'zod'

export const paginationSchema = z.object({
  page: z.number().int().min(0).optional(),
  page_size: z.number().int().min(1).max(100).optional(),
})

export const loginSearchSchema = z.object({
  redirect: z.string().optional(),
  clientId: z.string().optional(),
})

export const usersSearchSchema = z.object({
  page: z.number().int().min(0).optional(),
  pageSize: z.number().int().min(1).max(100).optional(),
  email: z.string().optional(),
})

export const realmsSearchSchema = z.object({
  page: z.number().int().min(0).optional(),
  pageSize: z.number().int().min(1).max(100).optional(),
  search: z.string().optional(),
  sortBy: z.string().optional(),
  sortOrder: z.enum(['asc', 'desc']).optional(),
})

export const clientAppsSearchSchema = z.object({
  page: z.number().int().min(0).optional(),
  pageSize: z.number().int().min(1).max(100).optional(),
})

export const auditSearchSchema = z.object({
  page: z.number().int().min(0).optional(),
  pageSize: z.number().int().min(1).max(100).optional(),
  category: z.string().optional(),
  action: z.string().optional(),
  actorId: z.string().optional(),
  startTime: z.string().optional(),
  endTime: z.string().optional(),
})

export type PaginationParams = z.infer<typeof paginationSchema>
export type LoginSearchParams = z.infer<typeof loginSearchSchema>
export type UsersSearchParams = z.infer<typeof usersSearchSchema>
export type RealmsSearchParams = z.infer<typeof realmsSearchSchema>
export type ClientAppsSearchParams = z.infer<typeof clientAppsSearchSchema>
export type AuditSearchParams = z.infer<typeof auditSearchSchema>
