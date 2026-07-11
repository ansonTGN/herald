ALTER TABLE legal_agreement_version
    ADD COLUMN mode TEXT NOT NULL DEFAULT 'full_text',
    ADD COLUMN external_url TEXT,
    ADD CONSTRAINT legal_agreement_version_mode_chk CHECK (mode IN ('full_text', 'link')),
    ADD CONSTRAINT legal_agreement_version_mode_url_chk CHECK (mode = 'full_text' OR external_url IS NOT NULL);

ALTER TABLE legal_agreement_draft
    ADD COLUMN mode TEXT NOT NULL DEFAULT 'full_text',
    ADD COLUMN external_url TEXT,
    ADD CONSTRAINT legal_agreement_draft_mode_chk CHECK (mode IN ('full_text', 'link'));

COMMENT ON COLUMN legal_agreement_version.mode IS 'Agreement content mode: full_text or link';
COMMENT ON COLUMN legal_agreement_version.external_url IS 'External agreement URL when mode is link';
COMMENT ON COLUMN legal_agreement_draft.mode IS 'Draft agreement content mode: full_text or link';
COMMENT ON COLUMN legal_agreement_draft.external_url IS 'Draft external agreement URL when mode is link';
