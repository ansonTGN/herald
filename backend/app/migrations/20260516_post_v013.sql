-- Post v0.1.3 schema changes

-- Add realm description
ALTER TABLE realm ADD COLUMN description text;

-- Add account index for realm-scoped queries
CREATE INDEX idx_account_realm_created ON account(realm_id, created_at DESC);

-- Product catalog: rename name→code, drop sort_order
ALTER TABLE products RENAME COLUMN name TO code;
ALTER TABLE products DROP COLUMN sort_order;
ALTER TABLE products DROP CONSTRAINT uq_products_realm_name;
ALTER TABLE products ADD CONSTRAINT uq_products_realm_code UNIQUE (realm_id, code);
DROP INDEX IF EXISTS idx_products_realm_sort_order;
