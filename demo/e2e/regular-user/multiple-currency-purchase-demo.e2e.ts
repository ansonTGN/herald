/**
 * Multiple-currency purchase demo — user-side main chain
 *
 * User stories: US-MC-002 S1/S2/S3 (preferred-currency override set / clear /
 * invalid), US-MC-003 S1/S2/S3 (purchase page currency grouping, manual
 * switch, preferred-unavailable hint), US-MC-004 S1 display side (explicit
 * price-row checkout in the manually-selected currency, no cross-currency
 * order). The US-MC-00x stories extend the published multi-price baseline in
 * docs/user-stories/billing/entitlement-mapping.md.
 *
 * The multi-currency catalog is a REAL Stripe product ensured in beforeAll via
 * {@link ensureMultiCurrencyCatalog} (create-or-reuse + provider sync + batch
 * enable): one product, four recurring prices (USD/month, USD/year, EUR/month,
 * CNY/month) sharing one entitlement key — the grouping substrate. The realm
 * default currency is seeded to USD via the realm-config API (US-MC-001's UI
 * write path is covered by realm-admin-billing-currency-config-demo.e2e.ts).
 *
 * Assertion discipline: highlight is asserted via the VISIBLE price-card set
 * (a switchable group renders only the active currency's cards — persistent
 * page state, not transient styling or toasts). The US-MC-004 load-bearing
 * assertion retrieves the real Stripe checkout session and verifies
 * line_items[0].price (id + currency + unit_amount) matches the EUR row the
 * user selected — the "charged currency equals displayed currency" invariant.
 *
 * Data lifecycle: catalog rows are disabled and the billing default-currency
 * config is deleted in afterAll, so this demo leaves the realm's purchase page
 * unpolluted for other demos. The user's preferred-currency override is left
 * cleared by the last test (US-MC-002 S2 exercises the clear in-UI).
 *
 * Run: uv run scripts/demo-test-runner.py demo/e2e/regular-user/multiple-currency-purchase-demo.e2e.ts
 */

import { test, expect } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { SELECTORS } from '../selectors'
import { hasStripePayment, secrets } from '../secrets/env'
import {
  initiateMultiPriceCheckout,
  selectPriceCard,
} from '../helpers/multi-price-purchase.helpers'
import {
  type MultiCurrencyCatalog,
  setupMultiCurrencyDemo,
  teardownMultiCurrencyDemo,
  setUserPreferredCurrency,
  slugifyEntitlementKey,
  withSessionApiContext,
} from '../helpers/multiple-currency-helpers'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'
const REALM_ID = 'realm-001'
const USER_EMAIL = 'user@realm-001.com'
const REALM_ADMIN_EMAIL = 'admin@realm-001.com'

/** Realm default seeded in beforeAll (US-MC-002 S1 Given + S2 fallback target). */
const REALM_DEFAULT_CURRENCY = 'USD'

// Resolved in beforeAll (worker scope). Empty until the multi-currency product
// is ensured + synced + enabled.
let catalog: MultiCurrencyCatalog | null = null
/** kebab-case slug of the catalog's entitlement key (per-entitlement testids). */
let entitlementSlug = ''

/**
 * `[Regular User] 多货币购买与偏好货币 (US-MC-002/003/004)` — the describe
 * title the runner targets via `--grep`.
 */
test.describe('[Regular User] 多货币购买与偏好货币 (US-MC-002/003/004)', () => {
  test.beforeAll(async () => {
    // Skip gracefully when Stripe credentials are absent (live dependency).
    test.skip(!hasStripePayment(), 'Stripe credentials required (live catalog dependency)')

    catalog = await setupMultiCurrencyDemo({
      baseUrl: BASE_URL,
      realmId: REALM_ID,
      adminEmail: REALM_ADMIN_EMAIL,
      stripeSecretKey: secrets.stripe.secretKey!,
      stripePublishableKey: secrets.stripe.publishableKey!,
      stripeWebhookSecret: secrets.stripe.webhookSecret ?? '',
      seedDefaultCurrency: REALM_DEFAULT_CURRENCY,
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
      console.error('[multiple-currency-purchase] afterAll cleanup failed:', error)
    }
  })

  test.beforeEach(async ({ page }) => {
    await verifyTestEnvironment(page, {
      requiredRealms: [REALM_ID],
      requiredUsers: [USER_EMAIL],
    })
  })

  // ==========================================================================
  // US-MC-002 S1/S3 + US-MC-003 S1 — invalid code rejected, override saved,
  // purchase page grouped by currency with the preferred currency highlighted
  // (default-expanded) and same-currency periods listed together.
  // ==========================================================================
  test('US-MC-002 S3/S1 + US-MC-003 S1: 非法货币码被拒，偏好货币生效并按货币分组高亮', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    requireCatalog()

    await test.step('Given: 用户登录且无偏好货币覆盖（清理残留）', async () => {
      await loginPage.loginAsUser(USER_EMAIL, 'password', REALM_ID)
      await withSessionApiContext(loginPage, (ctx) => setUserPreferredCurrency(ctx, BASE_URL, null))
    })

    await test.step('When: 在资料页填写保留码 XXX 并保存 (US-MC-002 S3)', async () => {
      await page.goto(`/${REALM_ID}/user/profile`)
      await expect(page.locator(SELECTORS.profile.preferredCurrencyInput)).toBeVisible()
      await page.locator(SELECTORS.profile.preferredCurrencyInput).fill('XXX')
      await page.locator(SELECTORS.profile.preferredCurrencySave).click()
    })

    await test.step('Then: 本地校验拒绝且原状态不变（未发起保存）', async () => {
      await expect(page.locator(SELECTORS.profile.preferredCurrencyError)).toBeVisible()
      // The "current" line still shows the not-set copy — the override was not
      // mutated by a rejected entry.
      await expect(page.locator(SELECTORS.profile.preferredCurrencyCurrent)).not.toContainText(
        'XXX',
      )
      await demoLogger.testCode.log('Reserved code XXX rejected at form validation')
    })

    await test.step('When: 设置偏好货币 CNY 并保存 (US-MC-002 S1)', async () => {
      await page.locator(SELECTORS.profile.preferredCurrencyInput).fill('CNY')
      await page.locator(SELECTORS.profile.preferredCurrencySave).click()
      await expect(page.locator(SELECTORS.profile.preferredCurrencyCurrent)).toHaveText('CNY')
    })

    await test.step('Then: 购买页按 CNY/USD/EUR 分组，偏好货币 CNY 高亮默认展开 (US-MC-003 S1)', async () => {
      await page.goto(`/${REALM_ID}/user/purchase-points`)
      await expect(page.locator(SELECTORS.purchasePoints.page)).toBeVisible({ timeout: 15000 })

      const group = page.locator(SELECTORS.purchaseCurrencyGroup.entitlement(entitlementSlug))
      await expect(group).toBeVisible()
      // All available currencies stay presented (展示侧全展示, DEC-006).
      for (const code of ['cny', 'usd', 'eur']) {
        await expect(
          group.locator(SELECTORS.purchaseCurrencyGroup.option(entitlementSlug, code)),
        ).toBeVisible()
      }
      // Base-currency annotation is part of the switcher (Adaptive Pricing note).
      await expect(
        group.locator(SELECTORS.purchaseCurrencyGroup.adaptivePricingNote(entitlementSlug)),
      ).toBeVisible()

      // Highlight = only the active currency's cards render. CNY is active.
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['cny-month'])),
      ).toBeVisible()
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['usd-month'])),
      ).toHaveCount(0)
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['eur-month'])),
      ).toHaveCount(0)
      await demoLogger.testCode.log('CNY group highlighted and expanded by preferred currency')
    })

    await test.step('When: 切换到 USD 组 (同货币多计费周期并列, DEC-005)', async () => {
      await page
        .locator(SELECTORS.purchaseCurrencyGroup.option(entitlementSlug, 'usd'))
        .click()
    })

    await test.step('Then: USD 月付与年付两行同时可见，CNY 行不渲染', async () => {
      const group = page.locator(SELECTORS.purchaseCurrencyGroup.entitlement(entitlementSlug))
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['usd-month'])),
      ).toBeVisible()
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['usd-year'])),
      ).toBeVisible()
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['cny-month'])),
      ).toHaveCount(0)
      await demoLogger.testCode.log('USD group lists month + year side by side')
    })
  })

  // ==========================================================================
  // US-MC-003 S2 + US-MC-004 S1 (display side) — manual switch to EUR, select
  // the EUR price row, and verify the real Stripe checkout session charges the
  // EUR row (id + currency + amount). Browser checkout always submits an
  // explicit mapping id (DEC-008), so the charged currency must equal the
  // displayed one — the no-cross-currency invariant.
  // ==========================================================================
  test('US-MC-003 S2 + US-MC-004 S1: 手动切换 EUR 并按 EUR 价格行发起购买（不串货币）', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    requireCatalog()

    await test.step('Given: 用户偏好货币为 CNY，购买页 CNY 组高亮', async () => {
      await loginPage.loginAsUser(USER_EMAIL, 'password', REALM_ID)
      await withSessionApiContext(loginPage, (ctx) => setUserPreferredCurrency(ctx, BASE_URL, 'CNY'))
      await page.goto(`/${REALM_ID}/user/purchase-points`)
      await expect(page.locator(SELECTORS.purchasePoints.page)).toBeVisible({ timeout: 15000 })
      const group = page.locator(SELECTORS.purchaseCurrencyGroup.entitlement(entitlementSlug))
      await expect(group).toBeVisible()
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['cny-month'])),
      ).toBeVisible()
    })

    await test.step('When: 手动切换到 EUR 组并选择 EUR 价格行 (US-MC-003 S2)', async () => {
      await page
        .locator(SELECTORS.purchaseCurrencyGroup.option(entitlementSlug, 'eur'))
        .click()
      const group = page.locator(SELECTORS.purchaseCurrencyGroup.entitlement(entitlementSlug))
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['eur-month'])),
      ).toBeVisible()
      await selectPriceCard(page, catalog!.product.priceIds['eur-month'])
    })

    let checkoutUrl = ''
    await test.step('And: 发起购买（显式 target_id → createPaymentAttempt）', async () => {
      // The purchase flow auto-redirects the page to the Stripe checkout URL
      // right after the POST, which evicts Playwright's buffered response
      // body — reading the Response's json() after the redirect flakes with
      // "Network.getResponseBody: No resource with given identifier". The
      // checkout URL is therefore taken from the final address bar.
      const checkoutResponse = await initiateMultiPriceCheckout(page, {
        mappingId: catalog!.mappingIds['eur-month'],
        paymentProvider: 'stripe',
      })
      // The status is readable even when the body has been evicted.
      expect(checkoutResponse.status(), 'create-payment-attempt must succeed').toBe(201)

      // The auto-redirect to the real Stripe checkout page is the persistent
      // post-initiation state (not a toast): wait for the navigation and take
      // the checkout URL from the address bar.
      await page.waitForURL(/checkout\.stripe\.com/, { timeout: 15000 })
      checkoutUrl = page.url()

      // The no-cross-currency invariant is asserted on the NEXT step against
      // the real Stripe checkout session (the charge artifact): its line item
      // must reference the EUR price row selected on the purchase page. The
      // Herald purchase-history API only lists SUCCEEDED attempts (webhook
      // completion never fires in the demo env), so it cannot witness a just
      // created Pending attempt.
      await demoLogger.testCode.log(`EUR checkout initiated: ${checkoutUrl.slice(0, 60)}...`)
    })

    await test.step('Then: Stripe 结算会话引用 EUR 价格行（货币与金额一致，US-MC-004 S1）', async () => {
      const match = /(cs_(?:live|test)_[A-Za-z0-9]+)/.exec(checkoutUrl)
      expect(match, 'checkout URL must expose a parseable Stripe session id').toBeTruthy()

      const sessionResp = await fetch(
        `https://api.stripe.com/v1/checkout/sessions/${match![1]}?expand[]=line_items`,
        { headers: { Authorization: `Bearer ${secrets.stripe.secretKey}` } },
      )
      // Read the body exactly once before asserting.
      const sessionText = await sessionResp.text()
      expect(
        sessionResp.ok,
        `Stripe retrieve session failed: ${sessionResp.status} ${sessionText}`,
      ).toBeTruthy()
      const session = JSON.parse(sessionText) as {
        line_items?: {
          data: Array<{
            price?: { id: string; currency: string; unit_amount: number | null } | null
          }>
        }
      }
      const lineItemPrice = session.line_items?.data?.[0]?.price
      expect(lineItemPrice, 'expected line_items[0].price to be populated').toBeTruthy()
      // THE load-bearing assertion: the charged row is the EUR/month price the
      // user picked on the purchase page — no cross-currency substitution.
      expect(lineItemPrice!.id).toBe(catalog!.product.priceIds['eur-month'])
      expect(lineItemPrice!.currency).toBe('eur')
      expect(lineItemPrice!.unit_amount).toBe(1000)
      await demoLogger.testCode.log(
        `Checkout line item: price=${lineItemPrice!.id} currency=eur amount=1000`,
      )
    })
  })

  // ==========================================================================
  // US-MC-003 S3 — preferred currency unavailable: every available currency
  // stays presented with a hint, and manual selection still works.
  // ==========================================================================
  test('US-MC-003 S3: 偏好货币不可用时全展示并提示，仍可手动选择', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    requireCatalog()

    await test.step('Given: 用户偏好货币为 GBP（产品仅 USD/EUR/CNY）', async () => {
      await loginPage.loginAsUser(USER_EMAIL, 'password', REALM_ID)
      await withSessionApiContext(loginPage, (ctx) => setUserPreferredCurrency(ctx, BASE_URL, 'GBP'))
    })

    await test.step('Then: 购买页仍展示全部可用货币并提示偏好货币不可用', async () => {
      await page.goto(`/${REALM_ID}/user/purchase-points`)
      await expect(page.locator(SELECTORS.purchasePoints.page)).toBeVisible({ timeout: 15000 })
      const group = page.locator(SELECTORS.purchaseCurrencyGroup.entitlement(entitlementSlug))
      await expect(group).toBeVisible()

      for (const code of ['cny', 'usd', 'eur']) {
        await expect(
          group.locator(SELECTORS.purchaseCurrencyGroup.option(entitlementSlug, code)),
        ).toBeVisible()
      }
      const hint = group.locator(
        SELECTORS.purchaseCurrencyGroup.preferredUnavailable(entitlementSlug),
      )
      await expect(hint).toBeVisible()
      await expect(hint).toContainText('GBP')
      await demoLogger.testCode.log('Preferred GBP unavailable hint shown, all currencies listed')
    })

    await test.step('When: 手动选择 USD 组', async () => {
      await page
        .locator(SELECTORS.purchaseCurrencyGroup.option(entitlementSlug, 'usd'))
        .click()
    })

    await test.step('Then: USD 价格行可见且可发起选择（不因偏好缺失受阻）', async () => {
      const group = page.locator(SELECTORS.purchaseCurrencyGroup.entitlement(entitlementSlug))
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['usd-month'])),
      ).toBeVisible()
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['usd-year'])),
      ).toBeVisible()
    })
  })

  // ==========================================================================
  // US-MC-002 S2 — clearing the override falls back to the realm default
  // (USD seeded in beforeAll). Also covers the degrade-display invariant: a
  // single-currency entitlement group renders NO currency switcher.
  // ==========================================================================
  test('US-MC-002 S2: 清除偏好回退 Realm 默认高亮；单货币组不渲染切换器', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    requireCatalog()

    await test.step('Given: 用户偏好货币为 CNY', async () => {
      await loginPage.loginAsUser(USER_EMAIL, 'password', REALM_ID)
      await withSessionApiContext(loginPage, (ctx) => setUserPreferredCurrency(ctx, BASE_URL, 'CNY'))
    })

    await test.step('When: 在资料页清除偏好货币 (US-MC-002 S2)', async () => {
      await page.goto(`/${REALM_ID}/user/profile`)
      await expect(page.locator(SELECTORS.profile.preferredCurrencyClear)).toBeEnabled()
      await page.locator(SELECTORS.profile.preferredCurrencyClear).click()
      // The current line flips back to the localized "not set" copy.
      await expect(page.locator(SELECTORS.profile.preferredCurrencyCurrent)).not.toHaveText('CNY')
    })

    await test.step('Then: 购买页高亮回退到 Realm 默认 USD', async () => {
      await page.goto(`/${REALM_ID}/user/purchase-points`)
      await expect(page.locator(SELECTORS.purchasePoints.page)).toBeVisible({ timeout: 15000 })
      const group = page.locator(SELECTORS.purchaseCurrencyGroup.entitlement(entitlementSlug))
      await expect(group).toBeVisible()
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['usd-month'])),
      ).toBeVisible()
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['usd-year'])),
      ).toBeVisible()
      await expect(
        group.locator(SELECTORS.purchasePriceCard.priceCard(catalog!.product.priceIds['cny-month'])),
      ).toHaveCount(0)
      await demoLogger.testCode.log('Fallback highlight = realm default USD after clear')
    })

    await test.step('And: 单货币权益组降级展示（无货币切换器, US-MC-006 语义）', async () => {
      // The demo seed provisions single-currency one-time Stripe mappings in
      // realm-001 (same substrate the unified-purchase demos rely on). Such a
      // group must degrade to a flat price list — no switch container.
      const creditPacks = page.locator(SELECTORS.purchasePriceCard.creditPacksGrid)
      await expect(creditPacks).toBeVisible()
      const entitlementBlocks = creditPacks.locator('[data-testid^="purchase-entitlement-"]')
      const blockCount = await entitlementBlocks.count()
      expect(blockCount, 'expected at least one seeded one-time entitlement group').toBeGreaterThan(
        0,
      )
      let degradeVerified = false
      for (let i = 0; i < blockCount; i++) {
        const block = entitlementBlocks.nth(i)
        const hasSwitch = await block
          .locator('[data-testid^="purchase-currency-switch-"]')
          .count()
        if (hasSwitch === 0) {
          degradeVerified = true
          break
        }
      }
      expect(
        degradeVerified,
        'a single-currency entitlement group must render without a currency switcher',
      ).toBeTruthy()
      await demoLogger.testCode.log('Single-currency group degrades without switcher')
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

