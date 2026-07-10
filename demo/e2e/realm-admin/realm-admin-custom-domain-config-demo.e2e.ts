/**
 * Realm Admin 演示测试 - Custom-domain 配置 admin 生命周期
 *
 * 用户故事（DRAFT，发布前不得改写为 docs/user-stories/，路径保持原样）：
 * - [US-CD-003] 自定义域名配置的草稿/发布/恢复生命周期（draft→publish→restore）
 * - [US-CD-001] 场景1（保存自定义域名草稿）、场景2（系统提供 CNAME 指引并展示生效状态）、
 *   场景3（自定义域名全局唯一冲突）
 *   DRAFT 来源（原样引用）：`.ai/user-stories/core/realm-custom-domain.md`
 *
 * ⚠️ 范围声明（OUT OF SCOPE / DEFERRED）：
 * 本测试只覆盖 config-admin 生命周期（5 个 config 端点的 UI 映射 + 持久后端状态）。
 * host→realm 路由相关的用户故事 —— US-CD-002（终端用户在自定义域名下完成 auth 流）、
 * US-CD-004（自定义域名与 path-based canonical 域名并存）以及 US-CD-005 中 host-resolution
 * 部分 —— 均 OUT OF SCOPE / DEFERRED。原因：host-mapping 机制已于 2026-07-09 回退，
 * 当前没有任何 host-based 公共入口可断言。因此 US-CD-003 场景1 中「草稿不泄漏到公共
 * 登录页」的端到端语义无法断言（没有 host 路由就没有 host-based 公共登录面）—— 本测试
 * 改为断言后端持久状态：草稿写入 `draft`，`published` 不变。authorize (ask) 门禁是
 * 独立 item DE-D02 的职责，本测试不断言。
 *
 * Teardown 策略（restore-balanced）：
 * 在 afterEach 中对每个被触及的 realm（admin / realm-001）通过 UI 驱动
 * `SettingsPage.resetCustomDomainConfig()` 清空 hostname 并发布空白 baseline config，
 * 丢弃任何 dangling draft。该 helper 在 SettingsPage 内部对所有失败做了兜底
 * （catch + warn），不会硬失败测试运行。
 *
 * 断言策略：
 * - 关键生命周期断言落在持久后端状态（GET /api/realms/{realm}/config/custom-domain 返回的
 *   `draft.hostname` / `published.hostname`），而非 sonner/toast 等自动消失提示。
 * - 草稿提示可见性（custom-domain-draft-notice）作为持久 UI 状态辅助断言。
 * - CNAME 指引断言落在 custom-domain-cname-guidance 面板文本（包含配置的 cname_target），
 *   该面板常驻渲染、不会自动消失。
 * - 域名冲突（409）通过直接 PUT /draft 的 409 响应断言（可靠、持久），而非瞬态表单错误。
 *
 * @see .ai/user-stories/core/realm-custom-domain.md （DRAFT 来源，路径保持原样）
 * @see frontend/src/components/realm-config/custom-domain-config-form.tsx
 * @see backend/api/src/application/http/realm/custom_domain_config.rs
 */

import { test, expect, cleanupTestData } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { loginAsAdmin } from '../helpers/auth'
import { SettingsPage } from '../pages/settings-page'
import type { CustomDomainFormValues } from '../pages/settings-page'
import { SELECTORS } from '../selectors'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'

/**
 * The configured CNAME target, from `[custom_domain].cname_target` in
 * backend/config/demo.toml. The CNAME guidance panel renders this value; the
 * draft-save test asserts it appears so the panel is never empty.
 */
const CONFIGURED_CNAME_TARGET = 'custom.demo.herald.local'

/**
 * Realms touched by this demo suite (all restore-balanced in afterEach).
 *
 * The uniqueness-conflict scenario uses `admin` + `realm-001` — the two realms
 * the demo seed (`scripts/lib/demo_seed.py`) provisions WITH the
 * `admin-web-console` client app so their admins can log in.
 */
const TOUCHED_REALMS = ['admin', 'realm-001'] as const

/**
 * GET the persisted custom-domain config state for a realm.
 *
 * Shares the browser context's auth cookies (admin is logged in). Returns the
 * parsed `CustomDomainConfigStateResponse` body
 * (`{published, draft, hasPrevious, cnameTarget, status}`).
 */
async function getCustomDomainState(
  page: import('@playwright/test').Page,
  realmId: string,
): Promise<{
  published: { hostname: string | null }
  draft: { hostname: string | null } | null
  hasPrevious: boolean
  cnameTarget: string
}> {
  const resp = await page.request.get(
    `${BASE_URL}/api/realms/${realmId}/config/custom-domain`,
  )
  expect(resp.ok(), `GET custom-domain config for "${realmId}" must succeed`).toBeTruthy()
  return resp.json()
}

test.describe('[Realm Admin] Custom-domain 配置 admin 生命周期演示测试', () => {
  let testStartTime: number
  let settingsPage: SettingsPage

  test.afterEach(async ({ page }) => {
    // Restore-balance every realm touched by this suite. resetCustomDomainConfig
    // is best-effort (catches + warns internally) so teardown never hard-fails.
    for (const realm of TOUCHED_REALMS) {
      try {
        const realmSettings = new SettingsPage(page, undefined, realm)
        await loginAsAdmin(page, { realmId: realm, forceRelogin: true })
        await realmSettings.goto()
        await realmSettings.waitForReady()
        await realmSettings.resetCustomDomainConfig()
      } catch (error) {
        console.warn(`[afterEach] restore-balance failed for realm "${realm}":`, error)
      }
    }

    // ⚠️ MANDATORY: 清理测试数据
    await cleanupTestData(page, 'admin', {
      keepUsers: ['admin@cas.com'],
      timestamp: testStartTime,
    })
  })

  // ==========================================================================
  // Test 1 — 草稿保存与 CNAME 指引展示
  // 映射: [US-CD-001] 场景1（保存自定义域名草稿）、场景2（CNAME 指引 + 生效状态展示）
  // DRAFT 来源: .ai/user-stories/core/realm-custom-domain.md
  // ==========================================================================

  test('草稿保存与 CNAME 指引展示', async ({ page, demoLogger }) => {
    // [US-CD-001] 场景1/场景2 — DRAFT: .ai/user-stories/core/realm-custom-domain.md
    testStartTime = Date.now()
    const realmId = 'admin'
    settingsPage = new SettingsPage(page, demoLogger, realmId)

    await verifyTestEnvironment(page, {
      requiredRealms: [realmId],
      requiredUsers: ['admin@cas.com'],
      skipRealmVerification: true,
      skipDatabaseCheck: false,
      skipRedisCheck: false,
    })

    const draftValues: CustomDomainFormValues = {
      hostname: 'login.acme.com',
    }

    await test.step('登录并进入 Custom-domain 配置 Tab', async () => {
      await loginAsAdmin(page, { realmId })
      await settingsPage.goto()
      await settingsPage.waitForReady()
      await settingsPage.switchToCustomDomainTab()
    })

    // ----------------------------------------------------------------------
    // Step A [US-CD-001] 场景1: 填入域名并保存草稿 → 草稿提示可见
    // ----------------------------------------------------------------------
    await test.step('Step A [US-CD-001] 场景1: 填入域名并保存草稿', async () => {
      await settingsPage.fillCustomDomainForm(draftValues)
      await settingsPage.saveCustomDomainDraft()

      // 草稿提示必须可见（持久 UI 状态，draft 已保存或表单为 dirty，非 toast）。
      await expect(page.getByTestId('custom-domain-draft-notice')).toBeVisible({ timeout: 10000 })
      demoLogger.testCode.log('草稿已保存，draft notice 可见')
    })

    // ----------------------------------------------------------------------
    // Step B [US-CD-001] 场景2: CNAME 指引面板展示配置的 cname_target
    // ----------------------------------------------------------------------
    await test.step('Step B [US-CD-001] 场景2: CNAME 指引面板包含配置的 cname_target', async () => {
      // 指引面板常驻渲染（custom-domain-config-form.tsx:184），包含配置的 cname_target。
      const guidanceText = await settingsPage.getCnameGuidanceText()
      expect(guidanceText.length, 'CNAME guidance panel must render non-empty').toBeGreaterThan(0)
      expect(
        guidanceText,
        `CNAME guidance must contain the configured cname_target "${CONFIGURED_CNAME_TARGET}"`,
      ).toContain(CONFIGURED_CNAME_TARGET)
      demoLogger.testCode.log(`CNAME 指引面板包含 cname_target: ${CONFIGURED_CNAME_TARGET}`)
    })

    // ----------------------------------------------------------------------
    // Step C [US-CD-001] 场景2: CNAME/TLS 生效状态展示（状态徽章存在，非瞬态）
    // ----------------------------------------------------------------------
    await test.step('Step C [US-CD-001] 场景2: CNAME/TLS 状态徽章渲染', async () => {
      // 状态徽章常驻渲染（custom-domain-config-form.tsx:201 / :209），断言其存在，
      // 不依赖自动消失提示。
      await expect(page.getByTestId('custom-domain-status-cname')).toBeVisible({ timeout: 10000 })
      await expect(page.getByTestId('custom-domain-status-tls')).toBeVisible({ timeout: 10000 })
      demoLogger.testCode.log('CNAME/TLS 状态徽章已渲染')
    })
  })

  // ==========================================================================
  // Test 2 — 草稿不污染已发布状态
  // 映射: [US-CD-003] 场景1（未发布草稿不影响终端用户当前域名）
  // DRAFT 来源: .ai/user-stories/core/realm-custom-domain.md
  // ==========================================================================

  test('草稿不污染已发布状态', async ({ page, demoLogger }) => {
    // [US-CD-003] 场景1 — DRAFT: .ai/user-stories/core/realm-custom-domain.md
    // ⚠️ 该用户故事原文要求「终端用户仍进入 login.acme.com（已发布域名），不受草稿影响」。
    // 但 host→realm 路由已于 2026-07-09 回退，当前没有 host-based 公共登录面可断言。
    // 因此本测试改为断言持久后端状态：草稿写入 draft.hostname，published 不变。
    testStartTime = Date.now()
    const realmId = 'admin'
    settingsPage = new SettingsPage(page, demoLogger, realmId)

    await verifyTestEnvironment(page, {
      requiredRealms: [realmId],
      requiredUsers: ['admin@cas.com'],
      skipRealmVerification: true,
      skipDatabaseCheck: false,
      skipRedisCheck: false,
    })

    const draftHostnameA = 'draft-a.cd.test'

    await test.step('登录并进入 Custom-domain 配置 Tab', async () => {
      await loginAsAdmin(page, { realmId })
      await settingsPage.goto()
      await settingsPage.waitForReady()
      await settingsPage.switchToCustomDomainTab()
    })

    await test.step('保存 hostname A 的草稿', async () => {
      await settingsPage.fillCustomDomainForm({ hostname: draftHostnameA })
      await settingsPage.saveCustomDomainDraft()
      await expect(page.getByTestId('custom-domain-draft-notice')).toBeVisible({ timeout: 10000 })
      demoLogger.testCode.log(`草稿 hostname A (${draftHostnameA}) 已保存`)
    })

    await test.step('通过 GET 断言草稿写入 draft.hostname，published 不变', async () => {
      const state = await getCustomDomainState(page, realmId)

      // 草稿必须包含 hostname A（持久后端状态）。
      expect(state.draft, 'draft must be present after saveCustomDomainDraft').not.toBeNull()
      expect(state.draft!.hostname).toBe(draftHostnameA)

      // published 不得被草稿污染：published.hostname 不应是草稿值。
      // （published 可能是 null/上一版 baseline，但绝不是未发布的草稿值。）
      expect(
        state.published.hostname,
        'published.hostname must NOT equal the unpublished draft value',
      ).not.toBe(draftHostnameA)

      demoLogger.testCode.log(
        `draft.hostname=${state.draft!.hostname}, published.hostname=${state.published.hostname} (草稿未污染已发布状态)`,
      )
    })

    // 文档化：端到端「草稿不泄漏到公共登录页」语义无法断言。
    demoLogger.testCode.log(
      '注：host→realm 路由已于 2026-07-09 回退，无 host-based 公共登录面可断言草稿不泄漏；改为断言持久后端状态。',
    )
  })

  // ==========================================================================
  // Test 3 — 发布后 mapping 生效
  // 映射: [US-CD-003] 场景2（发布后终端用户访问新自定义域名）
  // DRAFT 来源: .ai/user-stories/core/realm-custom-domain.md
  // ==========================================================================

  test('发布后 mapping 生效', async ({ page, demoLogger }) => {
    // [US-CD-003] 场景2 — DRAFT: .ai/user-stories/core/realm-custom-domain.md
    // ⚠️ 该用户故事原文要求「终端用户访问 login2.acme.com 进入 realm-1 的 auth 流」。
    // host→realm 路由已回退，无法断言终端用户访问。改为断言发布后持久后端状态：
    // published.hostname == 发布的值。authorize (ask) 门禁是 DE-D02 的职责，不断言。
    testStartTime = Date.now()
    const realmId = 'admin'
    settingsPage = new SettingsPage(page, demoLogger, realmId)

    await verifyTestEnvironment(page, {
      requiredRealms: [realmId],
      requiredUsers: ['admin@cas.com'],
      skipRealmVerification: true,
      skipDatabaseCheck: false,
      skipRedisCheck: false,
    })

    const publishedHostname = 'published.cd.test'

    await test.step('登录并进入 Custom-domain 配置 Tab', async () => {
      await loginAsAdmin(page, { realmId })
      await settingsPage.goto()
      await settingsPage.waitForReady()
      await settingsPage.switchToCustomDomainTab()
    })

    await test.step('填入 hostname 并保存草稿', async () => {
      // ⚠️ publish 发布的是已保存的草稿（settings.tsx:453-463 的 onPublish
      // mutation 故意不发送表单值；后端 handle_publish 从 DRAFT_KEY 读取）。
      // 因此必须先 saveDraft，再 publish。
      await settingsPage.fillCustomDomainForm({ hostname: publishedHostname })
      await settingsPage.saveCustomDomainDraft()
      await expect(page.getByTestId('custom-domain-draft-notice')).toBeVisible({ timeout: 10000 })
      demoLogger.testCode.log(`hostname ${publishedHostname} 草稿已保存`)
    })

    await test.step('发布草稿', async () => {
      await settingsPage.publishCustomDomain()
      demoLogger.testCode.log(`hostname ${publishedHostname} 已发布`)
    })

    await test.step('通过 GET 断言 published.hostname == 发布值', async () => {
      const state = await getCustomDomainState(page, realmId)
      expect(
        state.published.hostname,
        'published.hostname must equal the just-published value',
      ).toBe(publishedHostname)
      demoLogger.testCode.log(`published.hostname=${state.published.hostname} (发布生效)`)
    })

    // 文档化：终端用户在自定义域名下的 auth 流（US-CD-002）与 authorize 门禁（DE-D02）均不在本测试范围。
    demoLogger.testCode.log(
      '注：终端用户 host-based auth 流（US-CD-002）与 authorize 门禁（DE-D02）均不在本测试范围。',
    )
  })

  // ==========================================================================
  // Test 4 — 恢复上一版
  // 映射: [US-CD-003] 场景3（恢复上一版自定义域名配置）
  // DRAFT 来源: .ai/user-stories/core/realm-custom-domain.md
  // ==========================================================================

  test('恢复上一版', async ({ page, demoLogger }) => {
    // [US-CD-003] 场景3 — DRAFT: .ai/user-stories/core/realm-custom-domain.md
    // 发布 hostname A，再发布 hostname B，然后 restore → 断言 published 回退为 A。
    testStartTime = Date.now()
    const realmId = 'admin'
    settingsPage = new SettingsPage(page, demoLogger, realmId)

    await verifyTestEnvironment(page, {
      requiredRealms: [realmId],
      requiredUsers: ['admin@cas.com'],
      skipRealmVerification: true,
      skipDatabaseCheck: false,
      skipRedisCheck: false,
    })

    const hostnameA = 'restore-a.cd.test'
    const hostnameB = 'restore-b.cd.test'

    await test.step('登录、保存并发布 hostname A（作为可恢复的上一版）', async () => {
      // ⚠️ publish 发布的是已保存的草稿（onPublish mutation 不发送表单值；
      // 后端 handle_publish 从 DRAFT_KEY 读取），因此每次 publish 前都要 saveDraft。
      await loginAsAdmin(page, { realmId })
      await settingsPage.goto()
      await settingsPage.waitForReady()
      await settingsPage.switchToCustomDomainTab()
      await settingsPage.fillCustomDomainForm({ hostname: hostnameA })
      await settingsPage.saveCustomDomainDraft()
      await settingsPage.publishCustomDomain()
      demoLogger.testCode.log(`hostname A (${hostnameA}) 已发布（上一版）`)
    })

    await test.step('保存并发布 hostname B（当前版本）', async () => {
      await settingsPage.fillCustomDomainForm({ hostname: hostnameB })
      await settingsPage.saveCustomDomainDraft()
      await settingsPage.publishCustomDomain()
      demoLogger.testCode.log(`hostname B (${hostnameB}) 已发布（当前版本）`)
    })

    await test.step('通过 GET 确认当前 published == B', async () => {
      const state = await getCustomDomainState(page, realmId)
      expect(state.published.hostname).toBe(hostnameB)
      expect(state.hasPrevious, 'hasPrevious must be true after a second publish').toBeTruthy()
      demoLogger.testCode.log('当前 published == B，hasPrevious == true')
    })

    await test.step('执行 restore → 恢复上一版（hostname A）', async () => {
      // 驱动 restore 对话框：custom-domain-restore → 等待 restore-dialog 可见 → 点击 confirm。
      await settingsPage.restoreCustomDomain()
      demoLogger.testCode.log('restore 已确认，上一版（hostname A）已恢复')
    })

    await test.step('通过 GET 断言 published 已回退为 hostname A', async () => {
      const state = await getCustomDomainState(page, realmId)
      expect(
        state.published.hostname,
        'published.hostname must revert to hostname A after restore',
      ).toBe(hostnameA)
      demoLogger.testCode.log(`published.hostname=${state.published.hostname} (已回退为 A)`)
    })
  })

  // ==========================================================================
  // Test 5 — 域名全局唯一冲突
  // 映射: [US-CD-001] 场景3（自定义域名全局唯一）
  // DRAFT 来源: .ai/user-stories/core/realm-custom-domain.md
  // ==========================================================================

  test('域名全局唯一冲突', async ({ page, demoLogger }) => {
    // [US-CD-001] 场景3 — DRAFT: .ai/user-stories/core/realm-custom-domain.md
    // 在 realm-001 发布 unique hostname，然后在 admin 尝试保存相同 hostname 的草稿 → 期望 409。
    // 优先通过直接 PUT /draft 断言 409（可靠、持久），而非瞬态表单错误/toast。
    testStartTime = Date.now()

    await verifyTestEnvironment(page, {
      requiredRealms: ['admin'],
      requiredUsers: ['admin@cas.com'],
      skipRealmVerification: true,
      skipDatabaseCheck: false,
      skipRedisCheck: false,
    })

    const conflictRealm = 'realm-001'
    const otherRealm = 'admin'
    const uniqueHostname = 'unique.cd.test'

    await test.step(`在 ${conflictRealm} 发布独特 hostname ${uniqueHostname}`, async () => {
      const conflictSettings = new SettingsPage(page, demoLogger, conflictRealm)
      await loginAsAdmin(page, { realmId: conflictRealm, forceRelogin: true })
      await conflictSettings.goto()
      await conflictSettings.waitForReady()
      await conflictSettings.switchToCustomDomainTab()
      // ⚠️ publish 发布的是已保存的草稿（onPublish mutation 不发送表单值；
      // 后端 handle_publish 从 DRAFT_KEY 读取），因此先 saveDraft 再 publish。
      await conflictSettings.fillCustomDomainForm({ hostname: uniqueHostname })
      await conflictSettings.saveCustomDomainDraft()
      await conflictSettings.publishCustomDomain()
      demoLogger.testCode.log(`${conflictRealm} 已发布 ${uniqueHostname}`)
    })

    await test.step(`在 ${otherRealm} 直接 PUT /draft 相同 hostname → 期望 409`, async () => {
      // 登录到 otherRealm 以共享其 auth cookie，然后直接 PUT draft 端点。
      await loginAsAdmin(page, { realmId: otherRealm, forceRelogin: true })

      const resp = await page.request.put(
        `${BASE_URL}/api/realms/${otherRealm}/config/custom-domain/draft`,
        { data: { hostname: uniqueHostname } },
      )

      // 域名全局唯一：otherRealm 不得占用 conflictRealm 已发布的 hostname → 409。
      expect(
        resp.status(),
        `PUT draft with already-occupied hostname must return 409 (got ${resp.status()})`,
      ).toBe(409)

      const body = await resp.json().catch(() => ({}))
      demoLogger.testCode.log(
        `${otherRealm} PUT draft ${uniqueHostname} → 409 (body: ${JSON.stringify(body)})`,
      )

      // 额外验证：otherRealm 的 published 未被这次失败的 draft 写入污染。
      const otherState = await getCustomDomainState(page, otherRealm)
      expect(
        otherState.published.hostname,
        'failed conflict draft must NOT mutate otherRealm published config',
      ).not.toBe(uniqueHostname)
      demoLogger.testCode.log(`${otherRealm} published 未被冲突 draft 污染`)
    })
  })
})
