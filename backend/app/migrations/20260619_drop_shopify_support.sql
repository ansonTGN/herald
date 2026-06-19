-- Shopify provider support removed; drop dedicated binding tables.
DROP TABLE IF EXISTS shopify_subscription_binding CASCADE;
DROP TABLE IF EXISTS shopify_user_binding CASCADE;
