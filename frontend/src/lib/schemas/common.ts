import { z } from 'zod'

export const emailSchema = z.string().min(1, 'Email is required').email('Invalid email address')
export const passwordSchema = z.string().min(8, 'Password must be at least 8 characters')
export const usernameSchema = z.string().min(3, 'Username must be at least 3 characters')

export const loginSchema = z.object({
  username: usernameSchema.or(emailSchema),
  password: passwordSchema,
})

export const createUserSchema = z.object({
  email: emailSchema,
  password: passwordSchema,
  nickname: z.string().min(2).max(50).optional(),
  status: z.number().int().min(0).max(3).optional(),
  roleIds: z.array(z.string()).min(1, 'At least one role is required'),
})

export const updateUserSchema = z.object({
  email: emailSchema,
  nickname: z.string().min(2).max(50).optional(),
  status: z.number().int().min(0).max(3).optional(),
})

export const changePasswordSchema = z
  .object({
    oldPass: z.string().min(1, 'Current password is required'),
    newPass: passwordSchema,
    confirmPass: z.string().min(8, 'Password must be at least 8 characters'),
  })
  .superRefine((data, ctx) => {
    if (data.newPass !== data.confirmPass) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "Passwords don't match",
        path: ['confirmPass'],
      })
    }
  })

export type LoginFormData = z.infer<typeof loginSchema>
export type CreateUserData = z.infer<typeof createUserSchema>
export type CreateUserFormData = z.infer<typeof createUserSchema>
export type UpdateUserFormData = z.infer<typeof updateUserSchema>
export type ChangePasswordFormData = z.infer<typeof changePasswordSchema>

// Permission schemas
export const permissionNameSchema = z
  .string()
  .min(1, 'Permission name is required')
  .max(100, 'Permission name must be at most 100 characters')
  .regex(
    /^[a-z0-9_]+\.[a-z0-9_]+$/,
    'Permission name must be in format "resource.action" (e.g., "users.view")'
  )
  .refine((val) => !val.includes(' '), 'Permission name cannot contain spaces')
  .refine((val) => !val.includes('..'), 'Permission name cannot contain ".."')

export const permissionDescriptionSchema = z
  .string()
  .max(500, 'Description must be at most 500 characters')
  .optional()

export const createPermissionSchema = z.object({
  name: permissionNameSchema,
  description: permissionDescriptionSchema,
})

export const updatePermissionSchema = z.object({
  name: permissionNameSchema,
  description: permissionDescriptionSchema,
})

export type CreatePermissionFormData = z.infer<typeof createPermissionSchema>
export type UpdatePermissionFormData = z.infer<typeof updatePermissionSchema>

// Role schemas
export const roleNameSchema = z
  .string()
  .min(1, 'Role name is required')
  .max(100, 'Role name must be at most 100 characters')
  .regex(
    /^[a-zA-Z0-9_-]+$/,
    'Role name must contain only letters, numbers, hyphens, and underscores'
  )
  .refine((val) => !val.includes(' '), 'Role name cannot contain spaces')
  .refine((val) => !val.includes('--'), 'Role name cannot contain consecutive hyphens')

export const roleDescriptionSchema = z
  .string()
  .max(500, 'Description must be at most 500 characters')
  .optional()

export const createRoleSchema = z.object({
  name: roleNameSchema,
  description: roleDescriptionSchema,
})

export const updateRoleSchema = z.object({
  name: roleNameSchema,
  description: roleDescriptionSchema,
})

export type CreateRoleFormData = z.infer<typeof createRoleSchema>
export type UpdateRoleFormData = z.infer<typeof updateRoleSchema>
