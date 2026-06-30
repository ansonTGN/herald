import { z } from 'zod'
import { m } from '@/paraglide/messages'

export const recordRefundFormSchema = z.object({
  amount: z
    .string()
    .min(1, { error: () => m['billing.credit_note_validation_amount_required']() })
    .refine((v) => /^[0-9]+(\.[0-9]{0,2})?$/.test(v) && parseFloat(v) > 0, {
      error: () => m['billing.credit_note_validation_amount_positive'](),
    }),
  memo: z
    .string()
    .min(1, { error: () => m['billing.credit_note_validation_memo_required']() })
    .max(500, { error: () => m['billing.credit_note_validation_memo_max']() }),
})

export type RecordRefundFormData = z.infer<typeof recordRefundFormSchema>
