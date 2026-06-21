-- ====================================
-- Unified Purchase Architecture
-- ====================================
-- Migration: 20260408_unified_purchase
-- Description: Add tables for unified purchase architecture supporting both subscription and points package purchases
-- Created: 2026-04-08
-- ====================================

-- ====================================
-- Table 1: Points Packages
-- ====================================
-- This table stores points package catalog information
-- Points packages are standalone products that grant topup_credit when purchased

CREATE TABLE points_packages (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    realm_id text NOT NULL,
    name text NOT NULL,
    title text NOT NULL,
    description text,
    points bigint NOT NULL CHECK(points > 0),
    price bigint NOT NULL CHECK(price > 0),
    currency text NOT NULL,
    sort_order integer NOT NULL DEFAULT 0,
    enabled boolean NOT NULL DEFAULT true,
    package_type text NOT NULL DEFAULT 'standard',
    original_price BIGINT NULL,
    promo_start_time TIMESTAMPTZ NULL,
    promo_end_time TIMESTAMPTZ NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT uq_points_packages_realm_name UNIQUE (realm_id, name)
);

-- Index for querying enabled packages in a realm
CREATE INDEX idx_points_packages_realm_enabled ON points_packages(realm_id, enabled);

-- Comments for documentation
COMMENT ON TABLE points_packages IS 'Points package catalog for standalone credit purchases';
COMMENT ON COLUMN points_packages.id IS 'Unique package identifier (UUID v7)';
COMMENT ON COLUMN points_packages.realm_id IS 'Realm this package belongs to';
COMMENT ON COLUMN points_packages.name IS 'Unique package identifier within realm (e.g., credits-500)';
COMMENT ON COLUMN points_packages.title IS 'Display name (e.g., "500 Credits Package")';
COMMENT ON COLUMN points_packages.points IS 'Number of topup credits granted upon purchase';
COMMENT ON COLUMN points_packages.price IS 'Price in smallest currency unit (e.g., cents for USD)';
COMMENT ON COLUMN points_packages.currency IS 'ISO 4217 currency code (e.g., USD, CNY)';
COMMENT ON COLUMN points_packages.enabled IS 'Whether this package is available for purchase';

-- ====================================
-- Table 2: Points Package Payment Providers
-- ====================================
-- This table maps points packages to payment providers
-- Each package can be available on multiple payment platforms (WeChat, Stripe, Creem)

CREATE TABLE points_package_payment_providers (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    points_package_id uuid NOT NULL,
    payment_provider text NOT NULL,
    enabled boolean NOT NULL DEFAULT true,
    external_product_id text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT uq_package_provider UNIQUE (points_package_id, payment_provider),
    CONSTRAINT chk_payment_provider CHECK (payment_provider IN ('stripe', 'creem')),
    CONSTRAINT fk_package_providers_package
        FOREIGN KEY (points_package_id)
        REFERENCES points_packages(id)
        ON DELETE CASCADE
);

-- Index for querying provider mappings for a package
CREATE INDEX idx_package_providers_package ON points_package_payment_providers(points_package_id);

-- Index for querying packages by provider
CREATE INDEX idx_package_providers_provider ON points_package_payment_providers(payment_provider);

-- Foreign key is defined inline in the CREATE TABLE above

-- Comments for documentation
COMMENT ON TABLE points_package_payment_providers IS 'Payment provider availability mappings for points packages';
COMMENT ON COLUMN points_package_payment_providers.points_package_id IS 'Reference to the points package';
COMMENT ON COLUMN points_package_payment_providers.payment_provider IS 'Payment platform (stripe, creem)';
COMMENT ON COLUMN points_package_payment_providers.enabled IS 'Whether this package is purchasable via this provider';
COMMENT ON COLUMN points_package_payment_providers.external_product_id IS 'Product ID on the payment platform (e.g., WeChat product ID)';

-- ====================================
-- Table 3: Payment Attempts
-- ====================================
-- This table tracks payment attempts for initiator-based payment platforms (WeChat, Stripe, Creem)
-- Unifies payment flow for both subscription plans and points packages

CREATE TABLE payment_attempts (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    realm_id text NOT NULL,
    user_id uuid NOT NULL,
    payment_provider text NOT NULL,
    target_type text NOT NULL,
    target_id uuid NOT NULL,
    bucket_id uuid NOT NULL REFERENCES credit_buckets(id) ON DELETE RESTRICT,
    amount bigint NOT NULL CHECK(amount > 0),
    currency text NOT NULL,
    status text NOT NULL,
    provider_reference text,
    provider_status text,
    metadata jsonb,
    expires_at timestamptz NOT NULL,
    completed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT chk_payment_attempt_provider CHECK (payment_provider IN ('stripe', 'creem')),
    CONSTRAINT chk_target_type CHECK (target_type IN ('subscription_entitlement', 'points_package')),
    CONSTRAINT chk_status CHECK (status IN ('Pending', 'RequiresAction', 'Succeeded', 'Failed', 'Cancelled', 'Expired', 'completed'))
);

-- Index for querying user's payment attempts (most recent first)
CREATE INDEX idx_payment_attempts_user ON payment_attempts(user_id, created_at DESC);

-- Index for finding expired pending attempts
CREATE INDEX idx_payment_attempts_status_expires ON payment_attempts(status, expires_at);

-- Index for looking up attempts by provider reference (for webhooks)
CREATE INDEX idx_payment_attempts_provider_reference ON payment_attempts(payment_provider, provider_reference);
CREATE INDEX idx_payment_attempts_bucket_id ON payment_attempts(bucket_id);

-- Comments for documentation
COMMENT ON TABLE payment_attempts IS 'Unified payment attempt tracking for initiator-based payment platforms';
COMMENT ON COLUMN payment_attempts.target_type IS 'Type of purchasable target: subscription_entitlement or points_package';
COMMENT ON COLUMN payment_attempts.target_id IS 'ID of the subscription plan or points package being purchased';
COMMENT ON COLUMN payment_attempts.bucket_id IS 'Credit bucket snapshot for purchase fulfillment routing';
COMMENT ON COLUMN payment_attempts.provider_reference IS 'Platform-specific order reference (out_trade_no for WeChat, session ID for Stripe)';
COMMENT ON COLUMN payment_attempts.provider_status IS 'Raw status from payment platform';
COMMENT ON COLUMN payment_attempts.expires_at IS 'Payment attempt expiration time (2 hours after creation)';
COMMENT ON COLUMN payment_attempts.completed_at IS 'Time when payment was completed (succeeded or failed)';

-- ====================================
-- Table 4: Points Package Purchases
-- ====================================
-- This table records successful points package purchases
-- Separate from payment_attempts to provide user-readable purchase history

CREATE TABLE points_package_purchases (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    realm_id text NOT NULL,
    user_id uuid NOT NULL,
    points_package_id uuid NOT NULL,
    payment_attempt_id uuid NOT NULL,
    points bigint NOT NULL,
    amount bigint NOT NULL,
    currency text NOT NULL,
    payment_provider text NOT NULL,
    points_transaction_id uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT chk_purchase_provider CHECK (payment_provider IN ('stripe', 'creem'))
);

-- Index for querying user's purchase history (most recent first)
CREATE INDEX idx_points_package_purchases_user ON points_package_purchases(user_id, created_at DESC);

-- Index for querying purchases by package
CREATE INDEX idx_points_package_purchases_package ON points_package_purchases(points_package_id);

-- Unique constraint on payment_attempt_id to ensure idempotency
-- This prevents duplicate fulfillment for the same payment attempt
CREATE UNIQUE INDEX uq_points_package_purchases_payment_attempt ON points_package_purchases(payment_attempt_id);

-- Comments for documentation
COMMENT ON TABLE points_package_purchases IS 'User purchase history for points packages';
COMMENT ON COLUMN points_package_purchases.points_package_id IS 'Reference to the purchased package';
COMMENT ON COLUMN points_package_purchases.payment_attempt_id IS 'Reference to the payment attempt (unique - ensures idempotency)';
COMMENT ON COLUMN points_package_purchases.points_transaction_id IS 'Reference to the points grant transaction';
COMMENT ON COLUMN points_package_purchases.points IS 'Number of topup credits granted';
COMMENT ON COLUMN points_package_purchases.amount IS 'Amount paid (in smallest currency unit)';
