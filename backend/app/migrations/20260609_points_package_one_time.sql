-- ====================================
-- Points Package One-Time Payment
-- ====================================
-- Migration: 20260609_points_package_one_time
-- Description: Drop old points package tables, constrain payment_attempts.target_type to 'entitlement_mapping'
-- Created: 2026-06-09
-- ====================================

-- Step 1: Clear test data with old target_type values (project not in production)
DELETE FROM payment_attempts WHERE target_type IN ('subscription_entitlement', 'points_package');

-- Step 2: Drop old CHECK constraint
ALTER TABLE payment_attempts DROP CONSTRAINT IF EXISTS chk_target_type;

-- Step 3: Add new CHECK constraint - only 'entitlement_mapping' is valid now
ALTER TABLE payment_attempts ADD CONSTRAINT chk_target_type CHECK (target_type = 'entitlement_mapping');

-- Step 4: Drop purchase records table (depends on points_packages via payment_attempt references)
DROP TABLE IF EXISTS points_package_purchases;

-- Step 5: Drop payment provider mapping table (depends on points_packages via FK)
DROP TABLE IF EXISTS points_package_payment_providers;

-- Step 6: Drop points package catalog table
DROP TABLE IF EXISTS points_packages;
