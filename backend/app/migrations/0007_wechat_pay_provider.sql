-- ====================================
-- WeChat Pay provider support
-- ====================================
-- Widens the payment_provider CHECK constraints on payment_attempts and
-- provider_entitlement_mappings to allow 'wechat' (WeChat Pay v3 reuses the
-- unified payment_attempt pipeline, and WeChat entitlement mappings carry
-- payment_provider='wechat'). The original
-- 0002_billing.sql CHECKs enumerated only stripe/creem/apple/google.
--
-- payment_event has no provider CHECK (only a provider index + unique
-- constraint), so it accepts 'wechat' without change.

ALTER TABLE payment_attempts
    DROP CONSTRAINT IF EXISTS chk_payment_attempt_provider,
    ADD CONSTRAINT chk_payment_attempt_provider
        CHECK (payment_provider IN ('stripe', 'creem', 'apple', 'google', 'wechat'));

ALTER TABLE provider_entitlement_mappings
    DROP CONSTRAINT IF EXISTS chk_pem_payment_provider,
    ADD CONSTRAINT chk_pem_payment_provider
        CHECK (payment_provider IN ('stripe', 'creem', 'apple', 'google', 'wechat'));
