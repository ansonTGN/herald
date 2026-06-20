-- Stage 4 (revised): Maintain per-credit-type balance columns on
-- points_wallets as a fast, bucket-scoped, drift-free projection of
-- points_credit_ledger (the authoritative append-only log).
--
-- The ledger stays the source of truth; these columns are a maintained
-- projection written by a SINGLE writer (`apply_wallet_delta_in_tx`) inside
-- each ledger-mutation transaction. Reads (get_balance / get_wallet /
-- consume precheck / refund guard) are O(1).
--
-- This migration is idempotent: if the prior (uncommitted) draft already
-- dropped the columns on a dev DB, it recreates them and backfills from the
-- ledger. If the columns are still present, it is a no-op + backfill.
--
-- 1. points_wallets: ensure the per-type balance columns + aggregate totals
--    exist (BIGINT NOT NULL DEFAULT 0, CHECK >= 0). All ADD COLUMN IF NOT
--    EXISTS so re-running is safe regardless of prior state.

ALTER TABLE points_wallets ADD COLUMN IF NOT EXISTS topup_balance
    BIGINT NOT NULL DEFAULT 0 CHECK (topup_balance >= 0);
ALTER TABLE points_wallets ADD COLUMN IF NOT EXISTS subscription_balance
    BIGINT NOT NULL DEFAULT 0 CHECK (subscription_balance >= 0);
ALTER TABLE points_wallets ADD COLUMN IF NOT EXISTS granted_balance
    BIGINT NOT NULL DEFAULT 0 CHECK (granted_balance >= 0);
ALTER TABLE points_wallets ADD COLUMN IF NOT EXISTS registration_balance
    BIGINT NOT NULL DEFAULT 0 CHECK (registration_balance >= 0);
ALTER TABLE points_wallets ADD COLUMN IF NOT EXISTS free_periodic_balance
    BIGINT NOT NULL DEFAULT 0 CHECK (free_periodic_balance >= 0);

-- total_balance is GENERATED ALWAYS AS (sum of the five) STORED. Drop &
-- re-add so the expression matches even if a stale generated column exists
-- from an earlier draft (the prior uncommitted draft may have dropped it).
ALTER TABLE points_wallets DROP COLUMN IF EXISTS total_balance;
ALTER TABLE points_wallets ADD COLUMN total_balance BIGINT GENERATED ALWAYS AS (
    topup_balance + subscription_balance + granted_balance
        + registration_balance + free_periodic_balance
) STORED CHECK (total_balance >= 0);

ALTER TABLE points_wallets ADD COLUMN IF NOT EXISTS total_recharged
    BIGINT NOT NULL DEFAULT 0 CHECK (total_recharged >= 0);
ALTER TABLE points_wallets ADD COLUMN IF NOT EXISTS total_consumed
    BIGINT NOT NULL DEFAULT 0 CHECK (total_consumed >= 0);
ALTER TABLE points_wallets ADD COLUMN IF NOT EXISTS total_topup_granted
    BIGINT NOT NULL DEFAULT 0 CHECK (total_topup_granted >= 0);
ALTER TABLE points_wallets ADD COLUMN IF NOT EXISTS total_subscription_granted
    BIGINT NOT NULL DEFAULT 0 CHECK (total_subscription_granted >= 0);

-- 2. Backfill: recompute each wallet's projection from the ledger so the
--    columns match the source of truth regardless of prior state. Idempotent
--    (a re-run produces the same values). Wallets with no ledger rows keep 0.
--
--    Each points_wallets row is unique per (realm_id, user_id, bucket_id) and
--    the projection is bucket-scoped, so the backfill is a per-row join on
--    exactly that key.
WITH ledger_agg AS (
    SELECT
        realm_id,
        user_id,
        bucket_id,
        COALESCE(SUM(remaining_amount) FILTER (WHERE credit_type = 'topup_credit'), 0) AS topup_balance,
        COALESCE(SUM(remaining_amount) FILTER (WHERE credit_type = 'subscription_credit'), 0) AS subscription_balance,
        COALESCE(SUM(remaining_amount) FILTER (WHERE credit_type = 'granted_credit'), 0) AS granted_balance,
        COALESCE(SUM(remaining_amount) FILTER (WHERE credit_type = 'registration_credit'), 0) AS registration_balance,
        COALESCE(SUM(remaining_amount) FILTER (WHERE credit_type = 'free_periodic_credit'), 0) AS free_periodic_balance,
        COALESCE(SUM(used_amount), 0) AS total_consumed,
        COALESCE(SUM(granted_amount) FILTER (WHERE credit_type = 'topup_credit'), 0) AS total_topup_granted,
        COALESCE(SUM(granted_amount) FILTER (WHERE credit_type = 'subscription_credit'), 0) AS total_subscription_granted
    FROM points_credit_ledger
    GROUP BY realm_id, user_id, bucket_id
)
UPDATE points_wallets w
SET
    topup_balance             = la.topup_balance,
    subscription_balance      = la.subscription_balance,
    granted_balance           = la.granted_balance,
    registration_balance      = la.registration_balance,
    free_periodic_balance     = la.free_periodic_balance,
    total_consumed            = la.total_consumed,
    total_topup_granted       = la.total_topup_granted,
    total_subscription_granted = la.total_subscription_granted,
    total_recharged           = la.total_topup_granted + la.total_subscription_granted,
    updated_at                = NOW()
FROM ledger_agg la
WHERE w.realm_id = la.realm_id
  AND w.user_id  = la.user_id
  AND w.bucket_id = la.bucket_id;
