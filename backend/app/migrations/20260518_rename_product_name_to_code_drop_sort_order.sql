-- Rename product `name` to `code` and drop `sort_order`
ALTER TABLE products RENAME COLUMN name TO code;
ALTER TABLE products DROP COLUMN sort_order;

-- Update unique constraint
ALTER TABLE products DROP CONSTRAINT uq_products_realm_name;
ALTER TABLE products ADD CONSTRAINT uq_products_realm_code UNIQUE (realm_id, code);

-- Drop sort_order index (no longer needed)
DROP INDEX IF EXISTS idx_products_realm_sort_order;
