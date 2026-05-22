-- Generalize user_roles to support any Principal (user, api_key, client)
--
-- Adds principal_type and principal_id columns so that user_roles can bind
-- roles to any principal, not just users. Existing rows are backfilled with
-- principal_type='user' and principal_id=user_id::text.

-- Step 1: Add principal columns
ALTER TABLE user_roles
    ADD COLUMN principal_type TEXT NOT NULL DEFAULT 'user';

ALTER TABLE user_roles
    ADD COLUMN principal_id TEXT;

-- Backfill existing rows: set principal_id = user_id::text
UPDATE user_roles
SET principal_id = user_id::text
WHERE principal_id IS NULL;

-- Backfill assertion (only runs in PL/pgSQL context where ASSERT is supported)
DO $$ BEGIN
    ASSERT (SELECT COUNT(*) FROM user_roles WHERE principal_id IS NULL) = 0,
        'Backfill failed: some user_roles rows still have NULL principal_id';
END $$;

ALTER TABLE user_roles
    ALTER COLUMN principal_id SET NOT NULL;

-- Step 2: Relax user_id for API Key Principal rows (nullable)
ALTER TABLE user_roles DROP CONSTRAINT IF EXISTS fk_user_roles_user;
ALTER TABLE user_roles ALTER COLUMN user_id DROP NOT NULL;

-- Step 3: Relax client_id for API Key Principal rows (nullable)
ALTER TABLE user_roles ALTER COLUMN client_id DROP NOT NULL;

-- Step 4: Drop old unique constraint
ALTER TABLE user_roles DROP CONSTRAINT IF EXISTS uq_user_roles;

-- Step 5: New unique index on principal columns
CREATE UNIQUE INDEX IF NOT EXISTS idx_user_roles_principal_role
    ON user_roles (realm_id, principal_type, principal_id, role_id);

-- Update comments
COMMENT ON COLUMN user_roles.principal_type IS 'Type of principal: "user", "api_key", or "client"';
COMMENT ON COLUMN user_roles.principal_id IS 'ID of the principal (user_id for users, client_api_keys.id for API keys)';
COMMENT ON COLUMN user_roles.user_id IS 'UUID of the user (nullable for non-user principals)';
COMMENT ON COLUMN user_roles.client_id IS 'Client app identifier (nullable for non-user principals)';
