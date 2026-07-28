-- 0010_iap_provider_check.sql
--
-- Extend the two `payment_provider` CHECK constraints to accept the IAP
-- providers (`apple`, `google`) alongside the existing `stripe` / `creem`.
-- Design support-iap §4.3.3.
--
-- PostgreSQL has no `ALTER CONSTRAINT` for CHECK value sets, so the
-- constraints are dropped and re-added. The new value set is a strict
-- superset of the old one, so existing stripe/creem rows continue to
-- satisfy the constraint (no data backfill required).
--
-- Down migration is intentionally not provided: the repository maintains
-- unidirectional sqlx migrations (user decision 2026-07-27; this feature is
-- not yet shipped, so no rollback state needs covering per design §6.1 /
-- §7).

ALTER TABLE payment_attempts
  DROP CONSTRAINT chk_payment_attempt_provider,
  ADD  CONSTRAINT chk_payment_attempt_provider
       CHECK (payment_provider IN ('stripe', 'creem', 'apple', 'google'));

ALTER TABLE provider_entitlement_mappings
  DROP CONSTRAINT chk_pem_payment_provider,
  ADD  CONSTRAINT chk_pem_payment_provider
       CHECK (payment_provider IN ('stripe', 'creem', 'apple', 'google'));
