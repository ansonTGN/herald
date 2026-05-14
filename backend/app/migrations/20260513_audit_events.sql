-- ====================================
-- Audit Events Table
-- ====================================
-- Append-only audit log for security and compliance.
-- No foreign keys (audit table doesn't reference business tables).
-- No updated_at column (audit events are immutable).

CREATE TABLE audit_events (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    realm_id TEXT NOT NULL,
    category TEXT NOT NULL,
    action TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    actor_type TEXT,
    actor_name TEXT,
    target_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    target_name TEXT,
    result TEXT NOT NULL,
    details JSONB,
    ip_address TEXT,
    user_agent TEXT,
    trace_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- List query primary index (realm + time descending)
CREATE INDEX idx_audit_events_realm_created ON audit_events(realm_id, created_at DESC);

-- Filter by event type
CREATE INDEX idx_audit_events_realm_category_action ON audit_events(realm_id, category, action);

-- Filter by actor
CREATE INDEX idx_audit_events_realm_actor ON audit_events(realm_id, actor_id);

-- Filter by IP address (security forensics)
CREATE INDEX idx_audit_events_realm_ip ON audit_events(realm_id, ip_address);

COMMENT ON TABLE audit_events IS 'Append-only audit event log for security and compliance';
COMMENT ON COLUMN audit_events.category IS 'Event category: user_management, rbac, realm_management, auth';
COMMENT ON COLUMN audit_events.action IS 'Specific operation within category';
COMMENT ON COLUMN audit_events.actor_type IS 'Actor classification: user, admin, system';
COMMENT ON COLUMN audit_events.result IS 'Operation result: success, failure';
COMMENT ON COLUMN audit_events.details IS 'Extensible JSONB payload (change diffs, failure reasons, etc.)';
