-- ====================================
-- Up Migration
-- ====================================

-- 1. Create products table
CREATE TABLE products (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    realm_id TEXT NOT NULL,
    name TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    description TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_products_realm_name UNIQUE (realm_id, name)
);

CREATE INDEX idx_products_realm_id ON products(realm_id);
CREATE INDEX idx_products_realm_sort_order ON products(realm_id, sort_order);
CREATE INDEX idx_products_realm_enabled ON products(realm_id, enabled);

-- 2. Add product_id to plan table
ALTER TABLE plan ADD COLUMN product_id UUID;

-- 3. Create default products for each realm
INSERT INTO products (id, realm_id, name, title, description, sort_order, enabled)
SELECT
    uuidv7() as id,
    realm_id,
    'default' as name,
    'Default Product' as title,
    'Default product for migrated plans' as description,
    0 as sort_order,
    true as enabled
FROM (SELECT DISTINCT realm_id FROM plan) AS distinct_realms;

-- 4. Backfill plan.product_id
UPDATE plan
SET product_id = (
    SELECT id FROM products WHERE products.realm_id = plan.realm_id AND products.name = 'default'
);

-- 5. Make product_id NOT NULL
ALTER TABLE plan ALTER COLUMN product_id SET NOT NULL;

-- 6. Add foreign key constraint
ALTER TABLE plan ADD CONSTRAINT fk_plan_product
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE RESTRICT;

-- 7. Add indexes for product queries
CREATE INDEX idx_plan_product_id ON plan(product_id);
CREATE INDEX idx_plan_realm_product ON plan(realm_id, product_id);

-- ====================================
-- Down Migration (removed from up file)
-- ====================================
-- Down migrations should be in a separate .down.sql file if needed.
-- sqlx executes the entire .sql file, so including down SQL here
-- would undo the up migration immediately.
