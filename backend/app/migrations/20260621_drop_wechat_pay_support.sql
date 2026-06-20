-- WeChat Pay provider support removed; drop the dedicated order table.
-- (WeChat OAuth login and the infra-wechat SDK are retained.)
DROP TABLE IF EXISTS wechat_payment_order CASCADE;
