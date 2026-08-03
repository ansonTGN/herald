-- ====================================
-- Passkey Authentication
-- ====================================
-- Adds WebAuthn/FIDO2 passkey credentials. Realm passkey configuration reuses
-- realm_config with config_type = 'passkey'; no configuration table is added.

CREATE TABLE user_passkey_credential (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id UUID NOT NULL REFERENCES account(id) ON DELETE CASCADE,
    realm_id TEXT NOT NULL,
    rp_id TEXT NOT NULL,
    credential_id BYTEA NOT NULL,
    credential_public_key BYTEA NOT NULL,
    counter BIGINT NOT NULL DEFAULT 0,
    transports JSONB NOT NULL DEFAULT '[]',
    aaguid UUID NULL,
    backup_eligible BOOLEAN NOT NULL DEFAULT false,
    backup_state BOOLEAN NOT NULL DEFAULT false,
    user_verified BOOLEAN NOT NULL DEFAULT false,
    nickname TEXT NULL,
    last_used_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX uniq_user_passkey_credential_realm_user_rp_cred
    ON user_passkey_credential(realm_id, user_id, rp_id, credential_id);
CREATE INDEX idx_user_passkey_credential_user
    ON user_passkey_credential(user_id);
CREATE INDEX idx_user_passkey_credential_realm
    ON user_passkey_credential(realm_id);
CREATE INDEX idx_user_passkey_credential_rp
    ON user_passkey_credential(rp_id);

COMMENT ON TABLE user_passkey_credential IS 'User WebAuthn/FIDO2 passkey credentials';
COMMENT ON COLUMN user_passkey_credential.user_id IS 'Account that owns this passkey credential';
COMMENT ON COLUMN user_passkey_credential.realm_id IS 'Realm isolation boundary for the credential';
COMMENT ON COLUMN user_passkey_credential.credential_id IS 'Authenticator credential ID, unique within a realm';
COMMENT ON COLUMN user_passkey_credential.credential_public_key IS 'COSE public key returned by the authenticator';
COMMENT ON COLUMN user_passkey_credential.counter IS 'Signature counter used for cloned credential detection';
COMMENT ON COLUMN user_passkey_credential.transports IS 'Authenticator transports, e.g. ["usb","internal"]';
COMMENT ON COLUMN user_passkey_credential.aaguid IS 'Authenticator attestation GUID';
COMMENT ON COLUMN user_passkey_credential.backup_eligible IS 'Whether the credential is eligible for backup/sync';
COMMENT ON COLUMN user_passkey_credential.backup_state IS 'Current credential backup/sync state';
COMMENT ON COLUMN user_passkey_credential.user_verified IS 'Whether user verification was satisfied at registration';
COMMENT ON COLUMN user_passkey_credential.nickname IS 'User-visible passkey device name';
COMMENT ON COLUMN user_passkey_credential.last_used_at IS 'Most recent successful use timestamp';
