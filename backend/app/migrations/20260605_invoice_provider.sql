-- ====================================
-- Invoice Provider Extension Migration
-- ====================================
-- Adds provider columns, external invoice fields, partial unique constraints,
-- and relaxes NOT NULL constraints to support external invoice sync.

-- ====================================
-- New columns for provider and external invoice data
-- ====================================
ALTER TABLE invoice ADD COLUMN provider VARCHAR(20) NOT NULL DEFAULT 'manual';
ALTER TABLE invoice ADD COLUMN payment_provider VARCHAR(20);
ALTER TABLE invoice ADD COLUMN external_invoice_id TEXT;
ALTER TABLE invoice ADD COLUMN external_order_id TEXT;
ALTER TABLE invoice ADD COLUMN external_status TEXT;
ALTER TABLE invoice ADD COLUMN external_hosted_url TEXT;
ALTER TABLE invoice ADD COLUMN external_pdf_url TEXT;
ALTER TABLE invoice ADD COLUMN external_payload JSONB;
ALTER TABLE invoice ADD COLUMN tax_details JSONB;

-- ====================================
-- Partial unique constraints for external IDs
-- ====================================
CREATE UNIQUE INDEX uk_invoice_realm_external_id ON invoice(realm_id, external_invoice_id) WHERE external_invoice_id IS NOT NULL;
CREATE UNIQUE INDEX uk_invoice_realm_external_order_id ON invoice(realm_id, external_order_id) WHERE external_order_id IS NOT NULL;

-- ====================================
-- Index for provider filtering
-- ====================================
CREATE INDEX idx_invoice_realm_provider ON invoice(realm_id, provider);

-- ====================================
-- Relax CHECK constraints
-- ====================================

-- Allow negative total for external invoices (Stripe credit notes)
ALTER TABLE invoice DROP CONSTRAINT invoice_total_check;
ALTER TABLE invoice ADD CONSTRAINT invoice_total_check CHECK (total >= 0 OR provider != 'manual');

-- Extend source to include external_sync
ALTER TABLE invoice DROP CONSTRAINT invoice_source_check;
ALTER TABLE invoice ADD CONSTRAINT invoice_source_check CHECK (source IN ('admin_manual', 'user_application', 'external_sync'));

-- ====================================
-- Relax NOT NULL constraints for external invoice support
-- ====================================

-- External webhooks may not resolve a Herald account
ALTER TABLE invoice ALTER COLUMN account_id DROP NOT NULL;

-- External invoices do not provide billing data
ALTER TABLE invoice ALTER COLUMN billing_name DROP NOT NULL;
ALTER TABLE invoice ALTER COLUMN billing_address DROP NOT NULL;
ALTER TABLE invoice DROP CONSTRAINT invoice_billing_address_check;
ALTER TABLE invoice ADD CONSTRAINT invoice_billing_address_check CHECK (billing_address IS NULL OR BTRIM(billing_address) <> '');
ALTER TABLE invoice ALTER COLUMN billing_tax_id DROP NOT NULL;

-- External invoices do not provide seller data
ALTER TABLE invoice ALTER COLUMN seller_name DROP NOT NULL;
ALTER TABLE invoice ALTER COLUMN seller_address DROP NOT NULL;
ALTER TABLE invoice DROP CONSTRAINT invoice_seller_address_check;
ALTER TABLE invoice ADD CONSTRAINT invoice_seller_address_check CHECK (seller_address IS NULL OR BTRIM(seller_address) <> '');
ALTER TABLE invoice ALTER COLUMN seller_tax_id DROP NOT NULL;

-- External invoices may not have a due date
ALTER TABLE invoice ALTER COLUMN due_date DROP NOT NULL;

-- ====================================
-- Comments
-- ====================================
COMMENT ON COLUMN invoice.provider IS 'Invoice source provider: manual, stripe, creem, wechat, shopify';
COMMENT ON COLUMN invoice.payment_provider IS 'Actual payment platform that collected payment';
COMMENT ON COLUMN invoice.external_invoice_id IS 'External invoice ID (e.g. Stripe invoice ID)';
COMMENT ON COLUMN invoice.external_order_id IS 'External order ID (e.g. Creem order ID)';
COMMENT ON COLUMN invoice.external_status IS 'Raw status from external platform';
COMMENT ON COLUMN invoice.external_hosted_url IS 'External hosted page URL';
COMMENT ON COLUMN invoice.external_pdf_url IS 'External PDF download URL';
COMMENT ON COLUMN invoice.external_payload IS 'Raw external invoice data snapshot (debug only)';
COMMENT ON COLUMN invoice.tax_details IS 'External tax details (e.g. from Creem MoR)';
