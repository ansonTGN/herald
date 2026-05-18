import { describe, it, expect } from 'vitest'
import {
  invoiceCreateFormSchema,
  invoiceEditFormSchema,
  invoiceSellerConfigSchema,
  applyInvoiceSchema,
  voidInvoiceSchema,
  getInvoiceFormDefaults,
  getApplyFormDefaults,
} from '../invoice-forms'

// ---------- Shared valid line item ----------

const validLineItem = {
  name: 'Consulting',
  description: 'Dev work',
  quantity: '2',
  unitPrice: '100',
}

const validCreateForm = {
  accountId: 'acc_123',
  billingName: 'Acme Corp',
  billingEmail: 'billing@acme.com',
  billingAddress: '123 Main St',
  billingPhone: '+1234567890',
  billingTaxId: 'TAX123456',
  sellerName: 'Seller Inc',
  sellerEmail: 'seller@example.com',
  sellerAddress: '456 Oak Ave',
  sellerPhone: '+0987654321',
  sellerTaxId: 'TAX654321',
  currency: 'CNY',
  lineItems: [validLineItem],
  discountMode: null,
  discountValue: null,
  taxMode: null,
  taxValue: null,
  shippingMode: null,
  shippingValue: null,
  dueDate: '2025-12-31',
  paymentTerms: 'Net 30',
  notes: 'Thank you',
  subscriptionId: null,
  paymentAttemptId: null,
}

const validEditForm = {
  billingName: 'Acme Corp',
  billingEmail: 'billing@acme.com',
  billingAddress: '123 Main St',
  billingPhone: '+1234567890',
  billingTaxId: 'TAX123456',
  sellerName: 'Seller Inc',
  sellerEmail: 'seller@example.com',
  sellerAddress: '456 Oak Ave',
  sellerPhone: '+0987654321',
  sellerTaxId: 'TAX654321',
  lineItems: [validLineItem],
  discountMode: null,
  discountValue: null,
  taxMode: null,
  taxValue: null,
  shippingMode: null,
  shippingValue: null,
  dueDate: '2025-12-31',
  paymentTerms: 'Net 30',
  notes: 'Thank you',
}

// ==================== invoiceCreateFormSchema ====================

describe('invoiceCreateFormSchema', () => {
  it('accepts valid full form', () => {
    const result = invoiceCreateFormSchema.safeParse(validCreateForm)
    expect(result.success).toBe(true)
  })

  it('requires accountId', () => {
    const { accountId, ...withoutAccountId } = validCreateForm
    const result = invoiceCreateFormSchema.safeParse(withoutAccountId)
    expect(result.success).toBe(false)
  })

  it('rejects empty accountId', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      accountId: '',
    })
    expect(result.success).toBe(false)
  })

  it('defaults currency to CNY when omitted', () => {
    const { currency, ...withoutCurrency } = validCreateForm
    const result = invoiceCreateFormSchema.safeParse(withoutCurrency)
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.currency).toBe('CNY')
    }
  })

  it('rejects missing billingName', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      billingName: '',
    })
    expect(result.success).toBe(false)
  })

  it('rejects missing sellerName', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      sellerName: '',
    })
    expect(result.success).toBe(false)
  })

  it('rejects empty billingTaxId', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      billingTaxId: '',
    })
    expect(result.success).toBe(false)
  })

  it('rejects empty sellerTaxId', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      sellerTaxId: '',
    })
    expect(result.success).toBe(false)
  })

  it('rejects missing dueDate', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      dueDate: '',
    })
    expect(result.success).toBe(false)
  })

  it('rejects empty lineItems array', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      lineItems: [],
    })
    expect(result.success).toBe(false)
  })

  it('rejects line item with zero quantity', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      lineItems: [{ ...validLineItem, quantity: '0' }],
    })
    expect(result.success).toBe(false)
  })

  it('rejects line item with negative-looking quantity string', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      lineItems: [{ ...validLineItem, quantity: '-1' }],
    })
    expect(result.success).toBe(false)
  })

  it('rejects line item with negative unitPrice', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      lineItems: [{ ...validLineItem, unitPrice: '-100' }],
    })
    expect(result.success).toBe(false)
  })

  it('rejects line item with too many decimal places in unitPrice', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      lineItems: [{ ...validLineItem, unitPrice: '10.123' }],
    })
    expect(result.success).toBe(false)
  })

  it('rejects line item with empty name', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      lineItems: [{ ...validLineItem, name: '' }],
    })
    expect(result.success).toBe(false)
  })

  it('accepts discountMode as fixed', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      discountMode: 'fixed',
      discountValue: '100',
    })
    expect(result.success).toBe(true)
  })

  it('accepts discountMode as percent', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      discountMode: 'percent',
      discountValue: '10',
    })
    expect(result.success).toBe(true)
  })

  it('accepts discountMode as null', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      discountMode: null,
    })
    expect(result.success).toBe(true)
  })

  it('rejects invalid discountMode', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      discountMode: 'invalid',
    })
    expect(result.success).toBe(false)
  })

  it('accepts taxMode as fixed', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      taxMode: 'fixed',
      taxValue: '50',
    })
    expect(result.success).toBe(true)
  })

  it('accepts taxMode as percent', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      taxMode: 'percent',
      taxValue: '5',
    })
    expect(result.success).toBe(true)
  })

  it('rejects invalid taxMode', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      taxMode: 'flat',
    })
    expect(result.success).toBe(false)
  })

  it('accepts shippingMode as fixed', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      shippingMode: 'fixed',
      shippingValue: '20',
    })
    expect(result.success).toBe(true)
  })

  it('rejects shippingMode percent (only fixed allowed)', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      shippingMode: 'percent',
    })
    expect(result.success).toBe(false)
  })

  it('rejects invalid billingEmail', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      billingEmail: 'not-an-email',
    })
    expect(result.success).toBe(false)
  })

  it('accepts optional nullable fields as null', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      billingEmail: null,
      billingPhone: null,
      sellerEmail: null,
      sellerPhone: null,
      paymentTerms: null,
      notes: null,
      subscriptionId: null,
      paymentAttemptId: null,
    })
    expect(result.success).toBe(true)
  })

  it('accepts decimal quantity string', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      lineItems: [{ ...validLineItem, quantity: '1.5' }],
    })
    expect(result.success).toBe(true)
  })

  it('rejects non-numeric quantity', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      lineItems: [{ ...validLineItem, quantity: 'abc' }],
    })
    expect(result.success).toBe(false)
  })

  it('accepts unitPrice of zero', () => {
    const result = invoiceCreateFormSchema.safeParse({
      ...validCreateForm,
      lineItems: [{ ...validLineItem, unitPrice: '0' }],
    })
    expect(result.success).toBe(true)
  })
})

// ==================== invoiceEditFormSchema ====================

describe('invoiceEditFormSchema', () => {
  it('accepts valid full form', () => {
    const result = invoiceEditFormSchema.safeParse(validEditForm)
    expect(result.success).toBe(true)
  })

  it('does not include accountId field', () => {
    const result = invoiceEditFormSchema.safeParse({
      ...validEditForm,
      accountId: 'acc_123',
    })
    // accountId is stripped out since it's not in the schema
    expect(result.success).toBe(true)
    if (result.success) {
      expect((result.data as Record<string, unknown>)['accountId']).toBeUndefined()
    }
  })

  it('rejects missing billingName', () => {
    const result = invoiceEditFormSchema.safeParse({
      ...validEditForm,
      billingName: '',
    })
    expect(result.success).toBe(false)
  })

  it('rejects missing sellerName', () => {
    const result = invoiceEditFormSchema.safeParse({
      ...validEditForm,
      sellerName: '',
    })
    expect(result.success).toBe(false)
  })

  it('rejects empty billingTaxId', () => {
    const result = invoiceEditFormSchema.safeParse({
      ...validEditForm,
      billingTaxId: '',
    })
    expect(result.success).toBe(false)
  })

  it('rejects empty sellerTaxId', () => {
    const result = invoiceEditFormSchema.safeParse({
      ...validEditForm,
      sellerTaxId: '',
    })
    expect(result.success).toBe(false)
  })

  it('rejects missing dueDate', () => {
    const result = invoiceEditFormSchema.safeParse({
      ...validEditForm,
      dueDate: '',
    })
    expect(result.success).toBe(false)
  })

  it('rejects empty lineItems array', () => {
    const result = invoiceEditFormSchema.safeParse({
      ...validEditForm,
      lineItems: [],
    })
    expect(result.success).toBe(false)
  })
})

// ==================== invoiceSellerConfigSchema ====================

describe('invoiceSellerConfigSchema', () => {
  it('accepts valid config', () => {
    const result = invoiceSellerConfigSchema.safeParse({
      sellerName: 'Seller Inc',
      sellerAddress: '456 Oak Ave',
      sellerEmail: 'seller@example.com',
      sellerPhone: '+1234567890',
      sellerTaxId: 'TAX654321',
      defaultPaymentTerms: 'Net 30',
    })
    expect(result.success).toBe(true)
  })

  it('rejects missing sellerName', () => {
    const result = invoiceSellerConfigSchema.safeParse({
      sellerName: '',
    })
    expect(result.success).toBe(false)
  })

  it('accepts config with only sellerName, sellerAddress and sellerTaxId', () => {
    const result = invoiceSellerConfigSchema.safeParse({
      sellerName: 'Seller Inc',
      sellerAddress: '456 Oak Ave',
      sellerTaxId: 'TAX123',
    })
    expect(result.success).toBe(true)
  })

  it('rejects missing sellerTaxId', () => {
    const result = invoiceSellerConfigSchema.safeParse({
      sellerName: 'Seller Inc',
    })
    expect(result.success).toBe(false)
  })

  it('accepts optional fields as null', () => {
    const result = invoiceSellerConfigSchema.safeParse({
      sellerName: 'Seller Inc',
      sellerAddress: '456 Oak Ave',
      sellerEmail: null,
      sellerPhone: null,
      sellerTaxId: 'TAX123',
      defaultPaymentTerms: null,
    })
    expect(result.success).toBe(true)
  })

  it('accepts optional fields as undefined (omitted)', () => {
    const result = invoiceSellerConfigSchema.safeParse({
      sellerName: 'Seller Inc',
      sellerAddress: '456 Oak Ave',
      sellerTaxId: 'TAX123',
    })
    expect(result.success).toBe(true)
  })

  it('rejects invalid sellerEmail', () => {
    const result = invoiceSellerConfigSchema.safeParse({
      sellerName: 'Seller Inc',
      sellerEmail: 'not-an-email',
    })
    expect(result.success).toBe(false)
  })
})

// ==================== applyInvoiceSchema ====================

describe('applyInvoiceSchema', () => {
  it('accepts valid form with paymentAttemptId', () => {
    const result = applyInvoiceSchema.safeParse({
      currency: 'CNY',
      paymentAttemptId: 'pa_123',
      subscriptionId: null,
      billingName: 'Acme Corp',
      billingEmail: null,
      billingAddress: '123 Main St',
      billingPhone: null,
      dueDate: '2025-08-01',
      notes: null,
    })
    expect(result.success).toBe(true)
  })

  it('accepts valid form with subscriptionId', () => {
    const result = applyInvoiceSchema.safeParse({
      currency: 'CNY',
      paymentAttemptId: null,
      subscriptionId: 'sub_123',
      billingName: 'Acme Corp',
      billingEmail: null,
      billingAddress: '123 Main St',
      billingPhone: null,
      dueDate: '2025-08-01',
      notes: null,
    })
    expect(result.success).toBe(true)
  })

  it('accepts valid form with both paymentAttemptId and subscriptionId', () => {
    const result = applyInvoiceSchema.safeParse({
      currency: 'CNY',
      paymentAttemptId: 'pa_123',
      subscriptionId: 'sub_123',
      billingName: 'Acme Corp',
      billingAddress: '123 Main St',
      dueDate: '2025-08-01',
    })
    expect(result.success).toBe(true)
  })

  it('rejects when both paymentAttemptId and subscriptionId are missing', () => {
    const result = applyInvoiceSchema.safeParse({
      currency: 'CNY',
      paymentAttemptId: null,
      subscriptionId: null,
      billingName: 'Acme Corp',
      billingAddress: '123 Main St',
      dueDate: '2025-08-01',
    })
    expect(result.success).toBe(false)
  })

  it('rejects when both paymentAttemptId and subscriptionId are empty strings', () => {
    const result = applyInvoiceSchema.safeParse({
      currency: 'CNY',
      paymentAttemptId: '',
      subscriptionId: '',
      billingName: 'Acme Corp',
      billingAddress: '123 Main St',
      dueDate: '2025-08-01',
    })
    // Empty strings are falsy, so the refine will fail
    expect(result.success).toBe(false)
  })

  it('rejects missing billingName', () => {
    const result = applyInvoiceSchema.safeParse({
      currency: 'CNY',
      paymentAttemptId: 'pa_123',
      subscriptionId: null,
      billingName: '',
      billingAddress: '123 Main St',
      dueDate: '2025-08-01',
    })
    expect(result.success).toBe(false)
  })

  it('defaults currency to CNY when omitted', () => {
    const result = applyInvoiceSchema.safeParse({
      paymentAttemptId: 'pa_123',
      subscriptionId: null,
      billingName: 'Acme Corp',
      billingAddress: '123 Main St',
      dueDate: '2025-08-01',
    })
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.currency).toBe('CNY')
    }
  })
})

// ==================== voidInvoiceSchema ====================

describe('voidInvoiceSchema', () => {
  it('accepts valid form with voidReason', () => {
    const result = voidInvoiceSchema.safeParse({
      voidReason: 'Customer requested cancellation',
    })
    expect(result.success).toBe(true)
  })

  it('accepts empty object (voidReason is optional)', () => {
    const result = voidInvoiceSchema.safeParse({})
    expect(result.success).toBe(true)
  })

  it('accepts voidReason as null', () => {
    const result = voidInvoiceSchema.safeParse({
      voidReason: null,
    })
    expect(result.success).toBe(true)
  })

  it('accepts voidReason as undefined (omitted)', () => {
    const result = voidInvoiceSchema.safeParse({
      voidReason: undefined,
    })
    expect(result.success).toBe(true)
  })

  it('rejects voidReason exceeding max length', () => {
    const result = voidInvoiceSchema.safeParse({
      voidReason: 'a'.repeat(501),
    })
    expect(result.success).toBe(false)
  })

  it('accepts voidReason at max length boundary', () => {
    const result = voidInvoiceSchema.safeParse({
      voidReason: 'a'.repeat(500),
    })
    expect(result.success).toBe(true)
  })
})

// ==================== Helper Functions ====================

describe('getInvoiceFormDefaults', () => {
  it('returns correct default shape without seller config', () => {
    const defaults = getInvoiceFormDefaults()

    expect(defaults).toEqual({
      accountId: '',
      billingName: '',
      billingEmail: null,
      billingAddress: '',
      billingPhone: null,
      billingTaxId: '',
      sellerName: '',
      sellerEmail: null,
      sellerAddress: '',
      sellerPhone: null,
      sellerTaxId: '',
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
    })
  })

  it('merges seller config when provided', () => {
    const defaults = getInvoiceFormDefaults({
      sellerName: 'Seller Inc',
      sellerAddress: '456 Oak Ave',
      sellerEmail: 'seller@example.com',
      sellerPhone: '+1234567890',
      sellerTaxId: 'TAX654321',
    })

    expect(defaults.sellerName).toBe('Seller Inc')
    expect(defaults.sellerAddress).toBe('456 Oak Ave')
    expect(defaults.sellerEmail).toBe('seller@example.com')
    expect(defaults.sellerPhone).toBe('+1234567890')
    expect(defaults.sellerTaxId).toBe('TAX654321')
  })

  it('handles null seller config gracefully', () => {
    const defaults = getInvoiceFormDefaults(null)

    expect(defaults.sellerName).toBe('')
    expect(defaults.sellerEmail).toBe(null)
  })

  it('handles partial seller config', () => {
    const defaults = getInvoiceFormDefaults({
      sellerName: 'Seller Inc',
    })

    expect(defaults.sellerName).toBe('Seller Inc')
    expect(defaults.sellerAddress).toBe('')
    expect(defaults.sellerEmail).toBe(null)
    expect(defaults.sellerPhone).toBe(null)
  })
})

describe('getApplyFormDefaults', () => {
  it('returns correct default shape', () => {
    const defaults = getApplyFormDefaults()

    expect(defaults).toEqual({
      currency: 'CNY',
      paymentAttemptId: null,
      subscriptionId: null,
      billingName: '',
      billingEmail: null,
      billingAddress: '',
      billingPhone: null,
      dueDate: '',
      notes: null,
    })
  })
})
