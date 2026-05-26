-- ====================================
-- Auth Module Migration
-- ====================================
-- Consolidates auth-related schema additions after core init.

-- WeChat OAuth support: union_id lookup for cross-app user matching.
CREATE INDEX IF NOT EXISTS idx_provider_union_id
ON provider(realm_id, union_id)
WHERE union_id IS NOT NULL;

-- OAuth user linking table
CREATE TABLE user_oauth_providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    realm_id TEXT NOT NULL,
    provider_type text NOT NULL,
    provider_user_id TEXT NOT NULL,
    open_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT user_oauth_providers_user_id_fkey
        FOREIGN KEY (user_id) REFERENCES account(id) ON DELETE CASCADE,
    CONSTRAINT user_oauth_providers_unique
        UNIQUE (realm_id, provider_type, provider_user_id)
);

CREATE INDEX user_oauth_providers_user_id_idx
    ON user_oauth_providers(user_id);

CREATE INDEX user_oauth_providers_realm_provider_idx
    ON user_oauth_providers(realm_id, provider_type);

COMMENT ON TABLE user_oauth_providers IS 'Links user accounts to OAuth providers (GitHub, Google, etc.)';
COMMENT ON COLUMN user_oauth_providers.id IS 'Unique identifier for the OAuth provider link';
COMMENT ON COLUMN user_oauth_providers.user_id IS 'Reference to the user account';
COMMENT ON COLUMN user_oauth_providers.realm_id IS 'Realm identifier';
COMMENT ON COLUMN user_oauth_providers.provider_type IS 'OAuth provider type (github, google, facebook, apple, wechat)';
COMMENT ON COLUMN user_oauth_providers.provider_user_id IS 'User ID from the OAuth provider';
COMMENT ON COLUMN user_oauth_providers.open_id IS 'Open ID from the OAuth provider';
