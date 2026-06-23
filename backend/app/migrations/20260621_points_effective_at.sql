-- ====================================
-- Points Effective At Migration
-- ====================================
-- Migration: 20260621_points_effective_at
-- Description: Add effective_at column + CHECK constraint + partial index to points_credit_ledger,
--              and ledger_id FK to points_grant_records (A4 reclaim row-level positioning bridge).
-- Created: 2026-06-21
-- Design ref: .ai/design/point-time.md §4.3.2 / §4.3.3 / §8
-- ====================================
-- Additive only: no backfill (A6 project not in production; existing rows effective_at IS NULL
-- ⟺ immediately available, zero regression). No new tables. No points_wallets change. No
-- points_grant_schedules change.

-- (a) points_credit_ledger: effective_at column
-- effective_at semantics:
--   NULL           ⟺ immediately available (current behavior; existing rows stay available)
--   non-null value ⟺ enters the available set only at/after that time
-- Consumption selection AND derived balance both gate on (effective_at IS NULL OR effective_at <= NOW()),
-- so future-effective active rows are excluded from both until the effective moment — zero state
-- migration, zero delay (no pending→active state machine or status-flipping job required).
-- Anchored to the expected period boundary (subscription period_start, free-period next_grant_time).
ALTER TABLE points_credit_ledger ADD COLUMN effective_at TIMESTAMPTZ;

-- DB-layer guard preventing pre-grant from writing an inverted time window
-- (effective_at=period_start <= period_end <= expires_at always holds at pre-grant time).
ALTER TABLE points_credit_ledger ADD CONSTRAINT points_credit_ledger_effective_before_expires
    CHECK (effective_at IS NULL OR expires_at IS NULL OR effective_at <= expires_at);

-- Partial index covering the shared predicate for derived SUM (compute_available_balance /
-- compute_bucket_available_balances) and consumption selection — both filter on
-- status='active' AND remaining_amount>0 AND (effective_at IS NULL OR effective_at<=NOW())
-- AND (expires_at IS NULL OR expires_at>NOW()) grouped by (realm_id, user_id, bucket_id).
-- Indexes only active rows (small cardinality per user/bucket); INCLUDE carries the
-- covering columns so both SUM and selection are index-only scans.
CREATE INDEX idx_points_credit_ledger_bucket_avail
    ON points_credit_ledger (realm_id, user_id, bucket_id, status)
    INCLUDE (remaining_amount, effective_at, expires_at, credit_type)
    WHERE status = 'active';

COMMENT ON COLUMN points_credit_ledger.effective_at IS 'Expected effective time; NULL = immediately available, non-null = enters available set only at/after this time (consumption selection + derived balance predicate gating)';

-- (b) points_grant_records: ledger_id FK (A4 reclaim row-level positioning bridge)
-- Reverse-links the business idempotency key (schedule_id, period_number) — which lives only in
-- points_grant_records — to a unique ledger row in points_credit_ledger (which has no
-- schedule_id/period_number columns). This lets BE-D05 reclaim do row-level positioning via:
--   UPDATE points_credit_ledger SET status='revoked', revoked_amount = revoked_amount + remaining_amount
--   ... WHERE id IN (SELECT g.ledger_id FROM points_grant_records g
--                   WHERE g.schedule_id=$1 AND g.period_number=$2)
-- NOT NULL: A6 no existing rows, no backfill. All future writes come from
-- pregrant_next_period_atomic which INSERTs the ledger row in-tx first, fetches its id, then
-- writes points_grant_records(..., ledger_id).
-- ON DELETE RESTRICT: prevent orphaning a grant_record by deleting its ledger row.
-- Reuses the existing uk_points_grant_records_schedule_period UNIQUE index — no new index needed.
ALTER TABLE points_grant_records ADD COLUMN ledger_id UUID NOT NULL REFERENCES points_credit_ledger(id) ON DELETE RESTRICT;

COMMENT ON COLUMN points_grant_records.ledger_id IS 'FK to points_credit_ledger(id); bridges (schedule_id, period_number) idempotency key to a unique ledger row for row-level reclaim positioning (A4)';
