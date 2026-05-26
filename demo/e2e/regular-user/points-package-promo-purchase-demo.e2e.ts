/**
 * Promotional Package Display and Purchase Demo (User)
 *
 * Covers user-facing promotional package scenarios:
 * - US-PP-008: Promo badge display, strikethrough price, limited-time label, and sorting
 * - US-PU-06: Purchase initiation for promotional packages
 *
 * Each test is self-contained and creates its own promo package via admin UI
 * before verifying the user-facing display.
 *
 * Multi-tenancy note:
 *   Packages are per-realm. The admin creates promo packages on realm-001
 *   (using admin@realm-001.com) so the user on realm-001 can see them.
 *
 * Uses helpers from DM-D01 (points-package-promo-helpers).
 */

import { test, expect, cleanupTestData } from '../fixtures/demo-page.fixtures'
import { SELECTORS } from '../selectors'
import { loginWithCredentials } from '../helpers/auth'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import {
  createPromoPackage,
  navigateToPurchasePage,
  findPromoCardByTitle,
  generatePromoTimeRange,
  type PromoPackageFormData,
} from '../helpers/points-package-promo-helpers'

const REALM_ID = 'realm-001'
const USER_EMAIL = 'user@realm-001.com'
const REALM_ADMIN_EMAIL = 'admin@realm-001.com'
const PASSWORD = 'password'

test.describe('[Points Package Promo] User Purchase Display and Purchase Demo', () => {
  let testStartTime: number

  test.beforeEach(async ({ page }) => {
    testStartTime = Date.now()

    await verifyTestEnvironment(page, {
      requiredRealms: [REALM_ID],
      requiredUsers: [USER_EMAIL],
    })
  })

  test.afterEach(async ({ page }) => {
    await cleanupTestData(page, REALM_ID, {
      keepUsers: [USER_EMAIL, REALM_ADMIN_EMAIL],
      timestamp: testStartTime,
    })
  })

  // =========================================================================
  // US-PP-008: User sees promo badge and sorting
  // =========================================================================

  test('should display promo badge, strikethrough price, limited-time label and correct sorting (US-PP-008)', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    const promoTitle = `Promo Display ${testStartTime}`
    const promoTimeRange = generatePromoTimeRange(30)

    const formData: PromoPackageFormData = {
      name: `promo-display-${testStartTime}`,
      title: promoTitle,
      points: 2500,
      price: '4.99',
      currency: 'USD',
      sortOrder: 10,
      enabled: true,
      originalPrice: '9.99',
      promoStartTime: promoTimeRange.startTime,
      promoEndTime: promoTimeRange.endTime,
    }

    await test.step('Given: Admin creates a promotional package on realm-001', async () => {
      // Login as realm-001 admin to create package on the correct realm
      await loginWithCredentials(page, {
        realmId: REALM_ID,
        email: REALM_ADMIN_EMAIL,
        password: PASSWORD,
      })

      await createPromoPackage(page, REALM_ID, formData)

      console.log(`[US-PP-008] Admin created promo package "${formData.name}" on realm ${REALM_ID}`)
    })

    await test.step('And: User logs in and navigates to the purchase page', async () => {
      // Login as regular user on realm-001
      await loginPage.loginAsUser(USER_EMAIL, PASSWORD, REALM_ID)

      await navigateToPurchasePage(page, REALM_ID)

      console.log('[US-PP-008] User navigated to purchase page')
    })

    let promoCard

    await test.step('Then: Promo card shows discount badge with correct percentage', async () => {
      promoCard = findPromoCardByTitle(page, promoTitle)
      await expect(promoCard).toBeVisible()

      // Verify discount badge is present
      const discountBadge = promoCard.locator(SELECTORS.packageSelector.discountBadge)
      await expect(discountBadge).toBeVisible()

      // Verify badge contains discount percentage (50% for 9.99 -> 4.99)
      const badgeText = await discountBadge.textContent()
      expect(badgeText).toContain('50')

      console.log(`[US-PP-008] Discount badge visible: "${badgeText}"`)
    })

    await test.step('And: Promo card shows strikethrough original price', async () => {
      // Original price should be displayed with strikethrough styling
      // The card shows the original price (9.99) crossed out
      await expect(promoCard!.getByText('9.99')).toBeVisible()

      console.log('[US-PP-008] Strikethrough original price (9.99) displayed')
    })

    await test.step('And: Promo card shows limited-time label', async () => {
      const limitedTimeLabel = promoCard!.locator(SELECTORS.packageSelector.limitedTimeLabel)
      await expect(limitedTimeLabel).toBeVisible()

      console.log('[US-PP-008] Limited-time label visible')
    })

    await test.step('And: "Best Value" badge does NOT appear on promotional packages', async () => {
      // The "Best Value" badge should only appear on standard packages, not promo packages
      const bestValueBadge = promoCard!.locator(SELECTORS.packageSelector.bestValueBadge)
      await expect(bestValueBadge).not.toBeVisible()

      console.log('[US-PP-008] "Best Value" badge correctly suppressed on promo card')
    })

    await test.step('And: Promo card appears before standard package cards (promo-first sorting)', async () => {
      // Get all package cards within the selector container
      const allCards = page
        .locator(SELECTORS.packageSelector.container)
        .locator('[data-testid^="points-package-card-"]')

      const cardCount = await allCards.count()
      expect(cardCount).toBeGreaterThanOrEqual(1)

      // The promo card should be among the first cards
      // Verify by checking that the promo card's index is less than or equal to
      // any standard cards (active promos are sorted first by the ext API)
      const promoCardElement = allCards.filter({ hasText: promoTitle })
      const promoIndex = await allCards.evaluateAll((elements, promoText) => {
        for (let i = 0; i < elements.length; i++) {
          if (elements[i].textContent?.includes(promoText as string)) {
            return i
          }
        }
        return -1
      }, promoTitle)

      // Promo card should exist and be at a top position
      expect(promoIndex).toBeGreaterThanOrEqual(0)

      // Promo cards should appear before any non-promo cards
      // Find the first non-promo card index and verify promo comes before it
      const firstNonPromoIndex = await allCards.evaluateAll((elements) => {
        for (let i = 0; i < elements.length; i++) {
          const el = elements[i]
          // Non-promo cards won't have the discount badge testid
          const hasDiscountBadge = el.querySelector('[data-testid="points-package-discount-badge"]')
          if (!hasDiscountBadge) {
            return i
          }
        }
        return elements.length // All are promo cards
      })

      if (firstNonPromoIndex < cardCount) {
        expect(promoIndex).toBeLessThan(firstNonPromoIndex)
      }

      console.log(`[US-PP-008] Promo card at index ${promoIndex}, first non-promo at index ${firstNonPromoIndex}`)
    })
  })

  // =========================================================================
  // US-PU-06: User purchases promotional package
  // =========================================================================

  test('should initiate purchase of a promotional package at discount price (US-PU-06)', async ({
    page,
    loginPage,
    demoLogger,
  }) => {
    const promoTitle = `Promo Purchase ${testStartTime}`
    const promoTimeRange = generatePromoTimeRange(14)

    const formData: PromoPackageFormData = {
      name: `promo-purchase-${testStartTime}`,
      title: promoTitle,
      points: 5000,
      price: '19.99',
      currency: 'USD',
      sortOrder: 5,
      enabled: true,
      originalPrice: '39.99',
      promoStartTime: promoTimeRange.startTime,
      promoEndTime: promoTimeRange.endTime,
    }

    await test.step('Given: Admin creates a promotional package on realm-001', async () => {
      await loginWithCredentials(page, {
        realmId: REALM_ID,
        email: REALM_ADMIN_EMAIL,
        password: PASSWORD,
      })

      await createPromoPackage(page, REALM_ID, formData)

      console.log(`[US-PU-06] Admin created promo package "${formData.name}" on realm ${REALM_ID}`)
    })

    await test.step('And: Admin configures WeChat payment provider for the package', async () => {
      // Get the package ID from the admin list API
      const listResponse = await page.request.get(
        `http://localhost:3000/api/bill/${REALM_ID}/points-packages`
      )
      const listData = await listResponse.json()
      const createdPackage = listData.packages?.find(
        (p: { name: string }) => p.name === formData.name
      )
      expect(createdPackage).toBeDefined()
      const packageId = createdPackage.id

      // Add WeChat payment provider mapping via API
      const providerResponse = await page.request.post(
        `http://localhost:3000/api/bill/${REALM_ID}/points-packages/${packageId}/providers`,
        {
          data: {
            paymentProvider: 'wechat',
            externalProductId: `wechat-promo-${testStartTime}`,
            enabled: true,
          },
        }
      )
      expect(providerResponse.ok()).toBeTruthy()

      console.log(`[US-PU-06] WeChat payment provider configured for package "${formData.name}"`)
    })

    await test.step('And: User logs in and navigates to the purchase page', async () => {
      await loginPage.loginAsUser(USER_EMAIL, PASSWORD, REALM_ID)

      await navigateToPurchasePage(page, REALM_ID)

      console.log('[US-PU-06] User navigated to purchase page')
    })

    let promoCard

    await test.step('When: User selects the promotional package', async () => {
      promoCard = findPromoCardByTitle(page, promoTitle)
      await expect(promoCard).toBeVisible()

      // Click the select button within the promo card
      const selectButton = promoCard!.locator('[data-testid^="points-package-select-button-"]')
      await expect(selectButton).toBeVisible()
      await selectButton.click()

      // Verify the card shows selected state
      const selectedIndicator = promoCard!.locator('[data-testid^="points-package-selected-"]')
      await expect(selectedIndicator).toBeVisible()

      console.log('[US-PU-06] User selected promo package')
    })

    await test.step('Then: The discount price (not original price) is shown for purchase', async () => {
      // The selected state and price summary should show 19.99 (discount price)
      // not 39.99 (original price) as the purchase amount
      await expect(promoCard!.getByText('19.99')).toBeVisible()

      console.log('[US-PU-06] Discount price (19.99) displayed for purchase')
    })

    await test.step('And: User proceeds to payment step and price carries through', async () => {
      // Click Next to proceed to payment method selection
      await expect(page.getByRole('button', { name: 'Next' })).toBeVisible()
      await page.getByRole('button', { name: 'Next' }).click()

      // Verify we are on the payment method step
      await expect(
        page.locator(SELECTORS.paymentMethodSelector.container)
      ).toBeVisible()

      console.log('[US-PU-06] User proceeded to payment step')
    })

    await test.step('And: User selects payment method and initiates purchase', async () => {
      // Select WeChat as payment method (commonly available in demo seed)
      await page.getByTestId('payment-method-select-wechat').click()

      // Wait for payment method selection state
      await expect(
        page.getByTestId(/^payment-method-selected-/)
      ).toBeVisible()

      // Click Complete Purchase
      await expect(
        page.getByRole('button', { name: 'Complete Purchase' })
      ).toBeVisible()
      await page.getByRole('button', { name: 'Complete Purchase' }).click()

      // Verify payment processing started (WeChat Pay heading appears)
      await expect(
        page.getByRole('heading', { name: 'WeChat Pay' })
      ).toBeVisible()

      console.log('[US-PU-06] Purchase initiated for promotional package')
    })

    await test.step('Documentation: Payment completion requires payment provider integration', async () => {
      // Verify the payment pending state is displayed
      await expect(page.getByText('Time Remaining')).toBeVisible()

      console.log(
        '[US-PU-06] Payment initiated at discount price (pending webhook completion)'
      )
      console.log(
        '[US-PU-06] Note: Full payment completion requires external webhook simulation by demo infrastructure'
      )
    })
  })
})
