-- ====================================
-- Billing Module Migration
-- ====================================
-- Consolidates subscription history, points, ledger, free user points,
-- and idempotency schema into a single module migration.

-- ====================================
-- Subscription History
-- ====================================
CREATE TABLE subscription_history (
    id TEXT PRIMARY KEY,
    subscription_id UUID NOT NULL,
    event_type text NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    actor TEXT,
    changes JSONB,
    previous_state JSONB,
    new_state JSONB,
    realm_id TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT fk_subscription_history_subscription
        FOREIGN KEY (subscription_id) REFERENCES subscription(id) ON DELETE CASCADE
);

CREATE INDEX idx_subscription_history_subscription_id
    ON subscription_history(subscription_id);
CREATE INDEX idx_subscription_history_realm_id
    ON subscription_history(realm_id);
CREATE INDEX idx_subscription_history_timestamp
    ON subscription_history(timestamp);
CREATE INDEX idx_subscription_history_event_type
    ON subscription_history(event_type);
CREATE INDEX idx_subscription_history_realm_timestamp
    ON subscription_history(realm_id, timestamp DESC);

COMMENT ON TABLE subscription_history IS 'Audit trail of subscription changes including upgrades, downgrades, cancellations, and other events';
COMMENT ON COLUMN subscription_history.id IS 'Unique event identifier (UUID v7)';
COMMENT ON COLUMN subscription_history.subscription_id IS 'Reference to the subscription that changed';
COMMENT ON COLUMN subscription_history.event_type IS 'Type of event: created, upgraded, downgraded, canceled, expired, renewed, reactivated, billing_period_changed';
COMMENT ON COLUMN subscription_history.timestamp IS 'When the change occurred';
COMMENT ON COLUMN subscription_history.actor IS 'Who performed the change (user ID, system, or webhook)';
COMMENT ON COLUMN subscription_history.changes IS 'Detailed change information (JSON)';
COMMENT ON COLUMN subscription_history.previous_state IS 'Subscription state before the change (JSON)';
COMMENT ON COLUMN subscription_history.new_state IS 'Subscription state after the change (JSON)';
COMMENT ON COLUMN subscription_history.realm_id IS 'Realm ID for permission isolation';
COMMENT ON COLUMN subscription_history.created_at IS 'When this history record was created';

-- ====================================
-- Points Accounts
-- ====================================
CREATE TABLE points_wallets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES account(id) ON DELETE CASCADE,
    realm_id TEXT NOT NULL,
    topup_balance BIGINT NOT NULL DEFAULT 0 CHECK (topup_balance >= 0),
    subscription_balance BIGINT NOT NULL DEFAULT 0 CHECK (subscription_balance >= 0),
    total_balance BIGINT GENERATED ALWAYS AS (
        topup_balance + subscription_balance
    ) STORED CHECK (total_balance >= 0),
    total_recharged BIGINT NOT NULL DEFAULT 0 CHECK (total_recharged >= 0),
    total_consumed BIGINT NOT NULL DEFAULT 0 CHECK (total_consumed >= 0),
    total_topup_granted BIGINT NOT NULL DEFAULT 0 CHECK (total_topup_granted >= 0),
    total_subscription_granted BIGINT NOT NULL DEFAULT 0 CHECK (total_subscription_granted >= 0),
    status text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'frozen', 'closed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uk_points_wallets_user_id UNIQUE (user_id)
);

CREATE INDEX idx_points_wallets_user_id ON points_wallets(user_id);
CREATE INDEX idx_points_wallets_realm_id ON points_wallets(realm_id);
CREATE INDEX idx_points_wallets_status ON points_wallets(status);

COMMENT ON TABLE points_wallets IS 'User-level points wallets tracking balance, recharges, and consumption';
COMMENT ON COLUMN points_wallets.id IS 'Unique wallet identifier';
COMMENT ON COLUMN points_wallets.user_id IS 'Reference to user who owns this wallet';
COMMENT ON COLUMN points_wallets.realm_id IS 'Realm ID for permission isolation';
COMMENT ON COLUMN points_wallets.topup_balance IS 'Current balance of topup credits (purchased points)';
COMMENT ON COLUMN points_wallets.subscription_balance IS 'Current balance of subscription credits (from subscriptions)';
COMMENT ON COLUMN points_wallets.total_balance IS 'Computed total balance: topup_balance + subscription_balance';
COMMENT ON COLUMN points_wallets.total_recharged IS 'Total points ever recharged (for analytics)';
COMMENT ON COLUMN points_wallets.total_consumed IS 'Total points ever consumed (for analytics)';
COMMENT ON COLUMN points_wallets.total_topup_granted IS 'Total purchased points ever granted';
COMMENT ON COLUMN points_wallets.total_subscription_granted IS 'Total subscription points ever granted';
COMMENT ON COLUMN points_wallets.status IS 'Wallet status: active (normal operations), frozen (temporarily disabled), closed (permanently disabled)';

-- ====================================
-- Points Transactions
-- ====================================
CREATE TABLE points_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id UUID NOT NULL REFERENCES points_wallets(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES account(id) ON DELETE CASCADE,
    realm_id TEXT NOT NULL,
    type text NOT NULL CHECK (type IN (
        'recharge',
        'consume',
        'subscription_grant',
        'subscription_renewal',
        'subscription_upgrade',
        'registration_grant',
        'free_periodic_grant',
        'refund_revoke',
        'expire_revoke',
        'cancel_revoke',
        'idempotency_record',
        'expiration',
        'refund'
    )),
    amount BIGINT NOT NULL,
    balance_after BIGINT NOT NULL CHECK (balance_after >= 0),
    topup_balance_after BIGINT CHECK (topup_balance_after >= 0),
    subscription_balance_after BIGINT CHECK (subscription_balance_after >= 0),
    credit_type text CHECK (credit_type IN (
        'topup_credit',
        'subscription_credit',
        'registration_credit',
        'free_periodic_credit'
    )),
    description TEXT,
    client_app_id UUID REFERENCES client_app(id) ON DELETE SET NULL,
    subscription_id UUID REFERENCES subscription(id) ON DELETE SET NULL,
    external_ref_id TEXT,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_points_transactions_wallet_id ON points_transactions(wallet_id);
CREATE INDEX idx_points_transactions_user_id ON points_transactions(user_id);
CREATE INDEX idx_points_transactions_realm_id ON points_transactions(realm_id);
CREATE INDEX idx_points_transactions_type ON points_transactions(type);
CREATE INDEX idx_points_transactions_created_at ON points_transactions(created_at DESC);
CREATE INDEX idx_points_transactions_client_app_id ON points_transactions(client_app_id);
CREATE INDEX idx_points_transactions_subscription_id ON points_transactions(subscription_id);
CREATE INDEX idx_points_transactions_realm_created ON points_transactions(realm_id, created_at DESC);
CREATE INDEX idx_points_transactions_user_created ON points_transactions(user_id, created_at DESC);
CREATE INDEX idx_points_transactions_expires_at
    ON points_transactions(expires_at)
    WHERE expires_at IS NOT NULL;
CREATE UNIQUE INDEX idx_transactions_external_ref
    ON points_transactions(user_id, external_ref_id)
    WHERE external_ref_id IS NOT NULL;

COMMENT ON TABLE points_transactions IS 'Transaction history for all points movements';
COMMENT ON COLUMN points_transactions.credit_type IS 'Type of credit affected: topup_credit, subscription_credit, registration_credit, or free_periodic_credit';
COMMENT ON COLUMN points_transactions.type IS 'Transaction type for recharge, consumption, grant, revocation, expiration, refund, and idempotency records';
COMMENT ON COLUMN points_transactions.topup_balance_after IS 'Topup credit balance after this transaction';
COMMENT ON COLUMN points_transactions.subscription_balance_after IS 'Subscription credit balance after this transaction';
COMMENT ON COLUMN points_transactions.expires_at IS 'Expiration time for time-limited points (NULL = permanent points)';
COMMENT ON COLUMN points_transactions.updated_at IS 'When transaction was last updated';
COMMENT ON INDEX idx_transactions_external_ref IS 'Idempotency constraint for webhook event processing based on user_id + external_ref_id';

-- ====================================
-- Points Plan Configs
-- ====================================
CREATE TABLE points_plan_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    realm_id TEXT NOT NULL,
    plan_id UUID NOT NULL REFERENCES subscription_plan(id) ON DELETE CASCADE,
    grant_period_type text NOT NULL DEFAULT 'once' CHECK (grant_period_type IN ('once', 'daily', 'weekly', 'monthly')),
    points_per_period BIGINT NOT NULL DEFAULT 0 CHECK (points_per_period >= 0),
    validity_days BIGINT NOT NULL DEFAULT 0 CHECK (validity_days >= 0),
    grant_on_subscribe BOOLEAN NOT NULL DEFAULT TRUE,
    max_periods BIGINT CHECK (max_periods IS NULL OR max_periods > 0),
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uk_points_plan_configs_plan_id UNIQUE (plan_id)
);

CREATE INDEX idx_points_plan_configs_realm_id ON points_plan_configs(realm_id);
CREATE INDEX idx_points_plan_configs_plan_id ON points_plan_configs(plan_id);

COMMENT ON TABLE points_plan_configs IS 'Configuration mapping billing plans to flexible points grant rules';

-- ====================================
-- Idempotency Keys
-- ====================================
CREATE TABLE idempotency_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    realm_id TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'processing', 'completed', 'failed')),
    request_data JSONB NOT NULL DEFAULT '{}',
    response_data JSONB,
    transaction_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX uk_idempotency_keys ON idempotency_keys(realm_id, idempotency_key);
CREATE INDEX idx_idempotency_expires ON idempotency_keys(expires_at);
CREATE INDEX idx_idempotency_transaction ON idempotency_keys(transaction_id);

COMMENT ON TABLE idempotency_keys IS 'Idempotency keys for points consumption to prevent duplicate charges';

-- ====================================
-- Points Credit Ledger
-- ====================================
CREATE TABLE points_credit_ledger (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    realm_id TEXT NOT NULL,
    credit_type text NOT NULL CHECK (credit_type IN (
        'topup_credit',
        'subscription_credit',
        'registration_credit',
        'free_periodic_credit'
    )),
    source_type text NOT NULL CHECK (source_type IN (
        'subscription_initial',
        'subscription_renewal',
        'subscription_upgrade',
        'topup',
        'system_grant',
        'registration',
        'free_periodic_grant'
    )),
    source_id TEXT NOT NULL,
    granted_amount BIGINT NOT NULL CHECK (granted_amount > 0),
    used_amount BIGINT NOT NULL DEFAULT 0 CHECK (used_amount >= 0),
    revoked_amount BIGINT NOT NULL DEFAULT 0 CHECK (revoked_amount >= 0),
    remaining_amount BIGINT NOT NULL GENERATED ALWAYS AS (
        granted_amount - used_amount - revoked_amount
    ) STORED CHECK (remaining_amount >= 0),
    expires_at TIMESTAMPTZ,
    status text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'revoked', 'expired', 'fully_used')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_points_credit_ledger_user_id ON points_credit_ledger(user_id);
CREATE INDEX idx_points_credit_ledger_realm_id ON points_credit_ledger(realm_id);
CREATE INDEX idx_points_credit_ledger_credit_type ON points_credit_ledger(credit_type);
CREATE INDEX idx_points_credit_ledger_status ON points_credit_ledger(status);
CREATE INDEX idx_points_credit_ledger_expires_at ON points_credit_ledger(expires_at);
CREATE INDEX idx_points_credit_ledger_user_credit_status
    ON points_credit_ledger(user_id, credit_type, status);
CREATE INDEX idx_points_credit_ledger_created_at ON points_credit_ledger(created_at ASC);

COMMENT ON TABLE points_credit_ledger IS 'Source of truth for all points credits, tracking grants, usage, and revocation by type';
COMMENT ON COLUMN points_credit_ledger.credit_type IS 'Type of credit: topup_credit, subscription_credit, registration_credit, or free_periodic_credit';
COMMENT ON COLUMN points_credit_ledger.source_type IS 'Source of credit: subscription_initial/renewal/upgrade, topup, system_grant, registration, or free_periodic_grant';
COMMENT ON COLUMN points_credit_ledger.remaining_amount IS 'Computed field: granted_amount - used_amount - revoked_amount';

-- ====================================
-- Points Consumption Allocations
-- ====================================
CREATE TABLE points_consumption_allocations (
    id UUID PRIMARY KEY,
    transaction_id UUID NOT NULL,
    ledger_id UUID NOT NULL,
    user_id UUID NOT NULL,
    realm_id TEXT NOT NULL,
    allocated_amount BIGINT NOT NULL CHECK (allocated_amount > 0),
    ledger_remaining_after BIGINT NOT NULL CHECK (ledger_remaining_after >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_points_consumption_allocations_transaction_id
    ON points_consumption_allocations(transaction_id);
CREATE INDEX idx_points_consumption_allocations_ledger_id
    ON points_consumption_allocations(ledger_id);
CREATE INDEX idx_points_consumption_allocations_user_id
    ON points_consumption_allocations(user_id);
CREATE INDEX idx_points_consumption_allocations_realm_id
    ON points_consumption_allocations(realm_id);

COMMENT ON TABLE points_consumption_allocations IS 'Allocation records showing how each consumption transaction splits across ledger entries';

-- ====================================
-- Points Revocation Records
-- ====================================
CREATE TABLE points_revocation_records (
    id UUID PRIMARY KEY,
    ledger_id UUID NOT NULL,
    user_id UUID NOT NULL,
    realm_id TEXT NOT NULL,
    revocation_type text NOT NULL CHECK (revocation_type IN (
        'refund_revoke',
        'expire_revoke',
        'cancel_revoke',
        'upgrade_revoke'
    )),
    revoked_amount BIGINT NOT NULL CHECK (revoked_amount > 0),
    reason TEXT NOT NULL,
    reference_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_points_revocation_records_ledger_id
    ON points_revocation_records(ledger_id);
CREATE INDEX idx_points_revocation_records_user_id
    ON points_revocation_records(user_id);
CREATE INDEX idx_points_revocation_records_revocation_type
    ON points_revocation_records(revocation_type);
CREATE INDEX idx_points_revocation_records_realm_id
    ON points_revocation_records(realm_id);

COMMENT ON TABLE points_revocation_records IS 'Records of all points revocation operations';
COMMENT ON COLUMN points_revocation_records.revocation_type IS 'Type of revocation: refund_revoke, expire_revoke, cancel_revoke, or upgrade_revoke';

-- ====================================
-- Free User Points Config
-- ====================================
CREATE TABLE realm_default_configs (
    realm_id TEXT PRIMARY KEY,
    registration_bonus_points BIGINT NOT NULL DEFAULT 0 CHECK (registration_bonus_points >= 0),
    free_periodic_grant_period_type text NOT NULL DEFAULT 'daily' CHECK (free_periodic_grant_period_type IN ('once', 'daily', 'weekly', 'monthly')),
    free_periodic_points_amount BIGINT NOT NULL DEFAULT 0 CHECK (free_periodic_points_amount >= 0),
    free_periodic_validity_days BIGINT NOT NULL DEFAULT 0 CHECK (free_periodic_validity_days >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO realm_default_configs (realm_id)
VALUES ('default')
ON CONFLICT (realm_id) DO NOTHING;

COMMENT ON TABLE realm_default_configs IS 'Default configuration for free user points strategy at realm level';
COMMENT ON COLUMN realm_default_configs.free_periodic_grant_period_type IS 'Grant period type: once (one-time), daily, weekly, or monthly';
COMMENT ON COLUMN realm_default_configs.free_periodic_points_amount IS 'Amount of free points granted per period (0 = disabled)';
COMMENT ON COLUMN realm_default_configs.free_periodic_validity_days IS 'Validity period in days (0 = permanent, must be >= 1 for non-once periods)';

CREATE TABLE user_points_configs (
    user_id UUID PRIMARY KEY,
    realm_id TEXT NOT NULL,
    registration_bonus_points BIGINT NOT NULL CHECK (registration_bonus_points >= 0),
    free_periodic_grant_period_type text CHECK (free_periodic_grant_period_type IN ('once', 'daily', 'weekly', 'monthly')),
    free_periodic_points_amount BIGINT CHECK (free_periodic_points_amount >= 0),
    free_periodic_validity_days BIGINT CHECK (free_periodic_validity_days >= 0),
    next_grant_time TIMESTAMPTZ,
    granted_periods BIGINT NOT NULL DEFAULT 0 CHECK (granted_periods >= 0),
    grant_schedule_id UUID UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_points_configs_realm_id ON user_points_configs(realm_id);
CREATE INDEX idx_user_points_configs_next_grant_time
    ON user_points_configs(next_grant_time);

COMMENT ON TABLE user_points_configs IS 'Individual user configuration for free user points strategy';
COMMENT ON COLUMN user_points_configs.free_periodic_grant_period_type IS 'Grant period type for this user (NULL = not configured)';
COMMENT ON COLUMN user_points_configs.free_periodic_points_amount IS 'Amount of free points granted per period for this user';
COMMENT ON COLUMN user_points_configs.free_periodic_validity_days IS 'Validity period in days (0 = permanent, must be >= 1 for non-once periods)';

CREATE TABLE points_grant_schedules (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    realm_id TEXT NOT NULL,
    subscription_id UUID,
    plan_config_id UUID,
    grant_period_type text NOT NULL CHECK (grant_period_type IN ('once', 'daily', 'weekly', 'monthly')),
    base_time TIMESTAMPTZ NOT NULL,
    next_grant_time TIMESTAMPTZ NOT NULL,
    points_per_period BIGINT NOT NULL CHECK (points_per_period >= 0),
    validity_days BIGINT NOT NULL CHECK (validity_days >= 0),
    granted_periods BIGINT NOT NULL DEFAULT 0 CHECK (granted_periods >= 0),
    max_periods BIGINT CHECK (max_periods > 0),
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_points_grant_schedules_next_grant_time
    ON points_grant_schedules(next_grant_time)
    WHERE active = TRUE;
CREATE INDEX idx_points_grant_schedules_user_id
    ON points_grant_schedules(user_id);
CREATE INDEX idx_points_grant_schedules_subscription_id
    ON points_grant_schedules(subscription_id);

COMMENT ON TABLE points_grant_schedules IS 'Automatic points granting schedules for free users and subscriptions';

CREATE TABLE points_grant_records (
    id UUID PRIMARY KEY,
    schedule_id UUID NOT NULL,
    user_id UUID NOT NULL,
    realm_id TEXT NOT NULL,
    period_number BIGINT NOT NULL CHECK (period_number > 0),
    granted_amount BIGINT NOT NULL CHECK (granted_amount > 0),
    grant_time TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX uk_points_grant_records_schedule_period
    ON points_grant_records(schedule_id, period_number);
CREATE INDEX idx_points_grant_records_user_id
    ON points_grant_records(user_id);
CREATE INDEX idx_points_grant_records_grant_time
    ON points_grant_records(grant_time);

COMMENT ON TABLE points_grant_records IS 'History of points grants for each schedule';
