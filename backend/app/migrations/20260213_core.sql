-- ====================================
-- Core Module Migration
-- ====================================
-- Consolidates post-init core schema adjustments.

DROP INDEX IF EXISTS idx_client_api_keys_key;

CREATE UNIQUE INDEX idx_client_api_keys_key_unique
    ON client_api_keys(api_key_hash);

ALTER TABLE client_api_keys
    ADD COLUMN client_app_id UUID REFERENCES client_app(id) ON DELETE SET NULL;

CREATE INDEX idx_client_api_keys_client_app_id
    ON client_api_keys(client_app_id);
CREATE INDEX idx_client_api_keys_app_realm
    ON client_api_keys(client_app_id, realm_id);

COMMENT ON INDEX idx_client_api_keys_key_unique IS 'Unique index for O(1) API key hash lookups';
COMMENT ON COLUMN client_api_keys.client_app_id IS 'Client App this API key belongs to (1:1 relationship)';
