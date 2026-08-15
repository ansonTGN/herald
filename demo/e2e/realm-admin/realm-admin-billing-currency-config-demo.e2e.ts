/**
 * Realm Admin billing default-currency configuration demo (US-MC-001)
 *
 * User story: US-MC-001 — S1 set the realm default currency (persisted),
 * S2 update takes effect for users WITHOUT a personal override, S3 malformed /
 * reserved currency codes rejected at the form. The US-MC-00x stories extend
 * the published multi-price baseline in
 * docs/user-stories/billing/entitlement-mapping.md.
 *
 * The user-visible effect (S1/S2 "对本 Realm 未设个人偏好的用户生效") is asserted
 * on the purchase page as the regular user: a switchable group renders only
 * the highlighted (active) currency's price cards, so the highlighted realm
 * default is observed via the visible card set — persistent page state, not
 * transient styling or toasts.
 *
 * The multi-currency catalog (real Stripe product spanning USD/EUR/CNY) is
 * ensured in beforeAll via {@link ensureMultiCurrencyCatalog}; the regular
 * user's preferred-currency override is cleared via the profile API as a
 * defensive Given (its UI path is covered by the regular-user demo file).
 *
 * Data lifecycle: the billing default-currency config is deleted in beforeAll
 * (clean "not set" start) and afterAll; catalog mapping rows are disabled in
 * afterAll so the realm's purchase page stays unpolluted for other demos.
 *
 * Run: uv run scripts/demo-test-runner.py demo/e2e/realm-admin/realm-admin-billing-currency-config-demo.e2e.ts
 */

import { test, expect, clearBrowserStorage } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { SELECTORS } from '../selectors'
import { hasStripePayment, secrets } from '../secrets/env'
import { loginAsAdmin } from '../helpers/auth'
import {
  type MultiCurrencyCatalog,
  setupMultiCurrencyDemo,
  teardownMultiCurrencyDemo,
  setRealmDefaultCurrency,
  setUserPreferredCurrency,
  slugifyEntitlementKey,
  withSessionApiContext,
} from '../helpers/multiple-currency-helpers'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'
const REALM_ID = 'realm-001'
const REALM_ADMIN_EMAIL = 'admin@realm-001.com'
const USER_EMAIL = 'user@realm-001.com'

// Resolved in beforeAll (worker scope).
let catalog: MultiCurrencyCatalog | null = null
let entitlementSlug = ''

/**
 * `[Realm Admin] 默认货币配置 (US-MC-001)` — the describe title the runner
 * targets via `--grep`.
 */
test.describe('[Realm Admin] 默认货币配置 (US-MC-001)', () => {
  test.beforeAll(async () => {
    // Skip gracefully when Stripe credentials are absent (live dependency).
    test.skip(!hasStripePayment(), 'Stripe credentials required (live catalog dependency)')

    // Clean slate: S1 starts from "realm has no default currency".
    catalog = await setupMultiCurrencyDemo({
      baseUrl: BASE_URL,
      realmId: REALM_ID,
      adminEmail: REALM_ADMIN_EMAIL,
      stripeSecretKey: secrets.stripe.secretKey!,
      stripePublishableKey: secrets.stripe.publishableKey!,
      stripeWebhookSecret: secrets.stripe.webhookSecret ?? '',
      seedDefaultCurrency: null,
    })
    entitlementSlug = slugifyEntitlementKey(catalog.entitlementKey)
  })

  test.afterAll(async () => {
    if (!catalog || !hasStripePayment()) return
    try {
      await teardownMultiCurrencyDemo({
        baseUrl: BASE_URL,
        realmId: REALM_ID,
        adminEmail: REALM_ADMIN_EMAIL,
        catalog,
      })
    } catch (error) {
      // Cleanup failure must be loud but must not mask test failures.
      console.error('[realm-admin-billing-currency] afterAll cleanup failed:', error)
    }
  })

  test.beforeEach(async ({ page }) => {
    await verifyTestEnvironment(page, {
      requiredRealms: [REALM_ID],
      requiredUsers: [REALM_ADMIN_EMAIL],
    })
  })

  // ==========================================================================
  // US-MC-001 S1 + S3 — invalid codes rejected at the form (original state
  // untouched), a valid code saved and persisted, and the default immediately
  // visible to a user without a personal override.
  // ==========================================================================
  test('US-MC-001 S3/S1: 非法货币码被拒；设置默认货币并持久，未设偏好用户按其高亮', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    requireCatalog()

    await test.step('Given: Realm Admin 打开设置页 Billing Tab（默认货币未设置）', async () => {
      await loginAsAdmin(page, { realmId: REALM_ID })
      await page.goto(`/${REALM_ID}/manage/settings`)
      await expect(page.locator(SELECTORS.settingsPage.container)).toBeVisible()
      await page.locator(SELECTORS.billingCurrencyConfig.tab).click()
      await expect(page.locator(SELECTORS.billingCurrencyConfig.input)).toBeVisible()
      // beforeAll deleted the config row → the form starts empty.
      await expect(page.locator(SELECTORS.billingCurrencyConfig.input)).toHaveValue('')
    })

    await test.step('When: 填写非 ISO 货币码（美元）(US-MC-001 S3)', async () => {
      await page.locator(SELECTORS.billingCurrencyConfig.input).fill('美元')
    })

    await test.step('Then: 校验提示出现且保存被禁用（原状态不变）', async () => {
      await expect(page.locator(SELECTORS.billingCurrencyConfig.invalidHint)).toBeVisible()
      await expect(page.locator(SELECTORS.billingCurrencyConfig.saveButton)).toBeDisabled()
      await demoLogger.testCode.log('Non-ISO code rejected at form validation')
    })

    await test.step('And When: 填写保留码 XXX', async () => {
      await page.locator(SELECTORS.billingCurrencyConfig.input).fill('XXX')
      await expect(page.locator(SELECTORS.billingCurrencyConfig.invalidHint)).toBeVisible()
      await expect(page.locator(SELECTORS.billingCurrencyConfig.saveButton)).toBeDisabled()
      await demoLogger.testCode.log('Reserved code XXX rejected at form validation')
    })

    await test.step('When: 填写合法码 USD 并保存 (US-MC-001 S1)', async () => {
      await page.locator(SELECTORS.billingCurrencyConfig.input).fill('USD')
      await expect(page.locator(SELECTORS.billingCurrencyConfig.invalidHint)).toBeHidden()

      const saveResponse = await Promise.all([
        page.waitForResponse(
          (resp) =>
            resp.url().includes(`/api/configs/${REALM_ID}`) &&
            resp.url().includes('/batch') &&
            resp.request().method() === 'POST',
          { timeout: 15000 },
        ),
        page.locator(SELECTORS.billingCurrencyConfig.saveButton).click(),
      ])
      expect([200, 201]).toContain(saveResponse[0].status())
    })

    await test.step('Then: reload 后表单仍显示 USD（配置持久）', async () => {
      await page.goto(`/${REALM_ID}/manage/settings`)
      await expect(page.locator(SELECTORS.settingsPage.container)).toBeVisible()
      await page.locator(SELECTORS.billingCurrencyConfig.tab).click()
      await expect(page.locator(SELECTORS.billingCurrencyConfig.input)).toHaveValue('USD')
      await demoLogger.testCode.log('Realm default currency USD persisted after reload')
    })

    await test.step('And Then: 未设个人偏好的用户购买页高亮 USD（S1 用户生效面）', async () => {
      // Drop the admin session deterministically before switching personas:
      // leftover localStorage credentials can auto-redirect the login page and
      // starve loginAsUser of the login card (racy without this).
      await clearBrowserStorage(page)
      await loginPage.loginAsUser(USER_EMAIL, 'password', REALM_ID)
      await withSessionApiContext(loginPage, (ctx) => setUserPreferredCurrency(ctx, BASE_URL, null))
      await page.goto(`/${REALM_ID}/user/purchase-points`)
      await expect(page.locator(SELECTORS.purchasePoints.page)).toBeVisible({ timeout: 15000 })

      const group = page.locator(SELECTORS.purchaseCurrencyGroup.entitlement(entitlementSlug))
      await expect(group).toBeVisible()
      // Highlight chain falls to the realm default: USD cards render, the
      // other currencies' cards stay unmounted.
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['usd-month'])),
      ).toBeVisible()
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['cny-month'])),
      ).toHaveCount(0)
      await demoLogger.testCode.log('User without override highlights realm default USD')
    })
  })

  // ==========================================================================
  // US-MC-001 S2 — updating the default immediately re-anchors subsequent
  // resolution for users without a personal override.
  // ==========================================================================
  test('US-MC-001 S2: 更新默认货币为 EUR，未设偏好用户按 EUR 高亮', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    requireCatalog()

    await test.step('Given: 当前默认货币为 USD（API 预置）', async () => {
      await loginPage.loginAsAdmin(REALM_ADMIN_EMAIL, 'password', REALM_ID)
      await withSessionApiContext(loginPage, (ctx) =>
        setRealmDefaultCurrency(ctx, BASE_URL, REALM_ID, 'USD'),
      )
    })

    await test.step('When: 在设置页将默认货币改为 EUR 并保存', async () => {
      await page.goto(`/${REALM_ID}/manage/settings`)
      await expect(page.locator(SELECTORS.settingsPage.container)).toBeVisible()
      await page.locator(SELECTORS.billingCurrencyConfig.tab).click()
      await expect(page.locator(SELECTORS.billingCurrencyConfig.input)).toHaveValue('USD')

      await page.locator(SELECTORS.billingCurrencyConfig.input).fill('EUR')
      const saveResponse = await Promise.all([
        page.waitForResponse(
          (resp) =>
            resp.url().includes(`/api/configs/${REALM_ID}`) &&
            resp.url().includes('/batch') &&
            resp.request().method() === 'POST',
          { timeout: 15000 },
        ),
        page.locator(SELECTORS.billingCurrencyConfig.saveButton).click(),
      ])
      expect([200, 201]).toContain(saveResponse[0].status())
      await demoLogger.testCode.log('Realm default currency updated USD → EUR')
    })

    await test.step('Then: 未设个人偏好的用户购买页改按 EUR 高亮', async () => {
      // Drop the admin session deterministically before switching personas:
      // leftover localStorage credentials can auto-redirect the login page and
      // starve loginAsUser of the login card (racy without this).
      await clearBrowserStorage(page)
      await loginPage.loginAsUser(USER_EMAIL, 'password', REALM_ID)
      await withSessionApiContext(loginPage, (ctx) => setUserPreferredCurrency(ctx, BASE_URL, null))
      await page.goto(`/${REALM_ID}/user/purchase-points`)
      await expect(page.locator(SELECTORS.purchasePoints.page)).toBeVisible({ timeout: 15000 })

      const group = page.locator(SELECTORS.purchaseCurrencyGroup.entitlement(entitlementSlug))
      await expect(group).toBeVisible()
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['eur-month'])),
      ).toBeVisible()
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['usd-month'])),
      ).toHaveCount(0)
      await demoLogger.testCode.log('User without override now highlights EUR')
    })
  })
})

// ============================================================================
// Helpers
// ============================================================================

function requireCatalog(): void {
  if (!catalog || !entitlementSlug) {
    throw new Error('beforeAll did not populate the multi-currency catalog')
  }
}
