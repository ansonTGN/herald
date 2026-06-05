-- ====================================
-- Payment Attempts: Add 'completed' status
-- ====================================
-- The invoice provider policy tests insert payment_attempts with status='completed'.
-- This was previously not included in the CHECK constraint.

ALTER TABLE payment_attempts DROP CONSTRAINT chk_status;
ALTER TABLE payment_attempts ADD CONSTRAINT chk_status CHECK (status IN ('Pending', 'RequiresAction', 'Succeeded', 'Failed', 'Cancelled', 'Expired', 'completed'));
