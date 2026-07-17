-- BE-D01: Migrate Cloudflare Turnstile from realm_config to client_app (D-PROTECT-01).
--
-- Turnstile human-verification is fully delegated to the Client App. The
-- legacy realm_config `config_type='turnstile'` rows are NOT migrated here
-- (design §4.3.3: historical realm-level Turnstile config is not carried over
-- automatically; realm admins reconfigure Turnstile per Client App after
-- deploy). `turnstile_enabled` defaults to false so existing Client Apps keep
-- the "not configured -> skip" behaviour until explicitly enabled.

ALTER TABLE client_app
    ADD COLUMN turnstile_enabled boolean NOT NULL DEFAULT false,
    ADD COLUMN turnstile_site_key text,
    ADD COLUMN turnstile_secret_key text;

COMMENT ON COLUMN client_app.turnstile_enabled IS
    'Whether Cloudflare Turnstile human-verification is enforced for this Client App (D-PROTECT-01)';
COMMENT ON COLUMN client_app.turnstile_site_key IS
    'Cloudflare Turnstile site key (public) shown to the client widget; NULL when Turnstile is disabled';
COMMENT ON COLUMN client_app.turnstile_secret_key IS
    'Cloudflare Turnstile secret key (server-side, sensitive); NULL when Turnstile is disabled';

-- down migration
ALTER TABLE client_app
    DROP COLUMN IF EXISTS turnstile_secret_key,
    DROP COLUMN IF EXISTS turnstile_site_key,
    DROP COLUMN IF EXISTS turnstile_enabled;
