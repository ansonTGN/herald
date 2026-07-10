/**
 * Realm Admin 演示测试 - Custom-domain authorize (ask) 门禁（Caddy On-Demand TLS）
 *
 * 用户故事（DRAFT，发布前不得改写为 docs/user-stories/，路径保持原样）：
 * - [US-CD-005] 场景1（未注册自定义域名不提供 Realm 登录页 / 证书授权）
 *   DRAFT 来源（原样引用）：`.ai/user-stories/core/realm-custom-domain.md`
 *
 * ⚠️ 范围声明（OUT OF SCOPE / DEFERRED）：
 * 本测试只覆盖 Caddy On-Demand TLS 的 **authorize (ask) 门禁**
 * （`GET /api/internal/custom-domain/authorize`）—— 一个后端纯 API 的证书滥用
 * 门禁。它断言门禁的三种文档化结果：
 *   - 已发布且 enabled=true 的 host → 200 `{"authorized": true}`（不含 realm 信息）
 *   - 未注册 host → 404（Caddy 拒绝签发）
 *   - 缺少/错误的 `X-Herald-Ask-Key` → 401
 *
 * US-CD-005 的 host→realm 路由场景（终端用户经自定义域名访问 realm 的 auth 流）
 * 已于 2026-07-09 回退（DEFERRED）—— 当前没有任何 host-based 公共入口可断言，
 * 只有 ask 门禁（证书滥用门禁）可 demo。本测试不覆盖 US-CD-005 场景2
 * （未生效 CNAME 的状态可见）—— 那属于 config-admin UI（DE-D01）的职责。
 *
 * 边界：纯 API 测试 —— 无 UI、无 settings tab、无 host→realm 路由。
 * 失败归因：本测试失败意味着 ask 门禁逻辑或其配置接线错误，
 * 与生命周期 UI（DE-D01）相互独立。
 *
 * 断言策略：
 * - 关键断言落在持久后端状态（HTTP 状态码 + 响应体形状），而非 sonner/toast 等
 *   自动消失提示。
 * - 200 响应体必须 **只** 包含 `{"authorized": true}`，不含任何 realm 标识
 *   （`realmId` / `realm_id`）—— 证书滥用门禁不得泄漏 realm 信息
 *   （`custom_domain_config.rs:478-484`）。
 *
 * Setup 策略（publish-to-authorize）：
 * authorize 门禁读取 `custom_domain_mapping` 表过滤 `enabled = true` 的行。要得到
 * 一个「已授权」行，必须先 **发布** 一个 custom-domain config（publish 写入
 * `enabled=true, cname_verified=false, tls_ready=false` 的 mapping 行）。所以
 * setup 通过 API 在 realm `admin` 上发布一个专用 hostname
 * （PUT draft → POST publish，因为 publish 发布的是已保存的草稿，不是请求体），
 * 跑完门禁断言后再清理。
 *
 * ⚠️ LOAD-BEARING（来自 DE-D01 findings）：custom-domain publish 发布的是 **已保存
 * 的草稿**，不是请求体值。后端 `handle_publish` 从 `DRAFT_KEY` 读取，无草稿则
 * 返回 400。所以 setup 必须：
 *   1. PUT `/api/realms/admin/config/custom-domain/draft` 携带
 *      `{"hostname":"ask-gate-authorize.demo.test"}`
 *   2. POST `/api/realms/admin/config/custom-domain/publish`（无 body）
 * custom-domain 不能「发布空」（400），所以 teardown 必须 restore（恢复上一版），
 * 而非 publish-empty。
 *
 * Setup 认证：config 端点需要 `settings.manage`（realm admin）。本测试通过
 * `loginAsAdmin` 在浏览器上下文中 seed admin session cookie，再用 `page.request`
 * 调用需要 admin 认证的 config 端点（`page.request` 共享浏览器上下文的 cookie）。
 *
 * Teardown 策略（restore-balanced）：
 * 在 afterEach 中尽力 restore 专用 hostname 的发布（publish-empty 是 400，所以
 * 用 restore 端点回滚到上一版 baseline）。best-effort、有日志，不会硬失败测试运行。
 *
 * @see .ai/user-stories/core/realm-custom-domain.md （DRAFT 来源，路径保持原样）
 * @see backend/api/src/application/http/realm/custom_domain_config.rs （authorize handler 478-553）
 * @see backend/config/demo.toml [custom_domain].ask_key （shared secret 来源）
 */

import { test, expect, cleanupTestData } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { loginAsAdmin } from '../helpers/auth'
import { SettingsPage } from '../pages/settings-page'

/**
 * Base URL for the internal authorize gate.
 *
 * The authorize endpoint is a backend-only top-level route
 * (`/api/internal/custom-domain/authorize`, registered in `server/mod.rs`, NOT
 * under `/api/realms`) and is authenticated by a shared secret header, NOT a
 * session cookie. We call it directly against the backend to avoid coupling to
 * the frontend dev-server proxy.
 */
const API_BASE = process.env.API_BASE_URL || 'http://127.0.0.1:8080'

/**
 * Shared ask key that gates the authorize endpoint. MUST match the value in
 * `backend/config/demo.toml [custom_domain].ask_key` (set by DE-D01).
 *
 * The backend handler reads `X-Herald-Ask-Key` and compares it (trimmed) to the
 * configured `custom_domain_ask_key` (`custom_domain_config.rs:518-523`). A
 * missing or mismatched header yields 401 regardless of whether the host is
 * registered.
 */
const ASK_KEY = process.env.CUSTOM_DOMAIN_ASK_KEY || 'demo-custom-domain-ask-key'

/**
 * Frontend base URL (for browser login + config endpoints that share the
 * browser session cookie).
 */
const FRONTEND_BASE_URL = process.env.BASE_URL || 'http://localhost:3000'

/**
 * Dedicated test-only hostname published on realm `admin` for the authorize
 * 200 scenario. Distinct from DE-D01's hostnames to avoid cross-item state
 * pollution (both DE-D01 and DE-D02 publish on realm `admin`).
 */
const DEDICATED_HOSTNAME = 'ask-gate-authorize.demo.test'

/**
 * An unregistered host used for the 404 scenario. It must not collide with any
 * published mapping across realms; the `.test` TLD guarantees it never resolves
 * or gets published by another suite.
 */
const UNREGISTERED_HOST = 'evil-unregistered.test'

test.describe('[Realm Admin] Custom-domain authorize (ask) 门禁演示测试', () => {
  let testStartTime: number

  test.afterEach(async ({ page }) => {
    // Restore-balance the dedicated published hostname. publish-empty is a 400
    // (handle_publish rejects a draft without a hostname), so we use the
    // SettingsPage.resetCustomDomainConfig() helper (discards any dangling
    // draft) + a best-effort restore. All failures are caught + logged so
    // teardown never hard-fails the run.
    try {
      await loginAsAdmin(page, { realmId: 'admin', forceRelogin: true })
      const realmSettings = new SettingsPage(page, undefined, 'admin')
      await realmSettings.goto()
      await realmSettings.waitForReady()
      await realmSettings.resetCustomDomainConfig()
    } catch (error) {
      console.warn('[afterEach] restore-balance failed for realm "admin":', error)
    }

    // ⚠️ MANDATORY: 清理测试数据
    await cleanupTestData(page, 'admin', {
      keepUsers: ['admin@cas.com'],
      timestamp: testStartTime,
    })
  })

  // ==========================================================================
  // Test 1 — 未注册域名 → 404 (Caddy 拒绝签发)
  // 映射: [US-CD-005] 场景1（未注册自定义域名不提供证书授权）
  // DRAFT 来源: .ai/user-stories/core/realm-custom-domain.md
  // ==========================================================================

  test('未注册域名 → 404 (Caddy declines issuance)', async ({ page, demoLogger }) => {
    // [US-CD-005] 场景1 — DRAFT: .ai/user-stories/core/realm-custom-domain.md
    testStartTime = Date.now()

    await verifyTestEnvironment(page, {
      requiredRealms: ['admin'],
      requiredUsers: ['admin@cas.com'],
      skipRealmVerification: true,
      skipDatabaseCheck: false,
      skipRedisCheck: false,
    })

    await test.step('用正确 ask key 查询一个未在任何 realm 注册的 host', async () => {
      const resp = await page.request.get(
        `${API_BASE}/api/internal/custom-domain/authorize`,
        {
          params: { host: UNREGISTERED_HOST },
          headers: { 'X-Herald-Ask-Key': ASK_KEY },
        },
      )

      // 未注册 host → 404（Caddy 拒绝签发，US-CD-005 场景1）。
      expect(
        resp.status(),
        `未注册 host "${UNREGISTERED_HOST}" 必须返回 404 (got ${resp.status()})`,
      ).toBe(404)

      const body = await resp.json().catch(() => ({}))
      demoLogger.testCode.log(
        `未注册 host "${UNREGISTERED_HOST}" → 404 (body: ${JSON.stringify(body)})`,
      )

      // 404 是错误形状（ErrorResponse {code,message,error,details}），authorized
      // 字段不存在 / 不为 true —— 证书不会被签发。
      expect(
        body.authorized,
        '404 响应体不得含 authorized:true',
      ).not.toBe(true)
    })
  })

  // ==========================================================================
  // Test 2 — 已发布域名 → 200 {"authorized":true}（不含 realm 信息）
  // 映射: [US-CD-005] 场景1（已发布域名的反向：合法域名获授权）
  // DRAFT 来源: .ai/user-stories/core/realm-custom-domain.md
  // ==========================================================================

  test('已发布域名 → 200 {"authorized":true}', async ({ page, demoLogger }) => {
    // [US-CD-005] 场景1 — DRAFT: .ai/user-stories/core/realm-custom-domain.md
    testStartTime = Date.now()

    await verifyTestEnvironment(page, {
      requiredRealms: ['admin'],
      requiredUsers: ['admin@cas.com'],
      skipRealmVerification: true,
      skipDatabaseCheck: false,
      skipRedisCheck: false,
    })

    // ----------------------------------------------------------------------
    // Setup: 通过 API 在 realm admin 发布专用 hostname（PUT draft → POST publish）
    // publish 发布的是已保存的草稿，所以必须先 PUT draft。
    // ----------------------------------------------------------------------
    await test.step('Setup: loginAsAdmin + PUT draft + POST publish 专用 hostname', async () => {
      // loginAsAdmin seeds the admin session cookie into the browser context;
      // page.request.* shares that cookie so the config endpoints authenticate.
      await loginAsAdmin(page, { realmId: 'admin' })

      // 1) PUT draft —— 必须先存草稿，publish 才有内容可发布。
      const draftResp = await page.request.put(
        `${FRONTEND_BASE_URL}/api/realms/admin/config/custom-domain/draft`,
        { data: { hostname: DEDICATED_HOSTNAME } },
      )
      expect(
        draftResp.status(),
        `PUT draft for "${DEDICATED_HOSTNAME}" must succeed (got ${draftResp.status()})`,
      ).toBe(200)
      demoLogger.testCode.log(`Setup: PUT draft "${DEDICATED_HOSTNAME}" → ${draftResp.status()}`)

      // 2) POST publish —— 发布已保存的草稿，写入 enabled=true 的 mapping 行。
      const publishResp = await page.request.post(
        `${FRONTEND_BASE_URL}/api/realms/admin/config/custom-domain/publish`,
      )
      expect(
        publishResp.status(),
        `POST publish for "${DEDICATED_HOSTNAME}" must succeed (got ${publishResp.status()})`,
      ).toBe(200)
      demoLogger.testCode.log(`Setup: POST publish "${DEDICATED_HOSTNAME}" → ${publishResp.status()}`)
    })

    // ----------------------------------------------------------------------
    // Assertion: 用正确 ask key 查询刚发布的 host → 200 {"authorized":true}
    // ----------------------------------------------------------------------
    await test.step('GET authorize 已发布 host + 正确 key → 200 {"authorized":true}', async () => {
      const resp = await page.request.get(
        `${API_BASE}/api/internal/custom-domain/authorize`,
        {
          params: { host: DEDICATED_HOSTNAME },
          headers: { 'X-Herald-Ask-Key': ASK_KEY },
        },
      )

      expect(
        resp.status(),
        `已发布 host "${DEDICATED_HOSTNAME}" 必须返回 200 (got ${resp.status()})`,
      ).toBe(200)

      const json = await resp.json()

      // 200 响应体必须恰好是 {"authorized": true}（cert-abuse gate）。
      expect(
        json,
        `200 响应体必须等于 {authorized:true} (got ${JSON.stringify(json)})`,
      ).toEqual({ authorized: true })

      demoLogger.testCode.log(`已发布 host "${DEDICATED_HOSTNAME}" → 200 ${JSON.stringify(json)}`)
    })

    // ----------------------------------------------------------------------
    // Assertion: 200 响应不得泄漏 realm 信息（证书滥用门禁，design §4.2.2 ask）
    // custom_domain_config.rs:478-484 明确禁止 body 含 realm 标识。
    // ----------------------------------------------------------------------
    await test.step('断言 200 响应不含 realm 信息（cert-abuse gate 不得泄漏 realm）', async () => {
      const resp = await page.request.get(
        `${API_BASE}/api/internal/custom-domain/authorize`,
        {
          params: { host: DEDICATED_HOSTNAME },
          headers: { 'X-Herald-Ask-Key': ASK_KEY },
        },
      )
      expect(resp.status()).toBe(200)

      const json = await resp.json()

      // body 必须不含 realmId / realm_id / 任何其他 realm 形字段。
      expect(
        json.realmId,
        'ask 200 body 必须不含 realmId（cert-abuse gate）',
      ).toBeUndefined()
      expect(
        json.realm_id,
        'ask 200 body 必须不含 realm_id（cert-abuse gate）',
      ).toBeUndefined()

      // body 的 key 集合必须恰好是 ["authorized"]。
      const keys = Object.keys(json).sort()
      expect(
        keys,
        `ask 200 body 必须只含 authorized 字段 (got ${JSON.stringify(keys)})`,
      ).toEqual(['authorized'])

      demoLogger.testCode.log('已确认 200 响应不泄漏 realm 信息（仅 authorized 字段）')
    })
  })

  // ==========================================================================
  // Test 3 — 缺少/错误的 ask key → 401
  // 映射: [US-CD-005] §4.5 共享密钥门禁（shared-key gate）
  // DRAFT 来源: .ai/user-stories/core/realm-custom-domain.md
  // ==========================================================================

  test('缺少/错误的 ask key → 401', async ({ page, demoLogger }) => {
    // [US-CD-005] §4.5 shared-key gate — DRAFT: .ai/user-stories/core/realm-custom-domain.md
    testStartTime = Date.now()

    await verifyTestEnvironment(page, {
      requiredRealms: ['admin'],
      requiredUsers: ['admin@cas.com'],
      skipRealmVerification: true,
      skipDatabaseCheck: false,
      skipRedisCheck: false,
    })

    // ----------------------------------------------------------------------
    // Step A: 完全不带 X-Herald-Ask-Key → 401
    // ----------------------------------------------------------------------
    await test.step('Step A: GET authorize 不带 X-Herald-Ask-Key → 401', async () => {
      const resp = await page.request.get(
        `${API_BASE}/api/internal/custom-domain/authorize`,
        {
          params: { host: DEDICATED_HOSTNAME },
          // 故意不传 X-Herald-Ask-Key
        },
      )

      expect(
        resp.status(),
        `缺少 ask key 必须返回 401 (got ${resp.status()})`,
      ).toBe(401)

      const body = await resp.json().catch(() => ({}))
      demoLogger.testCode.log(`无 ask key → 401 (body: ${JSON.stringify(body)})`)
    })

    // ----------------------------------------------------------------------
    // Step B: 带错误 ask key → 401
    // ----------------------------------------------------------------------
    await test.step('Step B: GET authorize 带错误 ask key → 401', async () => {
      const resp = await page.request.get(
        `${API_BASE}/api/internal/custom-domain/authorize`,
        {
          params: { host: DEDICATED_HOSTNAME },
          headers: { 'X-Herald-Ask-Key': 'definitely-wrong-key' },
        },
      )

      expect(
        resp.status(),
        `错误 ask key 必须返回 401 (got ${resp.status()})`,
      ).toBe(401)

      const body = await resp.json().catch(() => ({}))
      demoLogger.testCode.log(`错误 ask key → 401 (body: ${JSON.stringify(body)})`)
    })

    // 文档化：host→realm 路由场景（US-CD-005 场景1 终端用户侧）已于 2026-07-09 回退，
    // 只有 ask 门禁可 demo。
    demoLogger.testCode.log(
      '注：US-CD-005 的 host→realm 路由场景已 DEFERRED（2026-07-09 回退），只有 ask 门禁可 demo。',
    )
  })
})
