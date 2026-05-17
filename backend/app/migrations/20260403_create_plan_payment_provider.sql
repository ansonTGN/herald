-- ====================================
-- Subscription Plan Payment Provider Migration
-- ====================================
-- This migration separates payment provider configuration from Plan entity,
-- allowing a Plan to support multiple payment providers.

-- ====================================
-- Step 1: Create subscription_plan_payment_provider table
-- ====================================
CREATE TABLE subscription_plan_payment_provider (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    plan_id UUID NOT NULL,
    payment_provider VARCHAR(50) NOT NULL,
    external_product_id TEXT NOT NULL,
    external_price_id TEXT,
    enabled BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_subscription_plan_payment_provider_plan
        FOREIGN KEY (plan_id) REFERENCES subscription_plan(id) ON DELETE CASCADE,
    CONSTRAINT uq_subscription_plan_payment_provider_plan_provider UNIQUE (plan_id, payment_provider)
);

-- Create indexes for subscription_plan_payment_provider
CREATE INDEX idx_subscription_plan_payment_provider_plan_id ON subscription_plan_payment_provider(plan_id);
CREATE INDEX idx_subscription_plan_payment_provider_provider ON subscription_plan_payment_provider(payment_provider);

-- Add comments for documentation
COMMENT ON TABLE subscription_plan_payment_provider IS 'Payment provider mappings for plans, allowing multiple providers per plan';
COMMENT ON COLUMN subscription_plan_payment_provider.plan_id IS 'Reference to the plan';
COMMENT ON COLUMN subscription_plan_payment_provider.payment_provider IS 'Payment provider name (stripe, creem, shopify, etc.)';
COMMENT ON COLUMN subscription_plan_payment_provider.external_product_id IS 'External product ID from the payment provider';
COMMENT ON COLUMN subscription_plan_payment_provider.external_price_id IS 'External price ID from the payment provider (optional)';
COMMENT ON COLUMN subscription_plan_payment_provider.enabled IS 'Whether this payment provider mapping is enabled for checkout';

-- ====================================
-- Step 2: Remove old payment provider fields from subscription_plan table
-- ====================================
-- Drop the index on payment_provider (if exists)
DROP INDEX IF EXISTS idx_subscription_plan_payment_provider;

-- Remove the check constraint on payment_provider (if exists)
ALTER TABLE subscription_plan DROP CONSTRAINT IF EXISTS chk_subscription_plan_payment_provider;

-- Remove the old payment provider columns
ALTER TABLE subscription_plan DROP COLUMN IF EXISTS payment_provider;
ALTER TABLE subscription_plan DROP COLUMN IF EXISTS external_product_id;
ALTER TABLE subscription_plan DROP COLUMN IF EXISTS external_price_id;
