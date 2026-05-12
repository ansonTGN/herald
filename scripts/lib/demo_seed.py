"""
Demo Seed Data Bootstrap

=======================================
职责说明（MANDATORY）
=======================================

此脚本属于【环境初始化层】，不受 Demo 测试规范限制。

允许的操作：
- ✅ 通过 HTTP API 创建基础 Realm 和 User
- ✅ 直接数据库操作创建复杂业务数据

职责：
1. 创建可重用的基础测试数据
2. 初始化 Realm、User、Points System、Subscription History
3. 在 Demo 环境启动时执行

注意：
- 此脚本仅在 Demo 环境初始化时执行
- Demo 测试代码不应直接调用此脚本
- Demo 测试代码必须通过 UI 验证功能，不创建数据（参考 spec/demo/e2e-testing.md）

=======================================
创建的数据
=======================================

Realm: realm-001
- Admin: admin@realm-001.com
- User: user@realm-001.com
- Points Demo App
- Subscription with history events (created, upgraded, renewed)

Points System:
- 账户余额：充值 3000，订阅 1900
- 事务记录：充值、消费事件
- 积分明细：订阅积分、充值积分

Subscription History:
- Test User: user1@demo.com
- 测试订阅和历史事件（created, upgraded, renewed）
"""

from __future__ import annotations

import json
import urllib.error
import urllib.request
from http.cookiejar import CookieJar
from pathlib import Path
from typing import TYPE_CHECKING, Any

from . import docker

if TYPE_CHECKING:
    from .logger import Logger


BACKEND_PORT = 8080
POSTGRES_CONTAINER = "cas-demo-postgres"
POSTGRES_DB = "herald_demo"
POSTGRES_USER = "postgres"

ADMIN_REALM = "admin"
ADMIN_EMAIL = "admin@cas.com"
ADMIN_PASSWORD = "password"
ADMIN_CLIENT_ID = "admin-web-console"

POINTS_REALM_ID = "realm-001"
POINTS_REALM_NAME = "Realm 001"
POINTS_REALM_ADMIN_EMAIL = "admin@realm-001.com"
POINTS_REALM_ADMIN_PASSWORD = "password"
POINTS_USER_EMAIL = "user@realm-001.com"
POINTS_USER_PASSWORD = "password"
POINTS_CLIENT_APP_ID = "points-demo-app"

# Subscription history demo data constants
SUBSCRIPTION_TEST_USER_EMAIL = "user1@demo.com"
SUBSCRIPTION_TEST_USER_PASSWORD = "password123"
SUBSCRIPTION_TEST_CLIENT_ID = "admin-web-console"


class SeedError(RuntimeError):
    """Raised when demo seed data cannot be established."""


def ensure_demo_seed_data(logger: "Logger | None" = None) -> bool:
    """Ensure the reusable demo environment contains deterministic demo seed data."""
    try:
        _info(logger, "Ensuring demo seed data for realm-001...")
        admin_opener = _login(ADMIN_REALM, ADMIN_EMAIL, ADMIN_PASSWORD)
        _ensure_points_realm(admin_opener, logger)

        realm_admin_opener = _login(
            POINTS_REALM_ID,
            POINTS_REALM_ADMIN_EMAIL,
            POINTS_REALM_ADMIN_PASSWORD,
        )
        user_id = _ensure_points_user(realm_admin_opener, logger)
        _seed_points_data(user_id, logger)
        _ensure_points_package_payment_demo_data(logger)

        # Ensure subscription history demo data for admin realm
        _info(logger, "Ensuring subscription history demo data...")
        _ensure_subscription_history_demo_data(admin_opener, logger)

        # Ensure subscription data for realm-001 (used by subscription timeline tests)
        _info(logger, "Ensuring subscription data for realm-001...")
        _ensure_realm001_subscription_data(logger)

        # Ensure realm default points config for admin realm
        _info(logger, "Ensuring admin realm default points config...")
        _ensure_admin_realm_points_config(logger)

        # Ensure Shopify unclaimed subscription for realm-001 (used by subscription claim tests)
        _info(logger, "Ensuring Shopify unclaimed subscription for realm-001...")
        _ensure_shopify_unclaimed_subscription(logger)

        _info(logger, "Demo seed data is ready")
        return True
    except SeedError as exc:
        _error(logger, f"Demo seed failed: {exc}")
        return False


def _login(realm_id: str, email: str, password: str) -> urllib.request.OpenerDirector:
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    payload = {
        "clientId": ADMIN_CLIENT_ID,
        "email": email,
        "password": password,
    }
    data = json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(
        _backend_url(f"/api/auth/{realm_id}/login"),
        data=data,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with opener.open(request, timeout=15) as response:
            body_text = response.read().decode("utf-8")
            body = json.loads(body_text) if body_text else {}
            token = _extract_cookie_token(response.headers.get("Set-Cookie", ""), "X-Auth")
            if response.status != 200:
                raise SeedError(f"Login failed for {email} in {realm_id}: status={response.status}")
    except urllib.error.HTTPError as exc:
        error_body = exc.read().decode("utf-8", errors="replace")
        raise SeedError(f"HTTP {exc.code} for login {email} in {realm_id}: {error_body}") from exc
    except urllib.error.URLError as exc:
        raise SeedError(f"HTTP request failed for login {email} in {realm_id}: {exc}") from exc

    if body.get("requiresTotp") is True:
        raise SeedError(f"Login for {email} in {realm_id} unexpectedly requires TOTP")
    if not token:
        raise SeedError(f"Login for {email} in {realm_id} did not return X-Auth cookie")

    opener.addheaders = [("Cookie", f"X-Auth={token}")]
    return opener


def _ensure_points_realm(opener: urllib.request.OpenerDirector, logger: "Logger | None") -> None:
    realm_exists = _sql_scalar(
        f"SELECT 1 FROM realm WHERE id = '{POINTS_REALM_ID}' LIMIT 1;"
    )
    if realm_exists == "1":
        _info(logger, "realm-001 already exists")
        _ensure_registration_enabled(logger)
        return

    _info(logger, "Creating realm-001 via HTTP API...")
    payload = {
        "id": POINTS_REALM_ID,
        "name": POINTS_REALM_NAME,
        "adminUser": {
            "email": POINTS_REALM_ADMIN_EMAIL,
            "password": POINTS_REALM_ADMIN_PASSWORD,
        },
    }
    status, body = _http_json(
        opener,
        "POST",
        _backend_url("/api/realms"),
        payload=payload,
        expected_statuses=(201,),
    )
    if status != 201:
        raise SeedError(
            f"Failed to create realm-001: status={status}, body={json.dumps(body, ensure_ascii=False)}"
        )

    _ensure_registration_enabled(logger)


def _ensure_registration_enabled(logger: "Logger | None") -> None:
    """Ensure registration is enabled for realm-001."""
    _info(logger, "Ensuring registration is enabled for realm-001...")
    _sql_exec(
        f"""
        INSERT INTO realm_config (realm_id, config_type, config_key, config_value, is_secret, enabled, metadata)
        VALUES ('{POINTS_REALM_ID}', 'registration', 'allowed', 'true', false, true, '{{}}')
        ON CONFLICT (realm_id, config_type, config_key)
        DO UPDATE SET enabled = true, config_value = 'true';
        """
    )
    _info(logger, "Registration enabled for realm-001")


def _ensure_points_user(opener: urllib.request.OpenerDirector, logger: "Logger | None") -> str:
    user_role_id = _sql_scalar(
        "SELECT id::text FROM roles "
        f"WHERE realm_id = '{POINTS_REALM_ID}' AND name = 'user' AND client_id = '{ADMIN_CLIENT_ID}' "
        "LIMIT 1;"
    )
    if not user_role_id:
        raise SeedError("Could not find realm-001 user role")

    user_id = _sql_scalar(
        "SELECT id::text FROM account "
        f"WHERE realm_id = '{POINTS_REALM_ID}' AND email = '{POINTS_USER_EMAIL}' "
        "LIMIT 1;"
    )

    if not user_id:
        _info(logger, "Creating user@realm-001.com via HTTP API...")
        payload = {
            "email": POINTS_USER_EMAIL,
            "password": POINTS_USER_PASSWORD,
            "nickname": "Points Demo User",
            "status": 1,
            "roleIds": [user_role_id],
        }
        status, body = _http_json(
            opener,
            "POST",
            _backend_url(f"/api/users/{POINTS_REALM_ID}"),
            payload=payload,
            expected_statuses=(201,),
        )
        if status != 201:
            raise SeedError(
                f"Failed to create user@realm-001.com: status={status}, body={json.dumps(body, ensure_ascii=False)}"
            )
        user_id = body.get("data", {}).get("id") or body.get("id")

    if not user_id:
        user_id = _sql_scalar(
            "SELECT id::text FROM account "
            f"WHERE realm_id = '{POINTS_REALM_ID}' AND email = '{POINTS_USER_EMAIL}' "
            "LIMIT 1;"
        )
    if not user_id:
        raise SeedError("Could not resolve user@realm-001.com ID after creation")

    _info(logger, "Ensuring user role assignment for user@realm-001.com...")
    status, body = _http_json(
        opener,
        "PUT",
        _backend_url(f"/api/users/{POINTS_REALM_ID}/{user_id}/roles"),
        payload={"roleIds": [user_role_id]},
        expected_statuses=(200,),
    )
    if status != 200:
        raise SeedError(
            f"Failed to update user roles: status={status}, body={json.dumps(body, ensure_ascii=False)}"
        )
    return user_id


def _seed_points_data(user_id: str, logger: "Logger | None") -> None:
    _info(logger, "Resetting points demo data for user@realm-001.com...")
    sql = f"""
DO $$
DECLARE
    v_user_id UUID := '{user_id}'::uuid;
    v_client_app_id UUID;
    v_account_id UUID := uuidv7();
    v_subscription_ledger_id UUID := uuidv7();
    v_topup_ledger_id UUID := uuidv7();
    v_tx_subscription UUID := uuidv7();
    v_tx_topup UUID := uuidv7();
    v_tx_consume_1 UUID := uuidv7();
    v_tx_consume_2 UUID := uuidv7();
BEGIN
    INSERT INTO client_app (
        id, realm_id, client_id, name, description, redirect_uris, enabled, session_ttl_seconds
    ) VALUES (
        uuidv7(),
        '{POINTS_REALM_ID}',
        '{POINTS_CLIENT_APP_ID}',
        'Points Demo App',
        'Client app used by realm-001 points demo data',
        '["http://localhost:3000/callback"]'::jsonb,
        TRUE,
        1800
    )
    ON CONFLICT (realm_id, client_id) DO UPDATE
        SET name = EXCLUDED.name,
            description = EXCLUDED.description,
            redirect_uris = EXCLUDED.redirect_uris,
            enabled = TRUE,
            session_ttl_seconds = EXCLUDED.session_ttl_seconds
    RETURNING id INTO v_client_app_id;

    DELETE FROM points_consumption_allocations WHERE user_id = v_user_id;
    DELETE FROM points_revocation_records WHERE user_id = v_user_id;
    DELETE FROM points_transactions WHERE user_id = v_user_id;
    DELETE FROM points_credit_ledger WHERE user_id = v_user_id;
    DELETE FROM points_accounts WHERE user_id = v_user_id;

    INSERT INTO points_accounts (
        id,
        user_id,
        realm_id,
        topup_balance,
        subscription_balance,
        total_topup_granted,
        total_subscription_granted,
        total_recharged,
        total_consumed,
        status,
        created_at,
        updated_at
    ) VALUES (
        v_account_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        3000,
        1900,
        3000,
        2000,
        5000,
        100,
        'active',
        TIMESTAMPTZ '2026-03-10 09:00:00+00',
        TIMESTAMPTZ '2026-03-22 16:00:00+00'
    );

    INSERT INTO points_credit_ledger (
        id,
        user_id,
        realm_id,
        credit_type,
        source_type,
        source_id,
        granted_amount,
        used_amount,
        revoked_amount,
        expires_at,
        status,
        created_at,
        updated_at
    ) VALUES (
        v_subscription_ledger_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        'subscription_credit',
        'subscription_initial',
        'demo-subscription-202603',
        2000,
        100,
        0,
        TIMESTAMPTZ '2026-04-15 00:00:00+00',
        'active',
        TIMESTAMPTZ '2026-03-10 09:00:00+00',
        TIMESTAMPTZ '2026-03-22 16:00:00+00'
    );

    INSERT INTO points_credit_ledger (
        id,
        user_id,
        realm_id,
        credit_type,
        source_type,
        source_id,
        granted_amount,
        used_amount,
        revoked_amount,
        expires_at,
        status,
        created_at,
        updated_at
    ) VALUES (
        v_topup_ledger_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        'topup_credit',
        'topup',
        'demo-topup-202603',
        3000,
        0,
        0,
        NULL,
        'active',
        TIMESTAMPTZ '2026-03-12 10:00:00+00',
        TIMESTAMPTZ '2026-03-12 10:00:00+00'
    );

    INSERT INTO points_transactions (
        id,
        account_id,
        user_id,
        realm_id,
        type,
        amount,
        balance_after,
        description,
        client_app_id,
        credit_type,
        topup_balance_after,
        subscription_balance_after,
        created_at
    ) VALUES
    (
        v_tx_subscription,
        v_account_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        'recharge',
        2000,
        2000,
        'Monthly subscription bonus',
        NULL,
        'subscription_credit',
        0,
        2000,
        TIMESTAMPTZ '2026-03-10 09:00:00+00'
    ),
    (
        v_tx_topup,
        v_account_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        'recharge',
        3000,
        5000,
        'Top-up purchase',
        NULL,
        'topup_credit',
        3000,
        2000,
        TIMESTAMPTZ '2026-03-12 10:00:00+00'
    ),
    (
        v_tx_consume_1,
        v_account_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        'consume',
        -40,
        4960,
        'API usage',
        v_client_app_id,
        'subscription_credit',
        3000,
        1960,
        TIMESTAMPTZ '2026-03-21 15:00:00+00'
    ),
    (
        v_tx_consume_2,
        v_account_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        'consume',
        -60,
        4900,
        'Premium feature usage',
        v_client_app_id,
        'subscription_credit',
        3000,
        1900,
        TIMESTAMPTZ '2026-03-22 16:00:00+00'
    );

    INSERT INTO points_consumption_allocations (
        id,
        transaction_id,
        ledger_id,
        user_id,
        realm_id,
        allocated_amount,
        ledger_remaining_after,
        created_at
    ) VALUES
    (
        uuidv7(),
        v_tx_consume_1,
        v_subscription_ledger_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        40,
        1960,
        TIMESTAMPTZ '2026-03-21 15:00:00+00'
    ),
    (
        uuidv7(),
        v_tx_consume_2,
        v_subscription_ledger_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        60,
        1900,
        TIMESTAMPTZ '2026-03-22 16:00:00+00'
    );
END $$;
"""
    _sql_exec(sql)


def _ensure_realm001_subscription_data(logger: "Logger | None") -> None:
    """Ensure subscription data exists for realm-001 (subscription timeline tests)."""
    sql = f"""
DO $$
DECLARE
    v_client_app_id UUID;
    v_plan_id UUID;
    v_subscription_id UUID;
    v_product_id UUID;
    v_test_timestamp TIMESTAMPTZ := TIMESTAMPTZ '2026-03-24 12:00:00+00';
BEGIN
    -- Ensure product exists
    INSERT INTO products (id, realm_id, name, title, description, sort_order, enabled)
    VALUES (uuidv7(), '{POINTS_REALM_ID}', 'default', 'Default Product', 'Default product for realm-001 demo', 0, TRUE)
    ON CONFLICT (realm_id, name) DO UPDATE SET title = EXCLUDED.title
    RETURNING id INTO v_product_id;

    -- Get existing client app
    SELECT id INTO v_client_app_id
    FROM client_app
    WHERE realm_id = '{POINTS_REALM_ID}' AND client_id = '{POINTS_CLIENT_APP_ID}'
    LIMIT 1;

    IF v_client_app_id IS NULL THEN
        RAISE EXCEPTION 'Client app {POINTS_CLIENT_APP_ID} not found in {POINTS_REALM_ID}';
    END IF;

    -- Create billing plan
    INSERT INTO plan (
        id, realm_id, name, description, title, type, price, currency,
        active, trial_days, sort_order, product_id
    ) VALUES (
        uuidv7(),
        '{POINTS_REALM_ID}',
        'realm001-subscription-plan',
        'Subscription plan for realm-001 demo',
        'Realm 001 Subscription Plan',
        'monthly',
        1000,
        'USD',
        TRUE,
        0,
        0,
        v_product_id
    )
    ON CONFLICT (realm_id, name) DO UPDATE
        SET title = EXCLUDED.title,
            description = EXCLUDED.description,
            active = TRUE
    RETURNING id INTO v_plan_id;

    -- Assign payment provider to plan
    INSERT INTO plan_payment_provider (id, plan_id, payment_provider, external_product_id)
    VALUES (uuidv7(), v_plan_id, 'stripe', 'realm001-product-subscription')
    ON CONFLICT (plan_id, payment_provider) DO UPDATE
        SET external_product_id = EXCLUDED.external_product_id;

    -- Assign plan to client app
    INSERT INTO client_app_plan (id, client_app_id, plan_id, enabled)
    VALUES (uuidv7(), v_client_app_id, v_plan_id, TRUE)
    ON CONFLICT (client_app_id, plan_id) DO UPDATE
        SET enabled = TRUE;

    -- Create subscription
    DELETE FROM subscription WHERE client_app_id = v_client_app_id;

    INSERT INTO subscription (
        id, realm_id, external_subscription_id, external_product_id,
        payment_provider, status, tier, current_period_start,
        current_period_end, plan_id, client_app_id, billing_period
    ) VALUES (
        uuidv7(),
        '{POINTS_REALM_ID}',
        'sub_realm001_' || uuidv7(),
        'prod_realm001_' || uuidv7(),
        'stripe',
        'active',
        'premium',
        v_test_timestamp - INTERVAL '30 days',
        v_test_timestamp + INTERVAL '30 days',
        v_plan_id,
        v_client_app_id,
        'monthly'
    )
    RETURNING id INTO v_subscription_id;

    -- Create subscription history events
    DELETE FROM subscription_history WHERE subscription_id = v_subscription_id;

    INSERT INTO subscription_history (
        id, subscription_id, event_type, timestamp, actor,
        changes, previous_state, new_state, realm_id
    ) VALUES
    (
        uuidv7()::text,
        v_subscription_id,
        'created',
        v_test_timestamp - INTERVAL '25 days',
        'admin@realm-001.com',
        '{{"tier": ["free", "premium"]}}'::jsonb,
        '{{"status": "inactive", "tier": "free"}}'::jsonb,
        '{{"status": "active", "tier": "premium"}}'::jsonb,
        '{POINTS_REALM_ID}'
    ),
    (
        uuidv7()::text,
        v_subscription_id,
        'upgraded',
        v_test_timestamp - INTERVAL '15 days',
        'admin@realm-001.com',
        '{{"tier": ["basic", "premium"]}}'::jsonb,
        '{{"tier": "basic"}}'::jsonb,
        '{{"tier": "premium"}}'::jsonb,
        '{POINTS_REALM_ID}'
    ),
    (
        uuidv7()::text,
        v_subscription_id,
        'renewed',
        v_test_timestamp - INTERVAL '5 days',
        'system',
        '{{"renewal_count": [1, 2]}}'::jsonb,
        '{{"renewal_count": 1}}'::jsonb,
        '{{"renewal_count": 2}}'::jsonb,
        '{POINTS_REALM_ID}'
    );

    RAISE NOTICE 'Subscription data created for {POINTS_REALM_ID}';
END $$;
"""
    _sql_exec(sql)
    _info(logger, "[OK] Subscription data ready for realm-001")


def _ensure_points_package_payment_demo_data(logger: "Logger | None") -> None:
    """Ensure demo points packages have Stripe payment mappings."""
    _info(logger, "Ensuring points package payment demo data...")
    wechat_private_key = (
        Path("backend/api/src/fixtures/rsa_test_key.pem")
        .read_text(encoding="utf-8")
        .strip()
        .replace("'", "''")
    )
    sql = f"""
DO $$
DECLARE
    v_package_id UUID;
BEGIN
    INSERT INTO realm_config (
        realm_id, config_type, config_key, config_value, is_secret, enabled, metadata
    ) VALUES
        ('{POINTS_REALM_ID}', 'stripe', 'api_key', 'sk_test_demo_points_package', true, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'stripe', 'publishable_key', 'pk_test_demo_points_package', false, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'stripe', 'webhook_secret', 'whsec_demo_points_package', true, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'stripe', 'timeout', '30', false, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'stripe', 'mock_base_url', 'mock://stripe', false, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'wechat', 'app_id', 'wx_demo_points_package', false, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'wechat', 'mch_id', 'mch_demo_points_package', false, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'wechat', 'serial_no', 'serial_demo_points_package', false, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'wechat', 'v3_key', 'abcd1234567890abcdef1234567890ab', true, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'wechat', 'private_key', '{wechat_private_key}', true, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'wechat', 'notify_url', 'https://example.com/api/third/pay/{POINTS_REALM_ID}/wechat/webhooks', false, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'wechat', 'mock_base_url', 'mock://wechat', false, true, '{{}}'::jsonb)
    ON CONFLICT (realm_id, config_type, config_key) DO UPDATE
        SET config_value = EXCLUDED.config_value,
            is_secret = EXCLUDED.is_secret,
            enabled = true,
            metadata = EXCLUDED.metadata,
            updated_at = now();

    INSERT INTO points_packages (
        id, realm_id, name, title, description, points, price, currency, sort_order, enabled
    ) VALUES (
        uuidv7(),
        '{POINTS_REALM_ID}',
        'credits-500',
        '500 Credits',
        'Demo top-up package for Stripe payment flow',
        500,
        500,
        'USD',
        10,
        true
    )
    ON CONFLICT (realm_id, name) DO UPDATE
        SET title = EXCLUDED.title,
            description = EXCLUDED.description,
            points = EXCLUDED.points,
            price = EXCLUDED.price,
            currency = EXCLUDED.currency,
            enabled = true,
            updated_at = now()
    RETURNING id INTO v_package_id;

    INSERT INTO points_package_payment_providers (
        id, points_package_id, payment_provider, enabled, external_product_id
    ) VALUES
        (
            uuidv7(),
            v_package_id,
            'stripe',
            true,
            'prod_demo_points_500'
        ),
        (
            uuidv7(),
            v_package_id,
            'wechat',
            true,
            'wx_demo_points_500'
        )
    ON CONFLICT (points_package_id, payment_provider) DO UPDATE
        SET enabled = true,
            external_product_id = EXCLUDED.external_product_id,
            updated_at = now();
END $$;
"""
    _sql_exec(sql)
    _info(logger, "[OK] Points package payment demo data ready")


def _ensure_admin_realm_points_config(logger: "Logger | None") -> None:
    """Ensure realm_default_configs row exists for admin realm with correct seed values."""
    _info(logger, "Ensuring admin realm default points config...")
    sql = f"""
    INSERT INTO realm_default_configs (realm_id, registration_bonus_points, free_periodic_points_amount, free_periodic_grant_period_type, free_periodic_validity_days)
    VALUES ('{ADMIN_REALM}', 1000, 50, 'daily', 1)
    ON CONFLICT (realm_id) DO UPDATE SET
        registration_bonus_points = EXCLUDED.registration_bonus_points,
        free_periodic_points_amount = EXCLUDED.free_periodic_points_amount,
        free_periodic_grant_period_type = EXCLUDED.free_periodic_grant_period_type,
        free_periodic_validity_days = EXCLUDED.free_periodic_validity_days;
    """
    _sql_exec(sql)


def _ensure_subscription_history_demo_data(admin_opener: urllib.request.OpenerDirector, logger: "Logger | None") -> None:
    """Ensure subscription history demo data exists for the admin realm."""
    # Check if test user exists
    test_user_id = _sql_scalar(
        "SELECT id::text FROM account "
        f"WHERE realm_id = '{ADMIN_REALM}' AND email = '{SUBSCRIPTION_TEST_USER_EMAIL}' "
        "LIMIT 1;"
    )

    if not test_user_id:
        # Create test user via API
        _info(logger, f"Creating {SUBSCRIPTION_TEST_USER_EMAIL} via HTTP API...")

        # Get user role for admin realm
        user_role_id = _sql_scalar(
            "SELECT id::text FROM roles "
            f"WHERE realm_id = '{ADMIN_REALM}' AND name = 'user' AND client_id = '{ADMIN_CLIENT_ID}' "
            "LIMIT 1;"
        )

        if not user_role_id:
            raise SeedError("Could not find admin realm user role")

        payload = {
            "email": SUBSCRIPTION_TEST_USER_EMAIL,
            "password": SUBSCRIPTION_TEST_USER_PASSWORD,
            "nickname": "Subscription Test User",
            "status": 1,
            "roleIds": [user_role_id],
        }
        status, body = _http_json(
            admin_opener,
            "POST",
            _backend_url(f"/api/users/{ADMIN_REALM}"),
            payload=payload,
            expected_statuses=(201,),
        )
        if status != 201:
            raise SeedError(
                f"Failed to create {SUBSCRIPTION_TEST_USER_EMAIL}: status={status}, body={json.dumps(body, ensure_ascii=False)}"
            )
        test_user_id = body.get("data", {}).get("id") or body.get("id")

    if not test_user_id:
        # Query again to get the user ID
        test_user_id = _sql_scalar(
            "SELECT id::text FROM account "
            f"WHERE realm_id = '{ADMIN_REALM}' AND email = '{SUBSCRIPTION_TEST_USER_EMAIL}' "
            "LIMIT 1;"
        )

    if not test_user_id:
        raise SeedError(f"Could not resolve {SUBSCRIPTION_TEST_USER_EMAIL} ID after creation")

    _info(logger, f"Ensuring subscription history test data for {SUBSCRIPTION_TEST_USER_EMAIL}...")

    # Use SQL to create or update subscription demo data
    sql = f"""
DO $$
DECLARE
    v_user_id UUID := '{test_user_id}'::uuid;
    v_client_app_id UUID;
    v_plan_id UUID;
    v_subscription_id UUID;
    v_product_id UUID;
    v_test_timestamp TIMESTAMPTZ := TIMESTAMPTZ '2026-03-24 12:00:00+00';
BEGIN
    -- Ensure default product exists
    INSERT INTO products (id, realm_id, name, title, description, sort_order, enabled)
    VALUES (uuidv7(), '{ADMIN_REALM}', 'default', 'Default Product', 'Default product for demo seed', 0, TRUE)
    ON CONFLICT (realm_id, name) DO UPDATE SET title = EXCLUDED.title
    RETURNING id INTO v_product_id;

    -- Get or create client app (use admin-web-console)
    SELECT id INTO v_client_app_id
    FROM client_app
    WHERE realm_id = '{ADMIN_REALM}' AND client_id = '{SUBSCRIPTION_TEST_CLIENT_ID}'
    LIMIT 1;

    IF v_client_app_id IS NULL THEN
        RAISE EXCEPTION 'Client app {SUBSCRIPTION_TEST_CLIENT_ID} not found in {ADMIN_REALM}';
    END IF;

    -- Create billing plan
    INSERT INTO plan (
        id, realm_id, name, description, title, type, price, currency,
        active, trial_days, sort_order, product_id
    ) VALUES (
        uuidv7(),
        '{ADMIN_REALM}',
        'test-subscription-plan',
        'Test subscription plan for demo',
        'Test Subscription Plan',
        'monthly',
        1000,
        'USD',
        TRUE,
        0,
        0,
        v_product_id
    )
    ON CONFLICT (realm_id, name) DO UPDATE
        SET title = EXCLUDED.title,
            description = EXCLUDED.description,
            active = TRUE
    RETURNING id INTO v_plan_id;

    -- Assign payment provider to plan
    INSERT INTO plan_payment_provider (id, plan_id, payment_provider, external_product_id)
    VALUES (uuidv7(), v_plan_id, 'stripe', 'test-product-subscription')
    ON CONFLICT (plan_id, payment_provider) DO UPDATE
        SET external_product_id = EXCLUDED.external_product_id;

    -- Assign plan to client app
    INSERT INTO client_app_plan (id, client_app_id, plan_id, enabled)
    VALUES (uuidv7(), v_client_app_id, v_plan_id, TRUE)
    ON CONFLICT (client_app_id, plan_id) DO UPDATE
        SET enabled = TRUE;

    -- Create subscription
    DELETE FROM subscription WHERE client_app_id = v_client_app_id;

    INSERT INTO subscription (
        id, realm_id, external_subscription_id, external_product_id,
        payment_provider, status, tier, current_period_start,
        current_period_end, plan_id, client_app_id, billing_period
    ) VALUES (
        uuidv7(),
        '{ADMIN_REALM}',
        'sub_demo_' || uuidv7(),
        'prod_demo_' || uuidv7(),
        'stripe',
        'active',
        'premium',
        v_test_timestamp - INTERVAL '30 days',
        v_test_timestamp + INTERVAL '30 days',
        v_plan_id,
        v_client_app_id,
        'monthly'
    )
    RETURNING id INTO v_subscription_id;

    -- Create subscription history events
    DELETE FROM subscription_history WHERE subscription_id = v_subscription_id;

    INSERT INTO subscription_history (
        id, subscription_id, event_type, timestamp, actor,
        changes, previous_state, new_state, realm_id
    ) VALUES
    (
        uuidv7()::text,
        v_subscription_id,
        'created',
        v_test_timestamp - INTERVAL '25 days',
        'admin@cas.com',
        '{{"tier": ["free", "premium"]}}'::jsonb,
        '{{"status": "inactive", "tier": "free"}}'::jsonb,
        '{{"status": "active", "tier": "premium"}}'::jsonb,
        '{ADMIN_REALM}'
    ),
    (
        uuidv7()::text,
        v_subscription_id,
        'upgraded',
        v_test_timestamp - INTERVAL '15 days',
        'admin@cas.com',
        '{{"tier": ["basic", "premium"]}}'::jsonb,
        '{{"tier": "basic"}}'::jsonb,
        '{{"tier": "premium"}}'::jsonb,
        '{ADMIN_REALM}'
    ),
    (
        uuidv7()::text,
        v_subscription_id,
        'renewed',
        v_test_timestamp - INTERVAL '5 days',
        'system',
        '{{"renewal_count": [1, 2]}}'::jsonb,
        '{{"renewal_count": 1}}'::jsonb,
        '{{"renewal_count": 2}}'::jsonb,
        '{ADMIN_REALM}'
    );

    RAISE NOTICE 'Subscription history demo data created for user %', v_user_id;
END $$;
"""
    _sql_exec(sql)
    _info(logger, f"[OK] Subscription history demo data ready for {SUBSCRIPTION_TEST_USER_EMAIL}")


def _http_json(
    opener: urllib.request.OpenerDirector,
    method: str,
    url: str,
    payload: dict[str, Any] | None = None,
    expected_statuses: tuple[int, ...] = (200,),
) -> tuple[int, dict[str, Any]]:
    data = None
    headers = {"Content-Type": "application/json"}
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")

    request = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with opener.open(request, timeout=15) as response:
            body = response.read().decode("utf-8")
            parsed = json.loads(body) if body else {}
            if response.status not in expected_statuses:
                raise SeedError(
                    f"Unexpected HTTP status {response.status} for {method} {url}: {body}"
                )
            return response.status, parsed
    except urllib.error.HTTPError as exc:
        error_body = exc.read().decode("utf-8", errors="replace")
        parsed = _safe_json_loads(error_body)
        if exc.code in expected_statuses:
            return exc.code, parsed
        raise SeedError(f"HTTP {exc.code} for {method} {url}: {error_body}") from exc
    except urllib.error.URLError as exc:
        raise SeedError(f"HTTP request failed for {method} {url}: {exc}") from exc


def _sql_scalar(sql: str) -> str | None:
    code, output = docker.exec_check(
        POSTGRES_CONTAINER,
        ["psql", "-U", POSTGRES_USER, "-d", POSTGRES_DB, "-At", "-c", sql],
    )
    if code != 0:
        raise SeedError(f"SQL scalar query failed: {sql}")
    value = output.strip()
    return value or None


def _sql_exec(sql: str) -> None:
    code, output = docker.exec_check(
        POSTGRES_CONTAINER,
        ["psql", "-v", "ON_ERROR_STOP=1", "-U", POSTGRES_USER, "-d", POSTGRES_DB, "-c", sql],
    )
    if code != 0:
        raise SeedError(f"SQL execution failed: {output}")


def _backend_url(path: str) -> str:
    return f"http://localhost:{BACKEND_PORT}{path}"


def _extract_cookie_token(set_cookie_header: str, name: str) -> str | None:
    if not set_cookie_header:
        return None
    first_part = set_cookie_header.split(';', 1)[0].strip()
    prefix = f"{name}="
    if not first_part.startswith(prefix):
        return None
    return first_part[len(prefix):]


def _safe_json_loads(value: str) -> dict[str, Any]:
    try:
        parsed = json.loads(value)
        return parsed if isinstance(parsed, dict) else {"raw": parsed}
    except json.JSONDecodeError:
        return {"raw": value}


def _info(logger: "Logger | None", message: str) -> None:
    # Changed to verbose_info to reduce noise in normal output
    if logger:
        logger.verbose_info(message)


def _error(logger: "Logger | None", message: str) -> None:
    if logger:
        logger.error(message)


def _ensure_shopify_unclaimed_subscription(logger: "Logger | None") -> None:
    """Ensure an unclaimed Shopify subscription exists for realm-001."""
    _info(logger, "Creating unclaimed Shopify subscription for realm-001...")

    # Check if unclaimed subscription already exists
    existing_count = _sql_scalar(
        f"""
        SELECT COUNT(*)::text
        FROM subscription s
        INNER JOIN shopify_subscription_binding b ON b.subscription_id = s.id
        WHERE s.realm_id = '{POINTS_REALM_ID}'
          AND s.user_id IS NULL
          AND b.customer_id = 'shopify_test_customer_001'
        LIMIT 1;
        """
    )

    if existing_count and existing_count != "0":
        _info(logger, "Unclaimed Shopify subscription already exists for realm-001")
        return

    # Get the plan ID from realm-001
    plan_id = _sql_scalar(
        f"""
        SELECT id::text
        FROM plan
        WHERE realm_id = '{POINTS_REALM_ID}'
          AND name = 'realm001-subscription-plan'
        LIMIT 1;
        """
    )

    if not plan_id:
        _info(logger, "Creating billing plan for Shopify unclaimed subscription...")
        # Create a plan if it doesn't exist
        sql = f"""
        DO $$
        DECLARE
            v_product_id UUID;
            v_plan_id UUID;
        BEGIN
            -- Ensure product exists
            INSERT INTO products (id, realm_id, name, title, description, sort_order, enabled)
            VALUES (uuidv7(), '{POINTS_REALM_ID}', 'shopify-default', 'Shopify Default Product', 'Default product for Shopify demo', 0, TRUE)
            ON CONFLICT (realm_id, name) DO UPDATE SET title = EXCLUDED.title
            RETURNING id INTO v_product_id;

            -- Create billing plan
            INSERT INTO plan (
                id, realm_id, name, description, title, type, price, currency,
                active, trial_days, sort_order, product_id
            ) VALUES (
                uuidv7(),
                '{POINTS_REALM_ID}',
                'realm001-subscription-plan',
                'Subscription plan for realm-001 demo',
                'Realm 001 Subscription Plan',
                'monthly',
                1000,
                'USD',
                TRUE,
                0,
                0,
                v_product_id
            )
            ON CONFLICT (realm_id, name) DO UPDATE
                SET title = EXCLUDED.title,
                    description = EXCLUDED.description,
                    active = TRUE
            RETURNING id INTO v_plan_id;

            -- Assign payment provider to plan
            INSERT INTO plan_payment_provider (id, plan_id, payment_provider, external_product_id)
            VALUES (uuidv7(), v_plan_id, 'shopify', 'shopify-product-subscription')
            ON CONFLICT (plan_id, payment_provider) DO UPDATE
                SET external_product_id = EXCLUDED.external_product_id;

            RAISE NOTICE 'Plan created: %', v_plan_id;
        END $$;
        """
        _sql_exec(sql)
        plan_id = _sql_scalar(
            f"""
            SELECT id::text
            FROM plan
            WHERE realm_id = '{POINTS_REALM_ID}'
              AND name = 'realm001-subscription-plan'
            LIMIT 1;
            """
        )

    if not plan_id:
        raise SeedError("Failed to create or find billing plan for Shopify subscription")

    # Create unclaimed subscription via SQL (simpler than HTTP API for demo seed)
    sql = f"""
    DO $$
    DECLARE
        v_subscription_id UUID := uuidv7();
        v_plan_id UUID := '{plan_id}'::uuid;
    BEGIN
        -- Create subscription with user_id = NULL (unclaimed)
        INSERT INTO subscription (
            id, realm_id, user_id, external_subscription_id, external_product_id,
            payment_provider, status, tier, current_period_start,
            current_period_end, plan_id, billing_period, created_at, updated_at
        ) VALUES (
            v_subscription_id,
            '{POINTS_REALM_ID}',
            NULL,  -- This is the key: user_id is NULL (unclaimed)
            'shopify_test_sub_' || v_subscription_id,
            'shopify_test_product',
            'shopify',
            'active',
            'premium',
            NOW(),
            NOW() + INTERVAL '30 days',
            v_plan_id,
            'monthly',
            NOW(),
            NOW()
        );

        -- Create Shopify subscription binding
        INSERT INTO shopify_subscription_binding (
            subscription_id, realm_id, shop_domain, customer_id,
            contract_id, contract_gid, contract_revision_id, last_order_id, created_at, updated_at
        ) VALUES (
            v_subscription_id,
            '{POINTS_REALM_ID}',
            'demo-store.myshopify.com',
            'shopify_test_customer_001',
            'gid://shopify/SubscriptionContract/001',
            'gid://shopify/SubscriptionContract/001',
            1,
            'gid://shopify/Order/001',
            NOW(),
            NOW()
        );

        RAISE NOTICE 'Unclaimed Shopify subscription created: %', v_subscription_id;
    END $$;
    """
    _sql_exec(sql)
    _info(logger, "[OK] Unclaimed Shopify subscription created for realm-001")

