-- ====================================
-- Payment Module Migration
-- ====================================
-- Consolidates payment provider configuration and multi-provider event schema.

DROP INDEX IF EXISTS idx_payment_event_creem_event_id;
ALTER TABLE payment_event DROP CONSTRAINT IF EXISTS payment_event_creem_event_id_key;

ALTER TABLE payment_event
    ADD COLUMN IF NOT EXISTS payment_provider VARCHAR(50) NOT NULL DEFAULT 'creem';

ALTER TABLE payment_event
    RENAME COLUMN creem_event_id TO external_event_id;

ALTER TABLE payment_event
    ALTER COLUMN external_event_id SET NOT NULL;

ALTER TABLE payment_event
    ADD CONSTRAINT payment_event_unique_external_provider
    UNIQUE (external_event_id, payment_provider);

CREATE INDEX idx_payment_event_provider
    ON payment_event(payment_provider);

COMMENT ON COLUMN payment_event.external_event_id IS 'External event ID from payment provider (unique per provider)';
COMMENT ON COLUMN payment_event.payment_provider IS 'Payment provider type (creem, stripe, etc.)';
COMMENT ON TABLE payment_event IS 'Payment events from multiple providers (Creem, Stripe, etc.)';
COMMENT ON TABLE realm_config IS 'Realm-specific configuration including TOTP, Turnstile, Registration, TotpKey, Creem, and Stripe payment provider settings';
