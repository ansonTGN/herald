-- ====================================
-- Invoice Module Migration
-- ====================================
-- Tables: invoice_seller_config, invoice, invoice_line_item,
--         invoice_history, invoice_number_counter

-- ====================================
-- Invoice Seller Config
-- ====================================
-- Realm-level seller configuration, auto-filled when creating invoices.
CREATE TABLE invoice_seller_config (
    realm_id TEXT PRIMARY KEY REFERENCES realm(id) ON DELETE CASCADE,
    seller_name TEXT NOT NULL,
    seller_address TEXT NOT NULL CHECK (BTRIM(seller_address) <> ''),
    seller_email TEXT,
    seller_phone TEXT,
    seller_tax_id TEXT NOT NULL,
    default_payment_terms TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE invoice_seller_config IS 'Realm-level seller configuration for invoice creation';
COMMENT ON COLUMN invoice_seller_config.seller_name IS 'Seller legal name';
COMMENT ON COLUMN invoice_seller_config.default_payment_terms IS 'Default payment terms text applied to new invoices';

-- ====================================
-- Invoice
-- ====================================
CREATE TABLE invoice (
    id UUID PRIMARY KEY,
    realm_id TEXT NOT NULL REFERENCES realm(id) ON DELETE CASCADE,
    invoice_number TEXT NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('admin_manual', 'user_application', 'external_sync')),
    provider VARCHAR(20) NOT NULL DEFAULT 'manual',
    payment_provider VARCHAR(20),
    account_id UUID,
    applicant_user_id UUID,
    subscription_id UUID REFERENCES subscription(id) ON DELETE SET NULL,
    payment_attempt_id UUID,
    status TEXT NOT NULL CHECK (status IN ('draft', 'issued', 'paid', 'void', 'overdue')),
    currency text NOT NULL,

    -- Dates
    issue_date DATE,
    due_date DATE,
    issued_at TIMESTAMPTZ,
    paid_at TIMESTAMPTZ,
    voided_at TIMESTAMPTZ,

    -- Monetary amounts in smallest currency unit (e.g. CNY cents)
    subtotal BIGINT NOT NULL DEFAULT 0 CHECK (subtotal >= 0),
    discount_amount BIGINT NOT NULL DEFAULT 0,
    tax_amount BIGINT NOT NULL DEFAULT 0,
    shipping_amount BIGINT NOT NULL DEFAULT 0,
    total BIGINT NOT NULL DEFAULT 0 CHECK (total >= 0 OR provider != 'manual'),

    -- Discount/tax/shipping mode and raw input value
    discount_mode TEXT CHECK (discount_mode IN ('fixed', 'percent')),
    discount_value NUMERIC(12, 4),
    tax_mode TEXT CHECK (tax_mode IN ('fixed', 'percent')),
    tax_value NUMERIC(12, 4),
    shipping_mode TEXT CHECK (shipping_mode IN ('fixed')),
    shipping_value NUMERIC(12, 4),

    -- Buyer info
    billing_name TEXT,
    billing_address TEXT CHECK (billing_address IS NULL OR BTRIM(billing_address) <> ''),
    billing_email TEXT,
    billing_phone TEXT,
    billing_tax_id TEXT,

    -- Seller info (snapshot at creation time)
    seller_name TEXT,
    seller_address TEXT CHECK (seller_address IS NULL OR BTRIM(seller_address) <> ''),
    seller_email TEXT,
    seller_phone TEXT,
    seller_tax_id TEXT,

    -- External invoice data
    external_invoice_id TEXT,
    external_order_id TEXT,
    external_status TEXT,
    external_hosted_url TEXT,
    external_pdf_url TEXT,
    external_payload JSONB,
    tax_details JSONB,

    -- Additional fields
    notes TEXT,
    payment_terms TEXT,
    void_reason TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uk_invoice_realm_number UNIQUE (realm_id, invoice_number)
);

CREATE INDEX idx_invoice_realm_status ON invoice(realm_id, status);
CREATE INDEX idx_invoice_realm_account ON invoice(realm_id, account_id);
CREATE INDEX idx_invoice_realm_created ON invoice(realm_id, created_at DESC);
CREATE UNIQUE INDEX uk_invoice_realm_external_id ON invoice(realm_id, external_invoice_id) WHERE external_invoice_id IS NOT NULL;
CREATE UNIQUE INDEX uk_invoice_realm_external_order_id ON invoice(realm_id, external_order_id) WHERE external_order_id IS NOT NULL;
CREATE INDEX idx_invoice_realm_provider ON invoice(realm_id, provider);

COMMENT ON TABLE invoice IS 'Invoice records with buyer/seller snapshots and monetary amounts';
COMMENT ON COLUMN invoice.invoice_number IS 'Formatted as INV-{YEAR}-{SEQ}';
COMMENT ON COLUMN invoice.source IS 'admin_manual = created by realm admin; user_application = applied by end user; external_sync = synced from external platform';
COMMENT ON COLUMN invoice.subtotal IS 'Sum of all line item subtotals in smallest currency unit';
COMMENT ON COLUMN invoice.total IS 'subtotal - discount_amount + tax_amount + shipping_amount';
COMMENT ON COLUMN invoice.discount_mode IS 'fixed = flat amount in currency unit; percent = percentage of subtotal';
COMMENT ON COLUMN invoice.provider IS 'Invoice source provider: manual, stripe, creem, wechat, shopify';
COMMENT ON COLUMN invoice.payment_provider IS 'Actual payment platform that collected payment';
COMMENT ON COLUMN invoice.external_invoice_id IS 'External invoice ID (e.g. Stripe invoice ID)';
COMMENT ON COLUMN invoice.external_order_id IS 'External order ID (e.g. Creem order ID)';
COMMENT ON COLUMN invoice.external_status IS 'Raw status from external platform';
COMMENT ON COLUMN invoice.external_hosted_url IS 'External hosted page URL';
COMMENT ON COLUMN invoice.external_pdf_url IS 'External PDF download URL';
COMMENT ON COLUMN invoice.external_payload IS 'Raw external invoice data snapshot (debug only)';
COMMENT ON COLUMN invoice.tax_details IS 'External tax details (e.g. from Creem MoR)';

-- ====================================
-- Invoice Line Item
-- ====================================
CREATE TABLE invoice_line_item (
    id UUID PRIMARY KEY,
    invoice_id UUID NOT NULL REFERENCES invoice(id) ON DELETE CASCADE,
    sort_order INT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    quantity NUMERIC(12, 3) NOT NULL,
    unit_price BIGINT NOT NULL,
    subtotal BIGINT NOT NULL
);

CREATE INDEX idx_invoice_line_item_invoice_sort ON invoice_line_item(invoice_id, sort_order);

COMMENT ON TABLE invoice_line_item IS 'Individual line items within an invoice';
COMMENT ON COLUMN invoice_line_item.subtotal IS 'Server-computed: round(quantity * unit_price)';
COMMENT ON COLUMN invoice_line_item.unit_price IS 'Unit price in smallest currency unit';

-- ====================================
-- Invoice History
-- ====================================
CREATE TABLE invoice_history (
    id UUID PRIMARY KEY,
    invoice_id UUID NOT NULL REFERENCES invoice(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK (event_type IN ('created', 'updated', 'issued', 'paid', 'voided', 'overdue')),
    actor_user_id UUID,
    actor_type TEXT NOT NULL CHECK (actor_type IN ('user', 'system')),
    changes JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_invoice_history_invoice_created ON invoice_history(invoice_id, created_at);

COMMENT ON TABLE invoice_history IS 'Audit trail of invoice status transitions and field changes';
COMMENT ON COLUMN invoice_history.event_type IS 'Type of event: created, updated, issued, paid, voided, overdue';
COMMENT ON COLUMN invoice_history.actor_type IS 'user = human action; system = automated job';
COMMENT ON COLUMN invoice_history.changes IS 'Change summary, e.g. {"field": "status", "from": "draft", "to": "issued"}';

-- ====================================
-- Invoice Number Counter
-- ====================================
-- Provides transaction-safe sequential invoice numbering per realm+year.
-- Uses SELECT FOR UPDATE row lock to prevent concurrent counter collisions.
CREATE TABLE invoice_number_counter (
    realm_id TEXT NOT NULL,
    year INT NOT NULL,
    next_seq BIGINT NOT NULL DEFAULT 2,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (realm_id, year)
);

COMMENT ON TABLE invoice_number_counter IS 'Counter for sequential invoice numbering within a realm+year scope';
COMMENT ON COLUMN invoice_number_counter.next_seq IS 'Next available sequence number (first invoice uses seq=1 via INSERT)';
