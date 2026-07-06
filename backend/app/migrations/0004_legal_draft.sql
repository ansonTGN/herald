-- ====================================
-- Herald Legal Agreement Draft
-- ====================================
-- Per-realm draft of a custom legal agreement, staged before publish.
--
-- Design rationale: `legal_agreement_version` is append-only and every read
-- path (current_effective / next_custom_version_no / has_custom / list_history)
-- assumes "each row = a published version". Mixing drafts into that table would
-- pollute all four paths, the version_no sequence, and the consent FK. A
-- separate draft table leaves the published-history table fully untouched —
-- drafts never affect end-user resolution, the source indicator, or consent.
--
-- One row per (realm_id, agreement_type): a realm edits a single draft per
-- agreement type; saving again overwrites it (last-write-wins via the unique
-- constraint + ON CONFLICT DO UPDATE in the repository).
--
-- `content` mirrors the published-version shape (locale→body JSONB map, e.g.
-- {"en": "..."}). `version_label` is the optional label captured at draft time
-- so publish-from-draft can reuse it without the admin re-entering it.

CREATE TABLE legal_agreement_draft (
    id             UUID PRIMARY KEY DEFAULT uuidv7(),
    realm_id       TEXT NOT NULL,
    agreement_type TEXT NOT NULL,
    content        JSONB NOT NULL,
    version_label  TEXT,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by     TEXT
);

-- One draft per (realm, agreement_type). Repository upserts rely on this for
-- INSERT ... ON CONFLICT DO UPDATE.
CREATE UNIQUE INDEX legal_agreement_draft_realm_type_unique
    ON legal_agreement_draft (realm_id, agreement_type);

COMMENT ON TABLE legal_agreement_draft IS 'Per-realm draft of a custom legal agreement, staged before publish (one row per realm+type)';
COMMENT ON COLUMN legal_agreement_draft.agreement_type IS 'terms_of_service | privacy_policy';
COMMENT ON COLUMN legal_agreement_draft.content IS 'JSONB { [locale]: body } — staged body pending publish';
COMMENT ON COLUMN legal_agreement_draft.updated_by IS 'Last editing user_id';
