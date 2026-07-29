-- 0011_pay_model.sql
--
-- pay_model data base (design pay_model §4.3 / DEC-pay_model-002/005/007/008):
--   1. Extend `provider_entitlement_mappings.billing_type` CHECK to accept the
--      new `non_renewing` value alongside `recurring` / `one_time`.
--   2. Add `provider_entitlement_mappings.service_duration_days` (INT NULL) —
--      the fixed service-period length (days) for non-renewing mappings
--      (DEC-pay_model-005). Kept semantically isolated from `validity_days`
--      (one-time credit expiry) by a dedicated CHECK.
--   3. Add `subscription.billing_type` (TEXT NOT NULL) — the billing-type
--      snapshot taken from the mapping at fulfillment time (DEC-pay_model-007),
--      so reconciliation/views/api-ext can filter without joining the mapping.
--
-- Product is not yet shipped (DEC-pay_model-008): there is no existing data to
-- backfill, so the new `subscription.billing_type` column is added NOT NULL
-- directly (no DEFAULT, no gradual rollout). This mirrors 0010's stance that no
-- rollback state needs covering (design §6.1 / §7).
--
-- PostgreSQL has no `ALTER CONSTRAINT` for CHECK value sets, so the
-- `chk_pem_billing_type` constraint is dropped and re-added (same DROP+ADD
-- pattern as 0010's `chk_pem_payment_provider`). The new value set is a strict
-- superset of the old one and still permits NULL (the domain field stays
-- `Option<BillingType>`), so existing rows continue to satisfy it.
--
-- Down migration is intentionally not provided: the repository maintains
-- unidirectional sqlx migrations (user decision 2026-07-27; this feature is
-- not yet shipped, so no rollback state needs covering per design §6.1 / §7).

-- 1. provider_entitlement_mappings.billing_type: accept non_renewing.
ALTER TABLE provider_entitlement_mappings
  DROP CONSTRAINT chk_pem_billing_type,
  ADD  CONSTRAINT chk_pem_billing_type
       CHECK (billing_type IS NULL OR billing_type IN ('recurring', 'one_time', 'non_renewing'));

-- 2. provider_entitlement_mappings.service_duration_days:
--    NULL except for non_renewing mappings, where it must be a positive integer.
--    `billing_type IS DISTINCT FROM 'non_renewing'` lets NULL/other types stay
--    valid without forcing a value; non_renewing requires NOT NULL and >= 1.
ALTER TABLE provider_entitlement_mappings
  ADD COLUMN service_duration_days INT;

ALTER TABLE provider_entitlement_mappings
  ADD CONSTRAINT chk_pem_service_duration_days
       CHECK (
           (billing_type IS DISTINCT FROM 'non_renewing')
           OR (service_duration_days IS NOT NULL AND service_duration_days >= 1)
       );

COMMENT ON COLUMN provider_entitlement_mappings.service_duration_days IS
  'Fixed service-period length in days; required (>=1) when billing_type = non_renewing, NULL otherwise (DEC-pay_model-005)';

-- 3. subscription.billing_type: snapshot of the mapping billing type at
--    fulfillment time. Only subscription-shape billing types land here
--    (recurring / non_renewing); one_time purchases never create a subscription
--    row. NOT NULL directly (no existing rows; DEC-pay_model-008).
ALTER TABLE subscription
  ADD COLUMN billing_type TEXT NOT NULL;

ALTER TABLE subscription
  ADD CONSTRAINT chk_subscription_billing_type
       CHECK (billing_type IN ('recurring', 'non_renewing'));

COMMENT ON COLUMN subscription.billing_type IS
  'Billing type snapshot from the entitlement mapping at fulfillment time (DEC-pay_model-007): recurring or non_renewing';
