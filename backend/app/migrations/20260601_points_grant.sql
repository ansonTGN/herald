-- ====================================
-- Points Grant Feature Migration
-- ====================================
-- Adds granted_balance column, updates total_balance generated column,
-- and expands CHECK constraints for grant-related enum values.

BEGIN;

-- 1. Add granted_balance column
ALTER TABLE points_wallets ADD COLUMN granted_balance BIGINT NOT NULL DEFAULT 0;

-- 2. Replace total_balance generated column (must DROP + re-ADD in a single transaction)
ALTER TABLE points_wallets DROP COLUMN total_balance;
ALTER TABLE points_wallets ADD COLUMN total_balance BIGINT GENERATED ALWAYS AS (
    topup_balance + subscription_balance + granted_balance
) STORED;

-- 3. Add CHECK constraint for granted_balance
ALTER TABLE points_wallets ADD CONSTRAINT points_wallets_granted_balance_check
    CHECK (granted_balance >= 0);

-- 4. Expand CHECK constraints on points_transactions (dynamic constraint lookup)
DO $$
DECLARE
  _type_constraint_name TEXT;
  _credit_type_constraint_name TEXT;
BEGIN
  SELECT conname INTO _type_constraint_name
    FROM pg_constraint c JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = ANY(c.conkey)
    WHERE c.conrelid = 'points_transactions'::regclass AND a.attname = 'type' AND c.contype = 'c';

  IF _type_constraint_name IS NOT NULL THEN
    EXECUTE format('ALTER TABLE points_transactions DROP CONSTRAINT %I', _type_constraint_name);
  END IF;

  SELECT conname INTO _credit_type_constraint_name
    FROM pg_constraint c JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = ANY(c.conkey)
    WHERE c.conrelid = 'points_transactions'::regclass AND a.attname = 'credit_type' AND c.contype = 'c';

  IF _credit_type_constraint_name IS NOT NULL THEN
    EXECUTE format('ALTER TABLE points_transactions DROP CONSTRAINT %I', _credit_type_constraint_name);
  END IF;
END $$;

ALTER TABLE points_transactions ADD CONSTRAINT points_transactions_type_check
    CHECK (type IN ('recharge','consume','subscription_grant','subscription_renewal','subscription_upgrade','registration_grant','free_periodic_grant','refund_revoke','expire_revoke','cancel_revoke','idempotency_record','expiration','refund','grant'));

ALTER TABLE points_transactions ADD CONSTRAINT points_transactions_credit_type_check
    CHECK (credit_type IN ('topup_credit','subscription_credit','registration_credit','free_periodic_credit','granted_credit'));

-- 5. Expand CHECK constraints on points_credit_ledger (dynamic constraint lookup)
DO $$
DECLARE
  _credit_type_constraint_name TEXT;
  _source_type_constraint_name TEXT;
BEGIN
  SELECT conname INTO _credit_type_constraint_name
    FROM pg_constraint c JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = ANY(c.conkey)
    WHERE c.conrelid = 'points_credit_ledger'::regclass AND a.attname = 'credit_type' AND c.contype = 'c';

  IF _credit_type_constraint_name IS NOT NULL THEN
    EXECUTE format('ALTER TABLE points_credit_ledger DROP CONSTRAINT %I', _credit_type_constraint_name);
  END IF;

  SELECT conname INTO _source_type_constraint_name
    FROM pg_constraint c JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = ANY(c.conkey)
    WHERE c.conrelid = 'points_credit_ledger'::regclass AND a.attname = 'source_type' AND c.contype = 'c';

  IF _source_type_constraint_name IS NOT NULL THEN
    EXECUTE format('ALTER TABLE points_credit_ledger DROP CONSTRAINT %I', _source_type_constraint_name);
  END IF;
END $$;

ALTER TABLE points_credit_ledger ADD CONSTRAINT points_credit_ledger_credit_type_check
    CHECK (credit_type IN ('topup_credit','subscription_credit','registration_credit','free_periodic_credit','granted_credit'));

ALTER TABLE points_credit_ledger ADD CONSTRAINT points_credit_ledger_source_type_check
    CHECK (source_type IN ('subscription_initial','subscription_renewal','subscription_upgrade','topup','system_grant','registration','free_periodic_grant','admin_grant','sdk_grant'));

COMMIT;

-- ====================================
-- Verification queries (run manually to validate)
-- ====================================
-- Verify total_balance unchanged for existing rows
-- SELECT COUNT(*) FROM points_wallets WHERE total_balance != topup_balance + subscription_balance;
-- Expected: 0 rows

-- Verify granted_balance defaults to 0
-- SELECT COUNT(*) FROM points_wallets WHERE granted_balance != 0;
-- Expected: 0 rows
