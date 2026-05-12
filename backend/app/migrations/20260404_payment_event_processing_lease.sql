ALTER TABLE payment_event
    ADD COLUMN IF NOT EXISTS processing_started_at TIMESTAMPTZ;

COMMENT ON COLUMN payment_event.processing_started_at
    IS 'When webhook processing last claimed the event for execution; null means idle';
