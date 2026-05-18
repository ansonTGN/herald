import { z } from 'zod'

export const invoiceLineItemSchema = z.object({
  name: z.string().min(1, 'Item name is required'),
  description: z.string().max(500).optional().nullable(),
  quantity: z
    .string()
    .min(1, 'Quantity is required')
    .refine(
      (val) => /^[0-9]+(\.[0-9]+)?$/.test(val) && parseFloat(val) > 0,
      'Quantity must be a positive decimal (e.g. 1.5)'
    ),
  unitPrice: z
    .string()
    .min(1, 'Unit price is required')
    .refine(
      (val) => /^[0-9]+(\.[0-9]{0,2})?$/.test(val) && parseFloat(val) >= 0,
      'Unit price must be a non-negative decimal (e.g. 12.32)'
    ),
})

export type InvoiceLineItemFormData = z.infer<typeof invoiceLineItemSchema>

const discountModeSchema = z.enum(['fixed', 'percent']).nullable().optional()
const taxModeSchema = z.enum(['fixed', 'percent']).nullable().optional()
const shippingModeSchema = z.enum(['fixed']).nullable().optional()

const numericStringSchema = z
  .string()
  .refine(
    (val) => val === '' || val === null || val === undefined || /^[0-9]+(\.[0-9]+)?$/.test(val),
    'Must be a valid number'
  )
  .nullable()
  .optional()

export const invoiceCreateFormSchema = z.object({
  accountId: z.string().min(1, 'Account is required'),
  billingName: z.string().min(1, 'Billing name is required'),
  billingEmail: z.string().email('Invalid email').optional().nullable(),
  billingAddress: z.string().min(1, 'Billing address is required'),
  billingPhone: z.string().max(50).optional().nullable(),
  billingTaxId: z.string().min(1, 'Tax ID is required'),
  sellerName: z.string().min(1, 'Seller name is required'),
  sellerEmail: z.string().email('Invalid email').optional().nullable(),
  sellerAddress: z.string().min(1, 'Seller address is required'),
  sellerPhone: z.string().max(50).optional().nullable(),
  sellerTaxId: z.string().min(1, 'Tax ID is required'),
  currency: z.string().min(3).max(3).default('CNY'),
  lineItems: z.array(invoiceLineItemSchema).min(1, 'At least one line item is required'),
  discountMode: discountModeSchema,
  discountValue: numericStringSchema,
  taxMode: taxModeSchema,
  taxValue: numericStringSchema,
  shippingMode: shippingModeSchema,
  shippingValue: numericStringSchema,
  dueDate: z.string().min(1, 'Due date is required'),
  paymentTerms: z.string().max(200).optional().nullable(),
  notes: z.string().max(2000).optional().nullable(),
  subscriptionId: z.string().optional().nullable(),
  paymentAttemptId: z.string().optional().nullable(),
})

export type InvoiceCreateFormData = z.infer<typeof invoiceCreateFormSchema>

export const invoiceEditFormSchema = z.object({
  billingName: z.string().min(1, 'Billing name is required'),
  billingEmail: z.string().email('Invalid email').optional().nullable(),
  billingAddress: z.string().min(1, 'Billing address is required'),
  billingPhone: z.string().max(50).optional().nullable(),
  billingTaxId: z.string().min(1, 'Tax ID is required'),
  sellerName: z.string().min(1, 'Seller name is required'),
  sellerEmail: z.string().email('Invalid email').optional().nullable(),
  sellerAddress: z.string().min(1, 'Seller address is required'),
  sellerPhone: z.string().max(50).optional().nullable(),
  sellerTaxId: z.string().min(1, 'Tax ID is required'),
  lineItems: z.array(invoiceLineItemSchema).min(1, 'At least one line item is required'),
  discountMode: discountModeSchema,
  discountValue: numericStringSchema,
  taxMode: taxModeSchema,
  taxValue: numericStringSchema,
  shippingMode: shippingModeSchema,
  shippingValue: numericStringSchema,
  dueDate: z.string().min(1, 'Due date is required'),
  paymentTerms: z.string().max(200).optional().nullable(),
  notes: z.string().max(2000).optional().nullable(),
})

export type InvoiceEditFormData = z.infer<typeof invoiceEditFormSchema>

export const invoiceSellerConfigSchema = z.object({
  sellerName: z.string().min(1, 'Seller name is required'),
  sellerAddress: z.string().min(1, 'Seller address is required'),
  sellerEmail: z.string().email('Invalid email').optional().nullable(),
  sellerPhone: z.string().max(50).optional().nullable(),
  sellerTaxId: z.string().min(1, 'Tax ID is required'),
  defaultPaymentTerms: z.string().max(200).optional().nullable(),
})

export type InvoiceSellerConfigFormData = z.infer<typeof invoiceSellerConfigSchema>

export const applyInvoiceSchema = z
  .object({
    currency: z.string().min(3).max(3).default('CNY'),
    paymentAttemptId: z.string().optional().nullable(),
    subscriptionId: z.string().optional().nullable(),
    billingName: z.string().min(1, 'Billing name is required'),
    billingEmail: z.string().email('Invalid email').optional().nullable(),
    billingAddress: z.string().min(1, 'Billing address is required'),
    billingPhone: z.string().max(50).optional().nullable(),
    dueDate: z.string().min(1, 'Due date is required'),
    notes: z.string().max(2000).optional().nullable(),
  })
  .refine((data) => data.paymentAttemptId || data.subscriptionId, {
    message: 'Either payment attempt or subscription is required',
    path: ['paymentAttemptId'],
  })

export type ApplyInvoiceFormData = z.infer<typeof applyInvoiceSchema>

export const voidInvoiceSchema = z.object({
  voidReason: z.string().max(500).optional().nullable(),
})

export type VoidInvoiceFormData = z.infer<typeof voidInvoiceSchema>

export function getInvoiceFormDefaults(
  sellerConfig?: {
    sellerName?: string
    sellerAddress?: string | null
    sellerEmail?: string | null
    sellerPhone?: string | null
    sellerTaxId?: string
  } | null
): InvoiceCreateFormData {
  return {
    accountId: '',
    billingName: '',
    billingEmail: null,
    billingAddress: '',
    billingPhone: null,
    billingTaxId: '',
    sellerName: sellerConfig?.sellerName ?? '',
    sellerEmail: sellerConfig?.sellerEmail ?? null,
    sellerAddress: sellerConfig?.sellerAddress ?? '',
    sellerPhone: sellerConfig?.sellerPhone ?? null,
    sellerTaxId: sellerConfig?.sellerTaxId ?? '',
    currency: 'CNY',
    lineItems: [{ name: '', description: null, quantity: '1', unitPrice: '0.00' }],
    discountMode: null,
    discountValue: null,
    taxMode: null,
    taxValue: null,
    shippingMode: null,
    shippingValue: null,
    dueDate: '',
    paymentTerms: null,
    notes: null,
    subscriptionId: null,
    paymentAttemptId: null,
  }
}

export function getApplyFormDefaults(): ApplyInvoiceFormData {
  return {
    currency: 'CNY',
    paymentAttemptId: null,
    subscriptionId: null,
    billingName: '',
    billingEmail: null,
    billingAddress: '',
    billingPhone: null,
    dueDate: '',
    notes: null,
  }
}
