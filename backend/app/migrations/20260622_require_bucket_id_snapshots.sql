-- Require bucket routing on purchasable mappings and payment attempts.
-- Existing NULL rows cannot be routed safely; drop them instead of guessing.

DELETE FROM payment_attempts WHERE bucket_id IS NULL;
DELETE FROM provider_entitlement_mappings WHERE bucket_id IS NULL;

ALTER TABLE payment_attempts
    ALTER COLUMN bucket_id SET NOT NULL;

ALTER TABLE provider_entitlement_mappings
    ALTER COLUMN bucket_id SET NOT NULL;
