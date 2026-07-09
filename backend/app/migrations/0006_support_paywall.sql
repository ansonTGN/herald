-- ====================================
-- Support Paywall
-- ====================================
-- Foundational data layer for the support-paywall feature (design §4.3.2 + §8).
-- Adds the role-grant configuration column on provider_entitlement_mappings, the
-- provenance + expiry columns on user_roles, relaxes the user_roles uniqueness
-- constraint to admit the same role from multiple sources (manual + payment),
-- adds the revoke-lookup partial index, and adds the retry backoff column on
-- payment_event for the processed=false sweep job (PaymentEventRetryJob).
--
-- No backfill: defaults are semantically correct (design §4.3.3).
--   - granted_role_ids '{}'  => no role grant on payment
--   - user_roles.source 'manual' => all historical rows treated as manual grants

-- ------------------------------------
-- provider_entitlement_mappings: role-grant configuration
-- ------------------------------------
ALTER TABLE provider_entitlement_mappings
    ADD COLUMN granted_role_ids UUID[] NOT NULL DEFAULT '{}'::uuid[];

COMMENT ON COLUMN provider_entitlement_mappings.granted_role_ids IS
    'Role IDs auto-granted on payment success (paywall). Empty = no role grant.';

-- ------------------------------------
-- user_roles: provenance + expiry columns
-- ------------------------------------
ALTER TABLE user_roles
    ADD COLUMN source TEXT NOT NULL DEFAULT 'manual',
    ADD COLUMN source_id TEXT NULL,
    ADD COLUMN expires_at TIMESTAMPTZ NULL;

COMMENT ON COLUMN user_roles.source IS
    'Grant origin: ''manual'' (hand-assigned) or ''payment'' (granted on payment success).';
COMMENT ON COLUMN user_roles.source_id IS
    'Payment origin identifier. one_time = payment attempt_id; subscription = subscription_id. NULL for manual grants.';
COMMENT ON COLUMN user_roles.expires_at IS
    'INFORMATIONAL/PROVENANCE ONLY: the subscription billing period end aligned '
    'at grant time. Not an authz TTL — there is NO background sweep that removes '
    'rows once this timestamp passes. Role removal is event-driven (ImmediateCancel '
    'webhook → revoke_roles_by_payment_source). NULL for manual or permanent '
    'one-time grants. Kept for observability/audit of when the grant was meant to lapse.';

-- Relax the principal-role uniqueness to admit the same role from multiple
-- sources (PRD §4.2 requires manual + payment grants to coexist). The old
-- 4-column index would block that; replace it with TWO partial unique indexes
-- (mirrors the account/0001 partial-index convention):
--
--   manual  path: dedup on (realm_id, principal_type, principal_id, role_id).
--                 source_id is always NULL for manual grants, so it is not part
--                 of the key — admin re-assign of an existing role stays an
--                 idempotent duplicate-key skip (api-admin user_roles.rs and
--                 infra authorization/mod.rs both rely on this).
--
--   payment path: dedup on (realm_id, principal_type, principal_id, role_id,
--                 source_id). source_id is the payment origin (attempt_id for
--                 one-time, subscription_id for subscription), so two distinct
--                 purchases/subscriptions granting the same role coexist and
--                 revoking one source leaves the other intact. This is the key
--                 the grant/revoke/idempotency code agrees on (BE-D03). Using a
--                 partial index (not NULLS NOT DISTINCT) avoids a PG>=15
--                 version dependency and keeps manual-grant dedup independent.
DROP INDEX IF EXISTS idx_user_roles_principal_role;

CREATE UNIQUE INDEX idx_user_roles_principal_role_manual
    ON user_roles(realm_id, principal_type, principal_id, role_id)
    WHERE source = 'manual';

CREATE UNIQUE INDEX idx_user_roles_principal_role_payment
    ON user_roles(realm_id, principal_type, principal_id, role_id, source_id)
    WHERE source = 'payment';

-- Revoke-lookup partial index: used by the payment-role revocation path to find
-- rows by payment origin.
CREATE INDEX idx_user_roles_source
    ON user_roles(source, source_id) WHERE source = 'payment';

-- ------------------------------------
-- payment_event: retry backoff column (M4 kill-criteria prerequisite)
-- ------------------------------------
ALTER TABLE payment_event
    ADD COLUMN next_retry_at TIMESTAMPTZ NULL;

COMMENT ON COLUMN payment_event.next_retry_at IS
    'Backoff-scheduled retry time for the processed=false sweep job (PaymentEventRetryJob). NULL = eligible for immediate retry.';
