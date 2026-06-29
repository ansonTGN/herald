-- Description: Add points_quota_entitlements table (window-based quota grants replacing
--              per-period ledger issuance for subscription/free-periodic credit), add
--              quota_windows config columns to provider_entitlement_mappings and
--              realm_default_configs, and add a window-aggregation covering index on
--              points_transactions.
-- Additive only: no backfill (A1 — no existing quota-entitlement data). The new table
-- coexists with points_credit_ledger (pool types keep using ledger); schedule writes
-- cease under the new model but the table is not dropped in this migration.

-- (a) points_quota_entitlements: window-based quota entitlement (replaces per-period
--     ledger issuance for subscription_credit / free_periodic_credit).
--     quota_windows is a snapshot [{windowSeconds, limit, key}] captured at grant time (A2).
--     Effective time window: effective_from..=effective_until (effective_until NULL ⟺ ongoing;
--     set on revoke/expire). Idempotency via UNIQUE(realm_id, user_id, bucket_id, credit_type,
--     idempotency_key) keyed by subscription period / webhook event.
CREATE TABLE points_quota_entitlements (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    realm_id TEXT NOT NULL,
    bucket_id UUID NOT NULL REFERENCES credit_buckets(id) ON DELETE RESTRICT,
    credit_type TEXT NOT NULL CHECK (credit_type IN ('subscription_credit', 'free_periodic_credit')),
    source_type TEXT NOT NULL CHECK (source_type IN ('subscription_initial', 'subscription_renewal', 'subscription_upgrade', 'free_periodic_grant')),
    source_id TEXT NOT NULL,
    quota_windows JSONB NOT NULL,
    effective_from TIMESTAMPTZ NOT NULL,
    effective_until TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'revoked', 'expired')),
    idempotency_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_points_quota_entitlements_idem
        UNIQUE (realm_id, user_id, bucket_id, credit_type, idempotency_key)
);

-- Consumption / balance read path: locate active entitlements for (user, bucket, credit_type).
CREATE INDEX idx_points_quota_entitlements_user_bucket_type_status
    ON points_quota_entitlements (user_id, bucket_id, credit_type, status);

-- Expiration sweep: active rows whose effective_until has passed.
CREATE INDEX idx_points_quota_entitlements_effective_until_active
    ON points_quota_entitlements (effective_until)
    WHERE status = 'active';

COMMENT ON TABLE points_quota_entitlements IS 'Window-based quota entitlements for subscription_credit / free_periodic_credit (replaces per-period ledger issuance)';
COMMENT ON COLUMN points_quota_entitlements.quota_windows IS 'Snapshot of [{windowSeconds, limit, key}] captured at grant time (A2)';
COMMENT ON COLUMN points_quota_entitlements.source_id IS 'subscription_id or registration/free source identifier';
COMMENT ON COLUMN points_quota_entitlements.idempotency_key IS 'Business idempotency key (subscription period / webhook event)';

-- (b) provider_entitlement_mappings: quota_windows config (subscription quota definition).
--     Non-NULL ⟺ this mapping grants a window entitlement. Existing
--     points_per_period / grant_period_type / validity_days remain nullable (pool / legacy
--     read compatibility); quota_windows non-NULL switches grant to the window model.
ALTER TABLE provider_entitlement_mappings ADD COLUMN quota_windows JSONB;

COMMENT ON COLUMN provider_entitlement_mappings.quota_windows IS 'Subscription quota window definition [{windowSeconds, limit, key}]; non-NULL ⟺ window-model grant';

-- (c) realm_default_configs: free_periodic_quota_windows config (free periodic quota definition).
--     Non-NULL ⟺ free periodic grant uses window entitlement (replaces per-period issuance).
ALTER TABLE realm_default_configs ADD COLUMN free_periodic_quota_windows JSONB;

COMMENT ON COLUMN realm_default_configs.free_periodic_quota_windows IS 'Free periodic quota window definition [{windowSeconds, limit, key}]; non-NULL ⟺ window-model grant';

-- (d) points_transactions: window-aggregation covering index (covers P1).
--     Existing indexes (e.g. (user_id, created_at DESC)) do not filter by credit_type, so
--     window SUM(amount) WHERE ... type='consume' AND created_at >= now-? could not use an
--     index range scan by credit_type. This partial covering index makes the window
--     aggregation an index-only scan.
CREATE INDEX idx_points_transactions_window_agg
    ON points_transactions (user_id, bucket_id, credit_type, created_at DESC)
    INCLUDE (amount)
    WHERE type = 'consume';
