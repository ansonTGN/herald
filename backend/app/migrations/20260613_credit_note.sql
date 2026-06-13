-- ====================================
-- Credit Note Migration
-- ====================================
-- Changes: invoice.amount_refunded, invoice.amount_remaining, credit_note (+status),
--          invoice_history.event_type extended for credit_note_created / credit_note_voided

-- ====================================
-- Invoice: Refund Columns
-- ====================================
-- Cached derived values maintained in the same transaction as credit_note rows.
-- amount_refunded = SUM(credit_note.amount WHERE status='active') for the invoice.
-- amount_remaining = total - amount_refunded.
ALTER TABLE invoice ADD COLUMN amount_refunded BIGINT NOT NULL DEFAULT 0;
ALTER TABLE invoice ADD COLUMN amount_remaining BIGINT NOT NULL DEFAULT 0;

-- Backfill existing invoices: no refunds recorded, so remaining equals total.
UPDATE invoice SET amount_remaining = total;

COMMENT ON COLUMN invoice.amount_refunded IS 'Accumulated refund amount in smallest currency unit (cached from credit_note)';
COMMENT ON COLUMN invoice.amount_remaining IS 'Remaining payable amount in smallest currency unit (= total - amount_refunded)';

ALTER TABLE invoice ADD CONSTRAINT invoice_amount_refunded_check CHECK (amount_refunded >= 0);
ALTER TABLE invoice ADD CONSTRAINT invoice_amount_remaining_check CHECK (amount_remaining >= 0);

-- ====================================
-- Credit Note
-- ====================================
-- Single table for both Stripe (passive sync) and Manual (admin created) credit notes.
-- source distinguishes the origin; source-specific fields are nullable.
-- status tracks active vs voided (voided refunds are reversed on the parent invoice).
CREATE TABLE credit_note (
    id UUID PRIMARY KEY,
    invoice_id UUID NOT NULL REFERENCES invoice(id) ON DELETE CASCADE,
    realm_id TEXT NOT NULL REFERENCES realm(id) ON DELETE CASCADE,
    amount BIGINT NOT NULL CHECK (amount > 0),
    currency TEXT NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('stripe', 'manual')),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'voided')),
    external_credit_note_id TEXT,
    memo TEXT,
    created_by_user_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_credit_note_invoice_id ON credit_note(invoice_id);
CREATE INDEX idx_credit_note_realm_id ON credit_note(realm_id);
CREATE UNIQUE INDEX uk_credit_note_external_id ON credit_note(external_credit_note_id) WHERE external_credit_note_id IS NOT NULL;

COMMENT ON TABLE credit_note IS 'Credit notes recording refunds against an invoice (Stripe sync or manual entry)';
COMMENT ON COLUMN credit_note.invoice_id IS 'Invoice this credit note refunds';
COMMENT ON COLUMN credit_note.realm_id IS 'Realm isolation key';
COMMENT ON COLUMN credit_note.amount IS 'Refund amount in smallest currency unit (must be positive)';
COMMENT ON COLUMN credit_note.currency IS 'Currency code, matches the invoice currency';
COMMENT ON COLUMN credit_note.source IS 'stripe = synced from Stripe credit_note.created webhook; manual = recorded by realm admin';
COMMENT ON COLUMN credit_note.status IS 'Lifecycle: active = applies to invoice; voided = reversed (credit_note.voided webhook or admin void)';
COMMENT ON COLUMN credit_note.external_credit_note_id IS 'Stripe Credit Note ID (idempotency key, only for source=stripe)';
COMMENT ON COLUMN credit_note.memo IS 'Manual refund reason (only for source=manual)';
COMMENT ON COLUMN credit_note.created_by_user_id IS 'Operator who created the manual credit note (only for source=manual)';

-- ====================================
-- invoice_history.event_type: extend for credit note events
-- ====================================
ALTER TABLE invoice_history DROP CONSTRAINT invoice_history_event_type_check;
ALTER TABLE invoice_history ADD CONSTRAINT invoice_history_event_type_check
    CHECK (event_type IN ('created', 'updated', 'issued', 'paid', 'voided', 'overdue', 'credit_note_created', 'credit_note_voided'));
