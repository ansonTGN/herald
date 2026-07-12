-- One effective template per scope and type keeps locale/realm fallback deterministic.
CREATE UNIQUE INDEX uq_email_templates_realm_type
    ON email_templates (realm_id, type)
    WHERE realm_id IS NOT NULL;

CREATE UNIQUE INDEX uq_email_templates_global_type
    ON email_templates (type)
    WHERE realm_id IS NULL;

COMMENT ON COLUMN email_templates.type IS
    'Template kind with optional locale suffix, e.g. verify_email or verify_email:zh-CN';

COMMENT ON COLUMN email_templates.content IS
    'JSON object with subject, text and html fields; allowed variables: brand_name and action_url';

-- down migration
DROP INDEX IF EXISTS uq_email_templates_global_type;
DROP INDEX IF EXISTS uq_email_templates_realm_type;
