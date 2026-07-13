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
        # Pre-establish admin@cas.com legal consent BEFORE logging in. When the
        # admin realm has global legal-agreement versions, login returns
        # consentRequired=true and never sets the X-Auth cookie, so the seed
        # _login below fails (chicken-and-egg). This is pure SQL — the admin
        # account is created by backend migration, so it already exists here.
        _ensure_current_legal_consent(ADMIN_REALM, ADMIN_EMAIL, logger)
        admin_opener = _login(ADMIN_REALM, ADMIN_EMAIL, ADMIN_PASSWORD)
        _ensure_points_realm(admin_opener, logger)
        _ensure_current_legal_consent(POINTS_REALM_ID, POINTS_REALM_ADMIN_EMAIL, logger)

        # Ensure credit buckets BEFORE any points/mapping/payment seed (bucket_id is NOT NULL).
        _ensure_credit_buckets(logger)

        realm_admin_opener = _login(
            POINTS_REALM_ID,
            POINTS_REALM_ADMIN_EMAIL,
            POINTS_REALM_ADMIN_PASSWORD,
        )
        user_id = _ensure_points_user(realm_admin_opener, logger)
        _seed_points_data(user_id, logger)
        # Re-bind bucket coverage now that `_seed_points_data` has created the
        # `points-demo-app` client app in realm-001. `_ensure_credit_buckets`
        # runs BEFORE `_seed_points_data` (so wallet/ledger rows can reference a
        # bucket_id), so its first coverage-binding pass no-ops with a "client
        # app not yet seeded" warning. This second pass is the one that actually
        # writes `credit_bucket_client_apps` rows on every seed run, making the
        # coverage set deterministic without requiring a manual second re-seed.
        # (In-slot seed fix; admin realm has no points-demo-app so its pass
        # no-ops cleanly.)
        for realm_id in (POINTS_REALM_ID, ADMIN_REALM):
            _ensure_realm_bucket_directory(realm_id, logger)

        # Ensure realm-001 subscription reference data. This includes the
        # recurring entitlement mapping (`realm001-product-subscription`) that
        # US-PW-001 expects to exist before the demo test opens the page.
        _ensure_realm001_subscription_data(user_id, logger)

        # Ensure payment provider credentials (realm_config only). This function
        # does NOT seed entitlement mappings; the realm-001 recurring mapping is
        # handled by `_ensure_realm001_subscription_data` above.
        _ensure_payment_provider_config(logger)

        # Ensure realm default points config for admin realm
        _info(logger, "Ensuring admin realm default points config...")
        _ensure_admin_realm_points_config(logger)

        # Ensure audit seed data for admin realm (realm_management events)
        _info(logger, "Ensuring admin realm audit seed data...")
        _ensure_admin_realm_audit_events(logger)

        # Ensure invoice seller config for realm-001
        _info(logger, "Ensuring invoice seller config for realm-001...")
        _ensure_invoice_seller_config(logger)

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
        "description": "Demo realm for testing and demonstration purposes",
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
        VALUES ('{POINTS_REALM_ID}', 'registration', 'enabled', 'true', false, true, '{{}}')
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

    _ensure_current_legal_consent(POINTS_REALM_ID, POINTS_USER_EMAIL, logger)

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


def _ensure_current_legal_consent(realm_id: str, email: str, logger: "Logger | None") -> None:
    """Record consent to the current effective legal agreements for a seeded user."""
    _info(logger, f"Ensuring legal consent for {email} in {realm_id}...")
    sql = f"""
DO $$
DECLARE
    v_user_id UUID;
    v_agreement_type TEXT;
    v_version_id UUID;
BEGIN
    SELECT id INTO v_user_id
    FROM account
    WHERE realm_id = '{realm_id}' AND email = '{email}'
    LIMIT 1;

    IF v_user_id IS NULL THEN
        RAISE EXCEPTION 'Could not find seeded user % in realm %', '{email}', '{realm_id}';
    END IF;

    FOREACH v_agreement_type IN ARRAY ARRAY['terms_of_service', 'privacy_policy']
    LOOP
        SELECT id INTO v_version_id
        FROM legal_agreement_version
        WHERE agreement_type = v_agreement_type
          AND (realm_id = '{realm_id}' OR realm_id IS NULL)
        ORDER BY CASE WHEN realm_id = '{realm_id}' THEN 0 ELSE 1 END, version_no DESC
        LIMIT 1;

        IF v_version_id IS NOT NULL THEN
            INSERT INTO user_agreement_consent (
                id, user_id, realm_id, agreement_type, consented_version_id
            ) VALUES (
                uuidv7(), v_user_id, '{realm_id}', v_agreement_type, v_version_id
            )
            ON CONFLICT (user_id, agreement_type)
            DO UPDATE SET
                realm_id = EXCLUDED.realm_id,
                consented_version_id = EXCLUDED.consented_version_id,
                consented_at = NOW();
        END IF;
    END LOOP;
END $$;
"""
    _sql_exec(sql)
    _info(logger, f"[OK] Legal consent ready for {email} in {realm_id}")


def _ensure_realm001_user_subscription_permissions(logger: "Logger | None") -> None:
    """Ensure the realm-001 demo user role can view subscription history dependencies."""
    sql = f"""
DO $$
DECLARE
    v_user_role_id UUID;
    v_permission_id UUID;
    v_permission RECORD;
BEGIN
    SELECT id INTO v_user_role_id
    FROM roles
    WHERE realm_id = '{POINTS_REALM_ID}' AND name = 'user'
    LIMIT 1;

    IF v_user_role_id IS NULL THEN
        RAISE EXCEPTION 'Could not find realm-001 user role';
    END IF;

    FOR v_permission IN
        SELECT *
        FROM (VALUES
            ('clients.view', 'clients', 'view', 'View client applications for subscription history'),
            ('billing.view', 'billing', 'view', 'View own billing subscription history')
        ) AS p(name, resource, action, description)
    LOOP
        INSERT INTO permissions (
            id, name, description, realm_id, resource, action, is_builtin
        ) VALUES (
            uuidv7(),
            v_permission.name,
            v_permission.description,
            '{POINTS_REALM_ID}',
            v_permission.resource,
            v_permission.action,
            TRUE
        )
        ON CONFLICT (name, realm_id) DO UPDATE
            SET description = EXCLUDED.description,
                resource = EXCLUDED.resource,
                action = EXCLUDED.action,
                is_builtin = TRUE
        RETURNING id INTO v_permission_id;

        INSERT INTO role_permissions (id, role_id, permission_id)
        VALUES (uuidv7(), v_user_role_id, v_permission_id)
        ON CONFLICT (role_id, permission_id) DO NOTHING;

        INSERT INTO role_policies (id, role_id, realm_id, resource, action, effect)
        VALUES (
            uuidv7(),
            v_user_role_id,
            '{POINTS_REALM_ID}',
            v_permission.resource,
            v_permission.action,
            TRUE
        )
        ON CONFLICT (role_id, resource, action) DO UPDATE
            SET effect = TRUE,
                updated_at = NOW();
    END LOOP;
END $$;
"""
    _sql_exec(sql)
    _delete_redis_keys(
        [
            f"user_roles:{POINTS_REALM_ID}:*",
            f"role_policies:{POINTS_REALM_ID}:*",
            f"perm:{POINTS_REALM_ID}:*",
        ],
        logger,
    )
    _info(logger, "[OK] User subscription history permissions ready for realm-001")


def _seed_points_data(user_id: str, logger: "Logger | None") -> None:
    _info(logger, "Resetting points demo data for user@realm-001.com...")
    bucket_id = _default_bucket_id(POINTS_REALM_ID)
    sql = f"""
DO $$
DECLARE
    v_user_id UUID := '{user_id}'::uuid;
    v_bucket_id UUID := '{bucket_id}'::uuid;
    v_client_app_id UUID;
    v_wallet_id UUID := uuidv7();
    v_subscription_ledger_id UUID := uuidv7();
    v_topup_ledger_id UUID := uuidv7();
    v_pregrant_ledger_id UUID := uuidv7();
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
    DELETE FROM points_wallets WHERE user_id = v_user_id;

    INSERT INTO points_wallets (
        id,
        user_id,
        realm_id,
        bucket_id,
        total_topup_granted,
        total_subscription_granted,
        total_recharged,
        total_consumed,
        status,
        created_at,
        updated_at
    ) VALUES (
        v_wallet_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        v_bucket_id,
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
        bucket_id,
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
        v_bucket_id,
        'subscription_credit',
        'subscription_initial',
        'demo-subscription-202603',
        2000,
        100,
        0,
        NOW() + INTERVAL '365 days',
        'active',
        TIMESTAMPTZ '2026-03-10 09:00:00+00',
        TIMESTAMPTZ '2026-03-22 16:00:00+00'
    );

    INSERT INTO points_credit_ledger (
        id,
        user_id,
        realm_id,
        bucket_id,
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
        v_bucket_id,
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

    -- Pre-granted credit that is NOT yet effective: effective_at is far in the
    -- future, so the derived-balance predicate (effective_at IS NULL OR
    -- effective_at <= NOW()) excludes it from available/consumable/visible
    -- balance until the effective moment. Used by future-effective E2E tests.
    INSERT INTO points_credit_ledger (
        id,
        user_id,
        realm_id,
        bucket_id,
        credit_type,
        source_type,
        source_id,
        granted_amount,
        used_amount,
        revoked_amount,
        expires_at,
        effective_at,
        status,
        created_at,
        updated_at
    ) VALUES (
        v_pregrant_ledger_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        v_bucket_id,
        'granted_credit',
        'system_grant',
        'demo-pregrant-future-effective',
        1000,
        0,
        0,
        NULL,
        NOW() + INTERVAL '365 days',
        'active',
        TIMESTAMPTZ '2026-03-12 10:00:00+00',
        TIMESTAMPTZ '2026-03-12 10:00:00+00'
    );

    INSERT INTO points_transactions (
        id,
        wallet_id,
        user_id,
        realm_id,
        bucket_id,
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
        v_wallet_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        v_bucket_id,
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
        v_wallet_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        v_bucket_id,
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
        v_wallet_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        v_bucket_id,
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
        v_wallet_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        v_bucket_id,
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
        wallet_id,
        user_id,
        realm_id,
        bucket_id,
        allocated_amount,
        ledger_remaining_after,
        created_at
    ) VALUES
    (
        uuidv7(),
        v_tx_consume_1,
        v_subscription_ledger_id,
        v_wallet_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        v_bucket_id,
        40,
        1960,
        TIMESTAMPTZ '2026-03-21 15:00:00+00'
    ),
    (
        uuidv7(),
        v_tx_consume_2,
        v_subscription_ledger_id,
        v_wallet_id,
        v_user_id,
        '{POINTS_REALM_ID}',
        v_bucket_id,
        60,
        1900,
        TIMESTAMPTZ '2026-03-22 16:00:00+00'
    );
END $$;
"""
    _sql_exec(sql)


def _ensure_realm001_subscription_data(user_id: str, logger: "Logger | None") -> None:
    """Ensure subscription data exists for realm-001 (subscription timeline tests).

    Uses the post-entitlement-migration schema: no products/subscription_plan tables.
    The subscription table now uses entitlement_key instead of plan_id/tier/billing_period.
    """
    bucket_id = _default_bucket_id(POINTS_REALM_ID)
    sql = f"""
DO $$
DECLARE
    v_user_id UUID := '{user_id}'::uuid;
    v_client_app_id UUID;
    v_subscription_id UUID;
    v_test_timestamp TIMESTAMPTZ := TIMESTAMPTZ '2026-03-24 12:00:00+00';
BEGIN
    -- Get existing client app
    SELECT id INTO v_client_app_id
    FROM client_app
    WHERE realm_id = '{POINTS_REALM_ID}' AND client_id = '{POINTS_CLIENT_APP_ID}'
    LIMIT 1;

    IF v_client_app_id IS NULL THEN
        RAISE EXCEPTION 'Client app {POINTS_CLIENT_APP_ID} not found in {POINTS_REALM_ID}';
    END IF;

    -- Ensure entitlement mapping exists for this realm (provider-sourced catalog)
    -- external_price_id added; ON CONFLICT targets the new 4-column unique key
    -- `uq_pem_realm_provider_product_price` (the old 3-column clause throws
    -- `no unique or exclusion constraint matching`). This row is a single-price
    -- subscription mapping; NULLS NOT DISTINCT lets a NULL external_price_id
    -- still match on re-seed.
    INSERT INTO provider_entitlement_mappings (
        id, realm_id, payment_provider, external_product_id, external_price_id, bucket_id,
        entitlement_key, billing_type, enabled
    ) VALUES (
        uuidv7(),
        '{POINTS_REALM_ID}',
        'stripe',
        'realm001-product-subscription',
        NULL,
        '{bucket_id}',
        'professional',
        'recurring',
        TRUE
    )
    ON CONFLICT (realm_id, payment_provider, external_product_id, external_price_id) DO UPDATE
        SET bucket_id = EXCLUDED.bucket_id,
            entitlement_key = EXCLUDED.entitlement_key,
            billing_type = EXCLUDED.billing_type,
            enabled = TRUE;

    -- Create subscription
    DELETE FROM subscription WHERE client_app_id = v_client_app_id;

    INSERT INTO subscription (
        id, realm_id, user_id, external_subscription_id, external_product_id,
        payment_provider, status, entitlement_key,
        current_period_start, current_period_end, client_app_id, bucket_id
    ) VALUES (
        uuidv7(),
        '{POINTS_REALM_ID}',
        v_user_id,
        'demo-subscription-202603',
        'prod_realm001_' || uuidv7(),
        'stripe',
        'active',
        'professional',
        v_test_timestamp - INTERVAL '30 days',
        v_test_timestamp + INTERVAL '30 days',
        v_client_app_id,
        '{bucket_id}'::uuid
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
        'webhook',
        NULL,
        NULL,
        '{{"status": "active", "entitlement_key": "professional", "cancel_at_period_end": false}}'::jsonb,
        '{POINTS_REALM_ID}'
    ),
    (
        uuidv7()::text,
        v_subscription_id,
        'upgraded',
        v_test_timestamp - INTERVAL '15 days',
        'webhook',
        '{{"changed_fields": ["entitlement_key"], "previous_entitlement_key": "starter", "new_entitlement_key": "professional"}}'::jsonb,
        '{{"status": "active", "entitlement_key": "starter", "cancel_at_period_end": false}}'::jsonb,
        '{{"status": "active", "entitlement_key": "professional", "cancel_at_period_end": false}}'::jsonb,
        '{POINTS_REALM_ID}'
    ),
    (
        uuidv7()::text,
        v_subscription_id,
        'renewed',
        v_test_timestamp - INTERVAL '5 days',
        'system',
        NULL,
        '{{"status": "active", "entitlement_key": "professional", "cancel_at_period_end": false}}'::jsonb,
        '{{"status": "active", "entitlement_key": "professional", "cancel_at_period_end": false}}'::jsonb,
        '{POINTS_REALM_ID}'
    );

    RAISE NOTICE 'Subscription data created for {POINTS_REALM_ID}';
END $$;
"""
    _sql_exec(sql)
    _info(logger, "[OK] Subscription data ready for realm-001")


def _load_demo_env() -> dict[str, str]:
    """Parse KEY=VALUE lines from demo/.env.demo (no interpolation).

    Used to inject real provider credentials into the seeded realm_config so
    live E2E tests can reach the real Stripe/Creem APIs. Returns an empty dict
    when the file is absent; callers fall back to placeholder values.
    """
    env_path = Path(__file__).resolve().parent.parent.parent / "demo" / ".env.demo"
    env: dict[str, str] = {}
    if not env_path.exists():
        return env
    for line in env_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, _, value = line.partition("=")
        env[key.strip()] = value.strip()
    return env


def _sql_escape(value: str) -> str:
    """Escape a string literal for safe use inside single-quoted SQL."""
    return value.replace("'", "''")


def _ensure_payment_provider_config(logger: "Logger | None") -> None:
    """Ensure payment provider credentials (realm_config) for demo.

    Only realm_config rows are seeded. Entitlement mappings are NOT seeded as
    placeholders — the admin triggers a provider sync from the UI to pull the
    real Stripe/Creem catalog. This avoids placeholder price ids (e.g.
    ``price_stripe_admin_onetime_1000``) appearing as unresolved rows.
    """
    _info(logger, "Ensuring payment provider config demo data...")
    # Live provider credentials: prefer real keys from demo/.env.demo so live
    # E2E tests hit the real Stripe/Creem APIs; fall back to placeholder values
    # when .env.demo is absent or a key is empty (default demo behaviour).
    env = _load_demo_env()
    stripe_api_key = _sql_escape(env.get("STRIPE_SECRET_KEY") or "sk_test_demo_points_package")
    stripe_publishable_key = _sql_escape(env.get("STRIPE_PUBLISHABLE_KEY") or "pk_test_demo_points_package")
    stripe_webhook_secret = _sql_escape(env.get("STRIPE_WEBHOOK_SECRET") or "whsec_demo_points_package")
    creem_api_key = _sql_escape(env.get("CREEM_API_KEY") or "creem_test_demo_points_package")
    creem_webhook_secret = _sql_escape(env.get("CREEM_WEBHOOK_SECRET") or "creem_whsec_demo_points_package")
    sql = f"""
    INSERT INTO realm_config (
        realm_id, config_type, config_key, config_value, is_secret, enabled, metadata
    ) VALUES
        ('{POINTS_REALM_ID}', 'stripe', 'api_key', '{stripe_api_key}', true, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'stripe', 'publishable_key', '{stripe_publishable_key}', false, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'stripe', 'webhook_secret', '{stripe_webhook_secret}', true, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'stripe', 'timeout', '30', false, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'creem', 'api_key', '{creem_api_key}', true, true, '{{}}'::jsonb),
        ('{POINTS_REALM_ID}', 'creem', 'webhook_secret', '{creem_webhook_secret}', true, true, '{{}}'::jsonb),
        ('{ADMIN_REALM}', 'stripe', 'api_key', '{stripe_api_key}', true, true, '{{}}'::jsonb),
        ('{ADMIN_REALM}', 'stripe', 'publishable_key', '{stripe_publishable_key}', false, true, '{{}}'::jsonb),
        ('{ADMIN_REALM}', 'stripe', 'webhook_secret', '{stripe_webhook_secret}', true, true, '{{}}'::jsonb),
        ('{ADMIN_REALM}', 'stripe', 'timeout', '30', false, true, '{{}}'::jsonb)
    ON CONFLICT (realm_id, config_type, config_key) DO UPDATE
        SET config_value = EXCLUDED.config_value,
            is_secret = EXCLUDED.is_secret,
            enabled = true,
            metadata = EXCLUDED.metadata,
            updated_at = now();
    """
    _sql_exec(sql)
    _info(logger, "[OK] Payment provider config demo data ready")


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


def _ensure_admin_realm_audit_events(logger: "Logger | None") -> None:
    """Seed realm_management audit events for the admin realm (created during bootstrap, no audit events)."""
    admin_user_id = _sql_scalar(
        f"SELECT id::text FROM account WHERE realm_id = '{ADMIN_REALM}' AND email = '{ADMIN_EMAIL}' LIMIT 1;"
    )
    if not admin_user_id:
        _info(logger, "Admin user not found, skipping audit seed")
        return

    existing = _sql_scalar(
        f"SELECT COUNT(*)::text FROM audit_events WHERE realm_id = '{ADMIN_REALM}' AND category = 'realm_management';"
    )
    if existing and existing != "0":
        _info(logger, "Admin realm audit events already exist")
        return

    _info(logger, "Inserting admin realm audit seed events...")
    sql = f"""
    DO $$
    DECLARE
        v_admin_id UUID := '{admin_user_id}'::uuid;
    BEGIN
        INSERT INTO audit_events (id, realm_id, category, action, actor_id, actor_type, actor_name, target_type, target_id, target_name, result, details, created_at)
        VALUES
        (uuidv7(), '{ADMIN_REALM}', 'realm_management', 'realm.create', v_admin_id::text, 'admin', '{ADMIN_EMAIL}', 'realm', '{ADMIN_REALM}', 'Admin', 'success', '{{\"status\": \"created\"}}', NOW() - INTERVAL '1 hour'),
        (uuidv7(), '{ADMIN_REALM}', 'realm_management', 'realm.rbac_init', v_admin_id::text, 'admin', '{ADMIN_EMAIL}', 'realm', '{ADMIN_REALM}', 'Admin', 'success', '{{\"roles\": [\"admin\", \"user\"]}}', NOW() - INTERVAL '59 minutes');
    END $$;
    """
    _sql_exec(sql)
    _info(logger, "[OK] Admin realm audit seed events ready")


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
    # Post-entitlement-migration: no products/subscription_plan tables.
    # The subscription table uses entitlement_key instead of plan_id/tier/billing_period.
    bucket_id = _default_bucket_id(ADMIN_REALM)
    sql = f"""
DO $$
DECLARE
    v_user_id UUID := '{test_user_id}'::uuid;
    v_client_app_id UUID;
    v_subscription_id UUID;
    v_test_timestamp TIMESTAMPTZ := TIMESTAMPTZ '2026-03-24 12:00:00+00';
BEGIN
    -- Get or create client app (use admin-web-console)
    SELECT id INTO v_client_app_id
    FROM client_app
    WHERE realm_id = '{ADMIN_REALM}' AND client_id = '{SUBSCRIPTION_TEST_CLIENT_ID}'
    LIMIT 1;

    IF v_client_app_id IS NULL THEN
        RAISE EXCEPTION 'Client app {SUBSCRIPTION_TEST_CLIENT_ID} not found in {ADMIN_REALM}';
    END IF;

    -- Ensure entitlement mapping exists for admin realm
    -- external_price_id added; ON CONFLICT targets the new 4-column unique key.
    -- Single-price subscription row; NULL external_price_id matches on re-seed
    -- via NULLS NOT DISTINCT.
    INSERT INTO provider_entitlement_mappings (
        id, realm_id, payment_provider, external_product_id, external_price_id, bucket_id,
        entitlement_key, billing_type, enabled
    ) VALUES (
        uuidv7(),
        '{ADMIN_REALM}',
        'stripe',
        'test-product-subscription',
        NULL,
        '{bucket_id}',
        'professional',
        'recurring',
        TRUE
    )
    ON CONFLICT (realm_id, payment_provider, external_product_id, external_price_id) DO UPDATE
        SET bucket_id = EXCLUDED.bucket_id,
            entitlement_key = EXCLUDED.entitlement_key,
            billing_type = EXCLUDED.billing_type,
            enabled = TRUE;

    -- Create subscription
    DELETE FROM subscription WHERE client_app_id = v_client_app_id;

    INSERT INTO subscription (
        id, realm_id, user_id, external_subscription_id, external_product_id,
        payment_provider, status, entitlement_key,
        current_period_start, current_period_end, client_app_id, bucket_id
    ) VALUES (
        uuidv7(),
        '{ADMIN_REALM}',
        v_user_id,
        'sub_demo_' || uuidv7(),
        'prod_demo_' || uuidv7(),
        'stripe',
        'active',
        'professional',
        v_test_timestamp - INTERVAL '30 days',
        v_test_timestamp + INTERVAL '30 days',
        v_client_app_id,
        '{bucket_id}'::uuid
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
        'webhook',
        NULL,
        NULL,
        '{{"status": "active", "entitlement_key": "professional", "cancel_at_period_end": false}}'::jsonb,
        '{ADMIN_REALM}'
    ),
    (
        uuidv7()::text,
        v_subscription_id,
        'upgraded',
        v_test_timestamp - INTERVAL '15 days',
        'webhook',
        '{{"changed_fields": ["entitlement_key"], "previous_entitlement_key": "starter", "new_entitlement_key": "professional"}}'::jsonb,
        '{{"status": "active", "entitlement_key": "starter", "cancel_at_period_end": false}}'::jsonb,
        '{{"status": "active", "entitlement_key": "professional", "cancel_at_period_end": false}}'::jsonb,
        '{ADMIN_REALM}'
    ),
    (
        uuidv7()::text,
        v_subscription_id,
        'renewed',
        v_test_timestamp - INTERVAL '5 days',
        'system',
        NULL,
        '{{"status": "active", "entitlement_key": "professional", "cancel_at_period_end": false}}'::jsonb,
        '{{"status": "active", "entitlement_key": "professional", "cancel_at_period_end": false}}'::jsonb,
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


def _delete_redis_keys(patterns: list[str], logger: "Logger | None") -> None:
    for pattern in patterns:
        code, output = docker.exec_check("cas-demo-redis", ["redis-cli", "--scan", "--pattern", pattern])
        if code != 0:
            _info(logger, f"Skipping Redis cache cleanup for {pattern}: {output}")
            continue

        keys = [line.strip() for line in output.splitlines() if line.strip()]
        if not keys:
            continue

        code, output = docker.exec_check("cas-demo-redis", ["redis-cli", "DEL", *keys])
        if code != 0:
            _info(logger, f"Skipping Redis cache cleanup for {pattern}: {output}")


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


# Credit Bucket directory seed constants.
#
# These keys mirror `demo/e2e/helpers/bucket-seed-ids.ts::CREDIT_BUCKET_KEYS`.
# Keep both in sync: a key change here must be reflected there (and vice versa)
# so authoring items reference stable constants, not magic strings.
#
# Per-realm directory (exactly one registration pool per realm, enforced by
# `uq_credit_buckets_registration_pool`):
#   - PRIMARY_POOL: receives_registration_credits=true; all seeded points
#     wallet/ledger/txn rows, assigned one-time mappings, and the seeded
#     subscription reference this bucket. Registration / free-periodic grants
#     target this bucket.
#   - SECONDARY_POOL: enabled, NOT a registration pool; covers POINTS_CLIENT_APP_ID
#     so cross-bucket demos can exercise cross-bucket assertions. Holds no seeded
#     balance by default.
CREDIT_BUCKET_KEY_PRIMARY = "primary-pool"
CREDIT_BUCKET_KEY_SECONDARY = "promo-pool"
CREDIT_BUCKET_NAME_PRIMARY = "Primary Pool"
CREDIT_BUCKET_NAME_SECONDARY = "Promo Pool"
CREDIT_BUCKET_DESC_PRIMARY = "Demo primary credit pool (registration receiver)"
CREDIT_BUCKET_DESC_SECONDARY = "Demo secondary credit pool (cross-bucket coverage)"

# Legacy single-bucket key from earlier seed revisions. Pre-bucket-directory
# seeds created one row with bucket_key='default'; this set tracks those keys
# so `_ensure_credit_buckets` can migrate dependent rows off them and drop the
# leftover bucket (prevents a 2nd registration pool from violating the partial
# unique index `uq_credit_buckets_registration_pool`).
_LEGACY_DEFAULT_BUCKET_KEYS = ("default",)


def _ensure_credit_buckets(logger: "Logger | None") -> None:
    """Ensure the Credit Bucket directory exists for admin and realm-001.

    Per realm creates exactly two enabled buckets:
      - `primary-pool` (receives_registration_credits=true) — the single
        registration pool per realm. All seeded points/mapping/payment rows
        reference this bucket via `bucket_id`.
      - `promo-pool` (receives_registration_credits=false) — a secondary pool
        covering POINTS_CLIENT_APP_ID so cross-bucket demos have a
        covered-but-empty pool to exercise.

    Both buckets bind the existing POINTS_CLIENT_APP_ID client app via
    `credit_bucket_client_apps` (`ON CONFLICT DO NOTHING`). All existing
    one-time `provider_entitlement_mappings` rows are assigned to the primary
    pool (`bucket_id` is NOT NULL; there is no unassigned path).

    Idempotent via `INSERT ... ON CONFLICT (realm_id, bucket_key) DO UPDATE`.
    Bucket ids are resolved at consumption time via `_bucket_id(realm_id, key)`,
    not hardcoded UUIDs.

    LOUD NOTE — migration from legacy `default` bucket: prior seed revisions
    created a single `bucket_key='default'` row. Re-seeding an existing demo
    DB would otherwise leave that row alongside the new `primary-pool`, both
    flagged as registration pool, violating `uq_credit_buckets_registration_pool`.
    This function therefore (a) migrates any dependent rows off legacy buckets
    onto the primary pool, (b) deletes the legacy bucket rows, THEN (c) inserts
    the directory. On a fresh DB the migration is a no-op.
    """
    _info(logger, "Ensuring Credit Bucket directory for admin and realm-001...")
    _migrate_legacy_default_buckets(logger)
    for realm_id in (POINTS_REALM_ID, ADMIN_REALM):
        _ensure_realm_bucket_directory(realm_id, logger)
    _info(logger, "[OK] Credit Bucket directory ready for admin and realm-001")


def _migrate_legacy_default_buckets(logger: "Logger | None") -> None:
    """Migrate dependent rows off legacy `default` buckets onto `primary-pool`.

    Runs BEFORE the directory is (re)created. On a fresh DB this is a no-op
    (no legacy buckets exist). On a DB seeded by an earlier revision it:
      1. Resolves the legacy bucket id per realm (if any).
      2. Resolves the primary-pool bucket id (creating a placeholder if the
         directory has not yet been seeded this run — the placeholder is then
         upserted by `_ensure_realm_bucket_directory`).
      3. Re-points every `bucket_id`-bearing row that referenced the legacy
         bucket to the primary pool.
      4. Removes the legacy bucket's coverage rows and finally the bucket row.

    The registration pool partial unique index (`uq_credit_buckets_registration_pool`)
    is what makes the delete-before-reinsert ordering matter: if we left the
    legacy `default` row (registration pool) in place alongside the new
    `primary-pool` (also registration pool), the index would reject the insert.
    """
    for realm_id in (POINTS_REALM_ID, ADMIN_REALM):
        for legacy_key in _LEGACY_DEFAULT_BUCKET_KEYS:
            legacy_id = _sql_scalar(
                "SELECT id::text FROM credit_buckets "
                f"WHERE realm_id = '{realm_id}' AND bucket_key = '{legacy_key}' LIMIT 1;"
            )
            if not legacy_id:
                continue

            _info(
                logger,
                f"Migrating legacy bucket '{legacy_key}' -> '{CREDIT_BUCKET_KEY_PRIMARY}' "
                f"in realm {realm_id}",
            )

            primary_id = _sql_scalar(
                "SELECT id::text FROM credit_buckets "
                f"WHERE realm_id = '{realm_id}' AND bucket_key = '{CREDIT_BUCKET_KEY_PRIMARY}' LIMIT 1;"
            )
            if not primary_id:
                # Directory not seeded yet this revision: create a placeholder
                # primary pool so dependent rows can be repointed. The directory
                # upsert below will update its mutable columns deterministically.
                _sql_exec(
                    f"""
                    INSERT INTO credit_buckets (
                        id, realm_id, bucket_key, name, description,
                        display_order, receives_registration_credits, enabled, created_at, updated_at
                    ) VALUES (
                        uuidv7(),
                        '{realm_id}',
                        '{CREDIT_BUCKET_KEY_PRIMARY}',
                        '{CREDIT_BUCKET_NAME_PRIMARY}',
                        '{CREDIT_BUCKET_DESC_PRIMARY}',
                        0,
                        TRUE,
                        TRUE,
                        NOW(),
                        NOW()
                    )
                    ON CONFLICT (realm_id, bucket_key) DO NOTHING;
                    """
                )
                primary_id = _sql_scalar(
                    "SELECT id::text FROM credit_buckets "
                    f"WHERE realm_id = '{realm_id}' AND bucket_key = '{CREDIT_BUCKET_KEY_PRIMARY}' LIMIT 1;"
                )
                if not primary_id:
                    raise SeedError(
                        f"Could not establish primary pool placeholder in realm {realm_id}"
                    )

            # Re-point every bucket_id-bearing dependent table. All of these
            # columns are NOT NULL on the points side and nullable on the
            # routing side; UPDATE is safe either way.
            _sql_exec(
                f"""
                UPDATE points_wallets SET bucket_id = '{primary_id}'::uuid
                  WHERE bucket_id = '{legacy_id}'::uuid;
                UPDATE points_credit_ledger SET bucket_id = '{primary_id}'::uuid
                  WHERE bucket_id = '{legacy_id}'::uuid;
                UPDATE points_transactions SET bucket_id = '{primary_id}'::uuid
                  WHERE bucket_id = '{legacy_id}'::uuid;
                UPDATE points_consumption_allocations SET bucket_id = '{primary_id}'::uuid
                  WHERE bucket_id = '{legacy_id}'::uuid;
                UPDATE provider_entitlement_mappings SET bucket_id = '{primary_id}'::uuid
                  WHERE bucket_id = '{legacy_id}'::uuid;
                UPDATE payment_attempts SET bucket_id = '{primary_id}'::uuid
                  WHERE bucket_id = '{legacy_id}'::uuid;
                """
            )

            # subscription.bucket_id is nullable; only repoint where the legacy
            # bucket was the recorded pool to keep lifecycle routing consistent.
            _sql_exec(
                f"""
                UPDATE subscription SET bucket_id = '{primary_id}'::uuid
                  WHERE bucket_id = '{legacy_id}'::uuid;
                """
            )

            # Drop the legacy bucket's coverage bindings, then the bucket row.
            _sql_exec(
                f"""
                DELETE FROM credit_bucket_client_apps WHERE bucket_id = '{legacy_id}'::uuid;
                DELETE FROM credit_buckets WHERE id = '{legacy_id}'::uuid;
                """
            )
            _info(
                logger,
                f"Migrated legacy bucket '{legacy_key}' off and removed it in realm {realm_id}",
            )


def _ensure_realm_bucket_directory(realm_id: str, logger: "Logger | None") -> None:
    """Create/refresh the per-realm Credit Bucket directory.

    Idempotent upsert of `primary-pool` (registration receiver) and `promo-pool`
    (secondary). Both are enabled. Only `primary-pool` carries
    `receives_registration_credits = true`; the partial unique index
    `uq_credit_buckets_registration_pool` enforces the per-realm-single invariant.
    """
    _info(logger, f"Upserting Credit Bucket directory for realm {realm_id}...")
    _sql_exec(
        f"""
        INSERT INTO credit_buckets (
            id, realm_id, bucket_key, name, description,
            display_order, receives_registration_credits, enabled, created_at, updated_at
        ) VALUES
        (
            uuidv7(),
            '{realm_id}',
            '{CREDIT_BUCKET_KEY_PRIMARY}',
            '{CREDIT_BUCKET_NAME_PRIMARY}',
            '{CREDIT_BUCKET_DESC_PRIMARY}',
            0,
            TRUE,
            TRUE,
            NOW(),
            NOW()
        ),
        (
            uuidv7(),
            '{realm_id}',
            '{CREDIT_BUCKET_KEY_SECONDARY}',
            '{CREDIT_BUCKET_NAME_SECONDARY}',
            '{CREDIT_BUCKET_DESC_SECONDARY}',
            1,
            FALSE,
            TRUE,
            NOW(),
            NOW()
        )
        ON CONFLICT (realm_id, bucket_key) DO UPDATE
            SET name = EXCLUDED.name,
                description = EXCLUDED.description,
                display_order = EXCLUDED.display_order,
                receives_registration_credits = EXCLUDED.receives_registration_credits,
                enabled = EXCLUDED.enabled,
                updated_at = NOW();
        """
    )

    # Bind both buckets to the points demo client app (coverage set).
    # Each realm's points demo app is resolved by (realm_id, client_id); if the
    # client app does not yet exist in this realm at seed time, coverage binding
    # is skipped silently — `_seed_points_data` will (re)create the client app
    # afterwards, and a later re-seed run will bind it. ON CONFLICT DO NOTHING
    # makes this safe to repeat.
    client_app_id = _sql_scalar(
        "SELECT id::text FROM client_app "
        f"WHERE realm_id = '{realm_id}' AND client_id = '{POINTS_CLIENT_APP_ID}' LIMIT 1;"
    )
    if client_app_id:
        primary_id = _bucket_id(realm_id, CREDIT_BUCKET_KEY_PRIMARY)
        secondary_id = _bucket_id(realm_id, CREDIT_BUCKET_KEY_SECONDARY)
        # realm_id is NOT NULL on credit_bucket_client_apps (schema baseline).
        # Include it explicitly so the binding row is writable on fresh DBs.
        _sql_exec(
            f"""
            INSERT INTO credit_bucket_client_apps (bucket_id, client_app_id, realm_id) VALUES
                ('{primary_id}'::uuid, '{client_app_id}'::uuid, '{realm_id}'),
                ('{secondary_id}'::uuid, '{client_app_id}'::uuid, '{realm_id}')
            ON CONFLICT DO NOTHING;
            """
        )
    else:
        _info(
            logger,
            f"Client app {POINTS_CLIENT_APP_ID} not yet seeded in realm {realm_id}; "
            "coverage binding will complete on a subsequent re-seed",
        )


def _bucket_id(realm_id: str, bucket_key: str) -> str:
    """Return the id of a seeded Credit Bucket by (realm_id, bucket_key).

    Deterministic and idempotent: callers must resolve bucket ids via this
    helper, not via hardcoded UUIDs. Raises SeedError if the bucket is missing
    (i.e. `_ensure_credit_buckets` was not run first).
    """
    bucket_id = _sql_scalar(
        f"SELECT id::text FROM credit_buckets "
        f"WHERE realm_id = '{realm_id}' AND bucket_key = '{bucket_key}' LIMIT 1;"
    )
    if not bucket_id:
        raise SeedError(
            f"Credit bucket '{bucket_key}' not found in realm {realm_id}; "
            "run _ensure_credit_buckets first"
        )
    return bucket_id


def _default_bucket_id(realm_id: str) -> str:
    """Return the primary registration-pool bucket id for a realm.

    Kept as a thin alias over `_bucket_id(realm_id, CREDIT_BUCKET_KEY_PRIMARY)`
    so existing seed call sites (`_seed_points_data`,
    `_ensure_subscription_history_demo_data`, etc.) keep compiling after the
    directory rename `default` -> `primary-pool`. The name is retained for
    minimal blast radius; the bucket it resolves is the registration pool.
    """
    return _bucket_id(realm_id, CREDIT_BUCKET_KEY_PRIMARY)


def _ensure_invoice_seller_config(logger: "Logger | None") -> None:
    """Ensure invoice_seller_config exists for both admin and realm-001 so invoice pages are visible."""
    for realm_id, label in [(POINTS_REALM_ID, "realm-001"), (ADMIN_REALM, "admin")]:
        sql = f"""
        INSERT INTO invoice_seller_config (realm_id, seller_name, seller_address, seller_email, seller_phone, seller_tax_id, default_payment_terms)
        VALUES ('{realm_id}', 'Herald Demo Corp', '123 Demo Street, Demo City', 'billing@{label}.demo', '+1-000-000-0000', 'DEMO-TAX-{label}', 'Net 30')
        ON CONFLICT (realm_id) DO UPDATE
            SET seller_name = EXCLUDED.seller_name,
                seller_address = EXCLUDED.seller_address,
                seller_email = EXCLUDED.seller_email,
                seller_phone = EXCLUDED.seller_phone,
                seller_tax_id = EXCLUDED.seller_tax_id,
                default_payment_terms = EXCLUDED.default_payment_terms;
        """
        _sql_exec(sql)
    _info(logger, "[OK] Invoice seller config ready for admin and realm-001")

