import { z } from 'zod'

/**
 * Schema for creating/updating points plan configuration
 */
export const pointsPlanConfigSchema = z.object({
  planId: z.string().min(1, 'Plan is required'),
  pointsPerPeriod: z.number().int('Points must be an integer').min(0, 'Points cannot be negative'),
  grantOnSubscribe: z.boolean(),
  grantPeriodType: z.enum(['once', 'daily', 'weekly', 'monthly']),
  maxPeriods: z
    .number()
    .int('Max periods must be an integer')
    .min(0, 'Max periods cannot be negative')
    .nullable()
    .optional(),
  validityDays: z
    .number()
    .int('Validity days must be an integer')
    .min(1, 'Validity days must be at least 1'),
})

export type PointsPlanConfigFormData = z.infer<typeof pointsPlanConfigSchema>

/**
 * Schema for transaction filters
 */
export const transactionFiltersSchema = z.object({
  transactionType: z.enum(['recharge', 'consume']).optional(),
  startTime: z.string().datetime().optional(),
  endTime: z.string().datetime().optional(),
  clientAppId: z.string().optional(),
})

export type TransactionFilters = z.infer<typeof transactionFiltersSchema>

/**
 * Schema for account filters
 */
export const accountFiltersSchema = z.object({
  search: z.string().optional(),
  status: z.enum(['active', 'frozen', 'closed']).optional(),
})

export type AccountFilters = z.infer<typeof accountFiltersSchema>

/**
 * Schema for granting points to a user
 */
export const grantPointsSchema = z.object({
  userId: z.string().min(1, 'User is required'),
  amount: z.number().int('Amount must be an integer').min(1, 'Amount must be at least 1'),
  reason: z.string().min(1, 'Reason is required'),
  validityDays: z
    .number()
    .int('Validity days must be an integer')
    .min(1, 'Validity days must be at least 1')
    .nullable()
    .optional(),
})

export type GrantPointsFormData = z.infer<typeof grantPointsSchema>

/**
 * Schema for realm default configuration
 *
 * ⚠️ Note: This schema matches the backend API response.
 * The backend uses freePeriodic* fields instead of dailyPoints* fields.
 * Use generated types from src/lib/api-generated/ as the source of truth.
 */
export const realmConfigSchema = z
  .object({
    registrationBonusPoints: z
      .number()
      .int('Registration bonus points must be an integer')
      .min(0, 'Registration bonus points cannot be negative'),
    freePeriodicPointsAmount: z
      .number()
      .int('Periodic points amount must be an integer')
      .min(0, 'Periodic points amount cannot be negative'),
    freePeriodicGrantPeriodType: z.enum(['once', 'daily', 'weekly', 'monthly'], {
      message: 'Please select a valid grant period type',
    }),
    freePeriodicValidityDays: z
      .number()
      .int('Validity days must be an integer')
      .min(0, 'Validity days cannot be negative'),
  })
  .superRefine((data, ctx) => {
    // Cross-field validation: once 周期允许 0（永久有效），其他周期要求 >= 1
    if (data.freePeriodicGrantPeriodType !== 'once' && data.freePeriodicValidityDays < 1) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: 'Validity days must be >= 1 for non-once periods (0 allowed for once period)',
        path: ['freePeriodicValidityDays'],
      })
    }
  })

export type RealmConfigFormData = z.infer<typeof realmConfigSchema>
