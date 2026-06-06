-- ====================================
-- Product Reduce Migration
-- ====================================
-- Drops legacy product/plan catalog tables, creates provider_entitlement_mappings,
-- and alters subscription/points_grant_schedules to use entitlement_key.

-- 1. DROP tables in dependency order (children first)
DROP TABLE IF EXISTS points_plan_configs;
DROP TABLE IF EXISTS client_app_subscription_plan;
DROP TABLE IF EXISTS subscription_plan_payment_provider;
DROP TABLE IF EXISTS subscription_plan;
DROP TABLE IF EXISTS products;

-- 2. CREATE provider_entitlement_mappings
CREATE TABLE provider_entitlement_mappings (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    realm_id TEXT NOT NULL,
    payment_provider TEXT NOT NULL,
    external_product_id TEXT NOT NULL,
    external_price_id TEXT,
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
CREATE INDEX idx_pem_entitlement_key ON provider_entitlement_mappings(entitlement_key);

COMMENT ON TABLE provider_entitlement_mappings IS 'Maps payment provider products to Herald entitlement keys with points strategy config';
COMMENT ON COLUMN provider_entitlement_mappings.entitlement_key IS 'Herald entitlement identifier, matching [a-z0-9-]{1,64}';
COMMENT ON COLUMN provider_entitlement_mappings.billing_type IS 'recurring or one_time';
COMMENT ON COLUMN provider_entitlement_mappings.payment_provider IS 'Payment provider: stripe, creem, wechat, shopify';
COMMENT ON COLUMN provider_entitlement_mappings.provider_product_info IS 'Cached provider product info (name, price, currency, etc.)';

-- 3. ALTER subscription: add new columns, drop old columns and indexes
ALTER TABLE subscription
    ADD COLUMN entitlement_key TEXT NOT NULL DEFAULT '',
    ADD COLUMN external_price_id TEXT,
    ADD COLUMN provider_metadata JSONB,
    ADD COLUMN synced_at TIMESTAMPTZ;

-- Drop old indexes if they exist
DROP INDEX IF EXISTS idx_subscription_plan_id;
DROP INDEX IF EXISTS idx_subscription_billing_period;

-- Drop old columns
ALTER TABLE subscription
    DROP COLUMN IF EXISTS plan_id,
    DROP COLUMN IF EXISTS tier,
    DROP COLUMN IF EXISTS billing_period;

-- Add new index
CREATE INDEX idx_subscription_entitlement_key ON subscription(entitlement_key);

-- 4. ALTER points_grant_schedules: drop plan_config_id, add entitlement_key
ALTER TABLE points_grant_schedules
    DROP COLUMN IF EXISTS plan_config_id,
    ADD COLUMN entitlement_key TEXT NOT NULL DEFAULT '';
