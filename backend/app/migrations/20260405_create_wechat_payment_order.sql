CREATE TABLE IF NOT EXISTS wechat_payment_order (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    realm_id TEXT NOT NULL,
    user_id UUID NOT NULL,
    plan_id UUID NOT NULL,
    client_app_id UUID,
    out_trade_no TEXT NOT NULL UNIQUE,
    transaction_id TEXT,
    amount INTEGER NOT NULL,
    currency TEXT NOT NULL DEFAULT 'CNY',
    code_url TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'paid', 'closed', 'expired')),
    description TEXT,
    paid_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_wechat_order_realm_status ON wechat_payment_order (realm_id, status);
CREATE INDEX IF NOT EXISTS idx_wechat_order_user ON wechat_payment_order (user_id);
CREATE INDEX IF NOT EXISTS idx_wechat_order_expires ON wechat_payment_order (status, expires_at);
