-- Seed the built-in API Key Client App for the admin realm.
-- New realms get this via create_realm(), but the admin realm was created
-- by the initial migration before admin-api-client existed.

INSERT INTO client_app (id, realm_id, client_id, name, description, enabled, redirect_uris, session_ttl_seconds)
VALUES (uuidv7(), 'admin', 'admin-api-client', 'API Key Client', 'Built-in client for API key authentication', true, '[]'::jsonb, 1800)
ON CONFLICT (realm_id, client_id) DO NOTHING;
