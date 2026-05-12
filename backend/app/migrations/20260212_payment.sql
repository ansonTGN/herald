-- ====================================
-- Payment Module Migration
-- ====================================
-- Consolidates payment provider configuration and multi-provider event schema.

INSERT INTO realm_config (
    realm_id,
    config_type,
    config_key,
    config_value,
    is_secret,
    enabled,
    metadata
) VALUES
    ('admin', 'creem', 'api_key', 'creem_your_api_key_here', true, true, null),
    ('admin', 'creem', 'webhook_secret', 'creem_your_webhook_secret_here', true, true, null),
    ('admin', 'creem', 'timeout', '30', false, true, null),
    ('admin', 'stripe', 'api_key', 'sk_test_your_api_key_here', true, true, null),
    ('admin', 'stripe', 'webhook_secret', 'whsec_your_webhook_secret_here', true, true, null),
    ('admin', 'stripe', 'publishable_key', 'pk_test_your_publishable_key_here', false, true, null),
    ('admin', 'stripe', 'timeout', '30', false, true, null)
ON CONFLICT (realm_id, config_type, config_key)
DO UPDATE SET
    config_value = EXCLUDED.config_value,
    is_secret = EXCLUDED.is_secret,
    enabled = EXCLUDED.enabled,
    updated_at = CURRENT_TIMESTAMP;

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
