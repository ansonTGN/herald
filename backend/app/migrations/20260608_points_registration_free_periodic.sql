-- ====================================
-- Points Registration & Free Periodic Balance Buckets
-- ====================================
-- Adds registration_balance and free_periodic_balance columns to points_wallets,
-- migrates existing data from topup_balance, and updates the generated total_balance column.

BEGIN;

-- 1. Add new balance columns
ALTER TABLE points_wallets ADD COLUMN registration_balance BIGINT NOT NULL DEFAULT 0;
ALTER TABLE points_wallets ADD COLUMN free_periodic_balance BIGINT NOT NULL DEFAULT 0;

-- 2. Migrate existing data: compute registration and free_periodic balances from ledgers
-- and adjust topup_balance accordingly (topup_balance currently aggregates topup + registration + free_periodic)
UPDATE points_wallets w
SET
    registration_balance = COALESCE((
        SELECT SUM(remaining_amount)
        FROM points_credit_ledger l
        WHERE l.user_id = w.user_id
          AND l.realm_id = w.realm_id
          AND l.credit_type = 'registration_credit'
          AND l.status = 'active'
    ), 0),
    free_periodic_balance = COALESCE((
        SELECT SUM(remaining_amount)
        FROM points_credit_ledger l
        WHERE l.user_id = w.user_id
          AND l.realm_id = w.realm_id
          AND l.credit_type = 'free_periodic_credit'
          AND l.status = 'active'
    ), 0);

-- 3. Adjust topup_balance to exclude registration and free_periodic (keep only actual topup)
UPDATE points_wallets
SET topup_balance = topup_balance - registration_balance - free_periodic_balance;

-- 4. Add CHECK constraints for new columns
ALTER TABLE points_wallets ADD CONSTRAINT points_wallets_registration_balance_check
    CHECK (registration_balance >= 0);
ALTER TABLE points_wallets ADD CONSTRAINT points_wallets_free_periodic_balance_check
    CHECK (free_periodic_balance >= 0);

-- 5. Replace total_balance generated column to include all 5 balance types
ALTER TABLE points_wallets DROP COLUMN total_balance;
ALTER TABLE points_wallets ADD COLUMN total_balance BIGINT GENERATED ALWAYS AS (
    topup_balance + subscription_balance + granted_balance + registration_balance + free_periodic_balance
) STORED;

COMMIT;
