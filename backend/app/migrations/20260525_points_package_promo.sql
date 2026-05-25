-- Promotional points package fields
ALTER TABLE points_packages
    ADD COLUMN package_type VARCHAR(20) NOT NULL DEFAULT 'standard',
    ADD COLUMN original_price BIGINT NULL,
    ADD COLUMN promo_start_time TIMESTAMPTZ NULL,
    ADD COLUMN promo_end_time TIMESTAMPTZ NULL;
