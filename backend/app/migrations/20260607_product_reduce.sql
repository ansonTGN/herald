-- ====================================
-- Product Reduce Migration
-- ====================================
-- Creates provider_entitlement_mappings for provider-sourced subscription catalog.

-- 1. CREATE provider_entitlement_mappings
CREATE TABLE provider_entitlement_mappings (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    realm_id TEXT NOT NULL,
    payment_provider TEXT NOT NULL,
    external_product_id TEXT NOT NULL,
    external_price_id TEXT,
    bucket_id UUID REFERENCES credit_buckets(id) ON DELETE RESTRICT,
    entitlement_key TEXT NOT NULL,
    billing_type TEXT,
    billing_period TEXT,
    points_per_period INTEGER,
    grant_period_type TEXT,
    validity_days INTEGER,
    grant_on_subscribe BOOLEAN NOT NULL DEFAULT false,
    max_periods INTEGER,
    enabled BOOLEAN NOT NULL DEFAULT false,
    provider_product_info JSONB,
    synced_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_pem_realm_provider_product UNIQUE (realm_id, payment_provider, external_product_id),
    CONSTRAINT chk_pem_entitlement_key CHECK (entitlement_key ~ '^[a-z0-9-]{1,64}$'),
    CONSTRAINT chk_pem_billing_type CHECK (billing_type IS NULL OR billing_type IN ('recurring', 'one_time')),
    CONSTRAINT chk_pem_payment_provider CHECK (payment_provider IN ('stripe', 'creem', 'wechat', 'shopify'))
);

CREATE INDEX idx_pem_realm_id ON provider_entitlement_mappings(realm_id);
CREATE INDEX idx_pem_realm_provider ON provider_entitlement_mappings(realm_id, payment_provider);
CREATE INDEX idx_pem_bucket_id ON provider_entitlement_mappings(bucket_id);
CREATE INDEX idx_pem_entitlement_key ON provider_entitlement_mappings(entitlement_key);

COMMENT ON TABLE provider_entitlement_mappings IS 'Maps payment provider products to Herald entitlement keys with points strategy config';
COMMENT ON COLUMN provider_entitlement_mappings.entitlement_key IS 'Herald entitlement identifier, matching [a-z0-9-]{1,64}';
COMMENT ON COLUMN provider_entitlement_mappings.bucket_id IS 'Credit bucket where purchases of this mapping are fulfilled';
COMMENT ON COLUMN provider_entitlement_mappings.billing_type IS 'recurring or one_time';
COMMENT ON COLUMN provider_entitlement_mappings.payment_provider IS 'Payment provider: stripe, creem, wechat, shopify';
COMMENT ON COLUMN provider_entitlement_mappings.provider_product_info IS 'Cached provider product info (name, price, currency, etc.)';
