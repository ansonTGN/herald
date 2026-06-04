import { z } from 'zod'
import { m } from '@/paraglide/messages'

/**
 * Schema for creating/updating points plan configuration
 */
export const pointsPlanConfigSchema = z.object({
  planId: z.string().min(1, { error: () => m['points.validation_plan_required']() }),
  pointsPerPeriod: z
    .number()
    .int({ error: () => m['points.validation_points_integer']() })
    .min(0, { error: () => m['points.validation_points_non_negative']() }),
  grantOnSubscribe: z.boolean(),
  grantPeriodType: z.enum(['once', 'daily', 'weekly', 'monthly']),
  maxPeriods: z
    .number()
    .int({ error: () => m['points.validation_max_periods_integer']() })
    .min(0, { error: () => m['points.validation_max_periods_non_negative']() })
    .nullable()
    .optional(),
  validityDays: z
    .number()
    .int({ error: () => m['points.validation_validity_days_integer']() })
    .min(1, { error: () => m['points.validation_validity_days_min']() }),
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
  userId: z.string().min(1, { error: () => m['points.validation_user_required']() }),
  amount: z
    .number()
    .int({ error: () => m['points.validation_amount_integer']() })
    .min(1, { error: () => m['points.validation_amount_min']() }),
  reason: z.string().min(1, { error: () => m['points.validation_reason_required']() }),
  validityDays: z
    .number()
    .int({ error: () => m['points.validation_validity_days_integer']() })
    .min(1, { error: () => m['points.validation_validity_days_min']() })
    .nullable()
    .optional(),
})

export type GrantPointsFormData = z.infer<typeof grantPointsSchema>

/**
 * Schema for realm default configuration
 *
 * Note: This schema matches the backend API response.
 * The backend uses freePeriodic* fields instead of dailyPoints* fields.
 * Use generated types from src/lib/api-generated/ as the source of truth.
 */
export const realmConfigSchema = z
  .object({
    registrationBonusPoints: z
      .number()
      .int({ error: () => m['points.validation_registration_bonus_integer']() })
      .min(0, { error: () => m['points.validation_registration_bonus_non_negative']() }),
    freePeriodicPointsAmount: z
      .number()
      .int({ error: () => m['points.validation_periodic_amount_integer']() })
      .min(0, { error: () => m['points.validation_periodic_amount_non_negative']() }),
    freePeriodicGrantPeriodType: z.enum(['once', 'daily', 'weekly', 'monthly'], {
      error: () => m['points.validation_grant_period_type_invalid'](),
    }),
    freePeriodicValidityDays: z
      .number()
      .int({ error: () => m['points.validation_validity_days_integer']() })
      .min(0, { error: () => m['points.validation_validity_days_non_negative']() }),
  })
  .superRefine((data, ctx) => {
    if (data.freePeriodicGrantPeriodType !== 'once' && data.freePeriodicValidityDays < 1) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: m['points.validation_validity_days_for_periodic'](),
        path: ['freePeriodicValidityDays'],
      })
    }
  })

export type RealmConfigFormData = z.infer<typeof realmConfigSchema>
