import { createFileRoute } from '@tanstack/react-router'
import { z } from 'zod'
import { ApplyInvoicePageRoute } from '@/routes/$realmId/user/invoices/new'

const invoiceApplySearchSchema = z
  .object({
    paymentAttemptId: z.string().uuid().optional(),
    subscriptionId: z.string().uuid().optional(),
    returnTo: z.string().optional(),
  })
  .refine((search) => !search.paymentAttemptId !== !search.subscriptionId, {
    message: 'Exactly one invoice reference is required',
  })

export const Route = createFileRoute('/user/invoices/new')({
  validateSearch: (search) => invoiceApplySearchSchema.parse(search),
  component: ApplyInvoicePageRoute,
})
