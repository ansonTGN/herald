-- ====================================
-- Shopify Payment Provider Migration
-- ====================================
-- Adds support for Shopify subscription billing integration including
-- subscription binding table, user binding table, and subscription tracking.

-- ====================================
-- Shopify Subscription Binding Table
-- ====================================
CREATE TABLE shopify_subscription_binding (
    id BIGSERIAL PRIMARY KEY,
    subscription_id UUID NOT NULL REFERENCES subscription(id) ON DELETE CASCADE,
    realm_id text NOT NULL REFERENCES realm(id) ON DELETE CASCADE,
    shop_domain text NOT NULL,
    contract_id text NOT NULL UNIQUE,
    contract_gid text NOT NULL,
    contract_revision_id BIGINT NOT NULL DEFAULT 1,
    customer_id text,
    customer_payment_method_id text,
    last_billing_attempt_id text,
    last_order_id text,
    cancel_reason TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    -- Constraints
    CONSTRAINT uq_shopify_binding_subscription UNIQUE(subscription_id)
);

-- Indexes
CREATE INDEX idx_shopify_binding_contract ON shopify_subscription_binding(contract_id);
CREATE INDEX idx_shopify_binding_realm ON shopify_subscription_binding(realm_id);
CREATE INDEX idx_shopify_binding_revision ON shopify_subscription_binding(contract_revision_id);

-- Comments
COMMENT ON TABLE shopify_subscription_binding IS 'Binding between Herald subscriptions and Shopify subscription contracts';
COMMENT ON COLUMN shopify_subscription_binding.id IS 'Primary key (BIGSERIAL)';
COMMENT ON COLUMN shopify_subscription_binding.subscription_id IS 'Reference to Herald subscription record (UUID)';
COMMENT ON COLUMN shopify_subscription_binding.realm_id IS 'Reference to realm for multi-tenancy (text type matching realm.id)';
COMMENT ON COLUMN shopify_subscription_binding.shop_domain IS 'Shopify shop domain (e.g., demo-store.myshopify.com)';
COMMENT ON COLUMN shopify_subscription_binding.contract_id IS 'Shopify subscription contract ID (unique)';
COMMENT ON COLUMN shopify_subscription_binding.contract_gid IS 'Shopify GraphQL global ID for the contract';
COMMENT ON COLUMN shopify_subscription_binding.contract_revision_id IS 'Contract revision version for idempotency (only accept higher revisions)';
COMMENT ON COLUMN shopify_subscription_binding.customer_id IS 'Shopify customer ID';
COMMENT ON COLUMN shopify_subscription_binding.customer_payment_method_id IS 'Last used payment method ID';
COMMENT ON COLUMN shopify_subscription_binding.last_billing_attempt_id IS 'Most recent billing attempt ID from Shopify';
COMMENT ON COLUMN shopify_subscription_binding.last_order_id IS 'Most recent order ID associated with this subscription';
COMMENT ON COLUMN shopify_subscription_binding.cancel_reason IS 'Reason for cancellation if applicable';
COMMENT ON COLUMN shopify_subscription_binding.created_at IS 'When this binding record was created';
COMMENT ON COLUMN shopify_subscription_binding.updated_at IS 'When this binding record was last updated';

COMMENT ON INDEX idx_shopify_binding_contract IS 'Fast lookup of binding by Shopify contract ID (used in webhook processing)';
COMMENT ON INDEX idx_shopify_binding_realm IS 'Query all Shopify subscriptions for a realm (used in config deletion checks)';
COMMENT ON INDEX idx_shopify_binding_revision IS 'Check for existing higher revision (prevents downgrade attacks)';

-- ====================================
-- Add user_id to subscription table
-- ====================================
-- user_id column is now created directly in core_init.sql

-- ====================================
-- Shopify User Binding Table
-- ====================================
-- Tracks binding between Shopify customers and Herald users
CREATE TABLE IF NOT EXISTS shopify_user_binding (
    id BIGSERIAL PRIMARY KEY,
    realm_id TEXT NOT NULL REFERENCES realm(id) ON DELETE CASCADE,
    shop_domain text NOT NULL,
    shopify_customer_id text NOT NULL,
    shopify_customer_gid text,
    user_id UUID NOT NULL REFERENCES account(id) ON DELETE CASCADE,
    status text NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_shopify_user_binding_customer
        UNIQUE (realm_id, shop_domain, shopify_customer_id)
);

CREATE INDEX IF NOT EXISTS idx_shopify_user_binding_user_id
    ON shopify_user_binding(user_id);

CREATE INDEX IF NOT EXISTS idx_shopify_user_binding_shop_domain
    ON shopify_user_binding(shop_domain);
