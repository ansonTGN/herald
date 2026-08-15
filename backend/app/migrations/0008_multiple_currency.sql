-- ====================================
-- Multiple-currency preference
-- ====================================
-- User-level preferred currency override (ISO 4217 code). NULL means the
-- user has no override and resolution falls back to the realm default
-- currency stored in realm_config (config_type='billing',
-- config_key='default_currency'); no config-table change is needed there.

ALTER TABLE profile ADD COLUMN preferred_currency TEXT NULL;
