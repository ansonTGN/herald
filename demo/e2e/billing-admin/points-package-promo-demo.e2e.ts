/**
 * Promotional Points Package Management Demo (Admin)
 *
 * Covers admin-side promotional package management:
 * - US-PP-006: Create promotional package with discount pricing
 * - US-PP-016: Edit promotional package prices and time range
 * - US-PP-018: View expired promotional package and extend to reactivate
 *
 * Each test is self-contained and creates its own test data.
 * Uses helpers from DM-D01 (points-package-promo-helpers).
 */

import { test, expect, cleanupTestData } from '../fixtures/demo-page.fixtures'
import { SELECTORS } from '../selectors'
import { DEMO_ADMIN } from '../helpers/auth'
import { loginWithCredentials } from '../helpers/auth'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import {
  navigateToPointsPackagesAdmin,
  createPromoPackage,
  editPromoPackage,
  findPackageRowByName,
  generatePromoTimeRange,
  generateExpiredPromoTimeRange,
  type PromoPackageFormData,
} from '../helpers/points-package-promo-helpers'

const REALM_ID = DEMO_ADMIN.realmId
const ADMIN_EMAIL = DEMO_ADMIN.email

test.describe('[Points Package Promo] Admin Management Demo', () => {
  let testStartTime: number

  test.beforeEach(async ({ page }) => {
    testStartTime = Date.now()

    await verifyTestEnvironment(page, {
      requiredUsers: [ADMIN_EMAIL],
    })
  })

  test.afterEach(async ({ page }) => {
    await cleanupTestData(page, REALM_ID, {
      keepUsers: [ADMIN_EMAIL],
      timestamp: testStartTime,
    })
  })

  // =========================================================================
  // US-PP-006: Create Promotional Package
  // =========================================================================

  test('should create a promotional package with discount pricing (US-PP-006)', async ({
    page,
    demoLogger,
  }) => {
    const packageName = `promo-create-${testStartTime}`
    const promoTimeRange = generatePromoTimeRange(30)

    const formData: PromoPackageFormData = {
      name: packageName,
      title: `Promo Package ${testStartTime}`,
      points: 2000,
      price: '4.99',
      currency: 'USD',
      sortOrder: 1,
      enabled: true,
      originalPrice: '9.99',
      promoStartTime: promoTimeRange.startTime,
      promoEndTime: promoTimeRange.endTime,
    }

    await test.step('Given: Admin is logged in and on the points packages page', async () => {
      await loginWithCredentials(page, {
        realmId: REALM_ID,
        email: ADMIN_EMAIL,
        password: DEMO_ADMIN.password,
      })

      await navigateToPointsPackagesAdmin(page, REALM_ID)
    })

    await test.step('When: Admin creates a promotional package with discount pricing', async () => {
      await createPromoPackage(page, REALM_ID, formData)
    })

    await test.step('Then: The promotional package appears in the list with correct details', async () => {
      const packageRow = findPackageRowByName(page, packageName)
      await expect(packageRow).toBeVisible()

      // Verify promotional badge/type is shown
      await expect(packageRow.getByText('Promotional')).toBeVisible()

      // Verify the package shows discounted price
      await expect(packageRow.getByText('4.99')).toBeVisible()

      // Verify the original price is displayed
      await expect(packageRow.getByText('9.99')).toBeVisible()

      console.log(`[US-PP-006] Promotional package "${packageName}" created with discount (9.99 -> 4.99)`)
    })

    await test.step('And: The package does NOT show as expired since it is active', async () => {
      const packageRow = findPackageRowByName(page, packageName)
      // Active promo should not show "Expired" badge
      await expect(packageRow.getByText('Expired')).not.toBeVisible()
    })
  })

  // =========================================================================
  // US-PP-016: Edit Promotional Package
  // =========================================================================

  test('should edit a promotional package prices and time range (US-PP-016)', async ({
    page,
    demoLogger,
  }) => {
    const packageName = `promo-edit-${testStartTime}`
    const promoTimeRange = generatePromoTimeRange(14)

    // S1: Create a promotional package first (self-contained)
    const createData: PromoPackageFormData = {
      name: packageName,
      title: `Edit Test Promo ${testStartTime}`,
      points: 3000,
      price: '14.99',
      currency: 'USD',
      sortOrder: 2,
      enabled: true,
      originalPrice: '29.99',
      promoStartTime: promoTimeRange.startTime,
      promoEndTime: promoTimeRange.endTime,
    }

    await test.step('Given: Admin creates a promotional package for editing', async () => {
      await loginWithCredentials(page, {
        realmId: REALM_ID,
        email: ADMIN_EMAIL,
        password: DEMO_ADMIN.password,
      })

      await createPromoPackage(page, REALM_ID, createData)

      // Verify initial creation
      const packageRow = findPackageRowByName(page, packageName)
      await expect(packageRow).toBeVisible()
    })

    let packageId: string

    await test.step('And: Admin extracts the package ID from the list', async () => {
      const packageRow = findPackageRowByName(page, packageName)
      await expect(packageRow).toBeVisible()

      // Find the edit button within the row to extract the package ID
      const editButton = packageRow.locator('[data-testid^="points-package-edit-button-"]')
      await expect(editButton).toBeVisible()

      const editButtonTestId = await editButton.getAttribute('data-testid')
      expect(editButtonTestId).toBeTruthy()
      // Extract ID from format "points-package-edit-button-{id}"
      packageId = editButtonTestId!.replace('points-package-edit-button-', '')
      expect(packageId).toBeTruthy()

      console.log(`[US-PP-016] Extracted package ID: ${packageId}`)
    })

    // S1: Edit promo prices
    await test.step('When: Admin updates the promo prices (discount and original)', async () => {
      const updatedTimeRange = generatePromoTimeRange(21)
      await editPromoPackage(page, REALM_ID, packageId, {
        price: '9.99',
        originalPrice: '39.99',
        promoStartTime: updatedTimeRange.startTime,
        promoEndTime: updatedTimeRange.endTime,
      })
    })

    await test.step('Then: The updated promo prices are reflected in the list', async () => {
      const packageRow = findPackageRowByName(page, packageName)
      await expect(packageRow).toBeVisible()

      // Verify updated discounted price
      await expect(packageRow.getByText('9.99')).toBeVisible()

      // Verify updated original price
      await expect(packageRow.getByText('39.99')).toBeVisible()

      console.log(`[US-PP-016 S1] Promo prices updated (29.99/14.99 -> 39.99/9.99)`)
    })

    // S4: Edit promo time range
    const newTimeRange = generatePromoTimeRange(60)

    await test.step('When: Admin extends the promo time range', async () => {
      await editPromoPackage(page, REALM_ID, packageId, {
        promoStartTime: newTimeRange.startTime,
        promoEndTime: newTimeRange.endTime,
      })
    })

    await test.step('Then: The package remains visible and active with the new time range', async () => {
      const packageRow = findPackageRowByName(page, packageName)
      await expect(packageRow).toBeVisible()

      // Package should still show as promotional
      await expect(packageRow.getByText('Promotional')).toBeVisible()

      // Should not be expired
      await expect(packageRow.getByText('Expired')).not.toBeVisible()

      console.log(`[US-PP-016 S4] Promo time range extended to 60 days`)
    })
  })

  // =========================================================================
  // US-PP-018: Expired Promotional Package
  // =========================================================================

  test('should view expired promo and extend to reactivate (US-PP-018)', async ({
    page,
    demoLogger,
  }) => {
    const packageName = `promo-expired-${testStartTime}`
    const expiredTimeRange = generateExpiredPromoTimeRange()

    const createData: PromoPackageFormData = {
      name: packageName,
      title: `Expired Promo ${testStartTime}`,
      points: 1500,
      price: '5.99',
      currency: 'USD',
      sortOrder: 3,
      enabled: true,
      originalPrice: '12.99',
      promoStartTime: expiredTimeRange.startTime,
      promoEndTime: expiredTimeRange.endTime,
    }

    await test.step('Given: Admin creates a promotional package with expired time range', async () => {
      await loginWithCredentials(page, {
        realmId: REALM_ID,
        email: ADMIN_EMAIL,
        password: DEMO_ADMIN.password,
      })

      await createPromoPackage(page, REALM_ID, createData)
    })

    // S2: Admin views expired promo
    await test.step('When: Admin views the expired promotional package in the list', async () => {
      const packageRow = findPackageRowByName(page, packageName)
      await expect(packageRow).toBeVisible()
    })

    await test.step('Then: The package is shown with "Expired" badge', async () => {
      const packageRow = findPackageRowByName(page, packageName)
      await expect(packageRow).toBeVisible()

      // Verify "Expired" badge is displayed
      await expect(packageRow.getByText('Expired', { exact: true })).toBeVisible()

      console.log(`[US-PP-018 S2] Expired promo "${packageName}" shows "Expired" badge`)
    })

    // S3: Extend expired promo to reactivate
    let packageId: string

    await test.step('And: Admin extracts the package ID to extend the promo', async () => {
      const packageRow = findPackageRowByName(page, packageName)
      await expect(packageRow).toBeVisible()

      const editButton = packageRow.locator('[data-testid^="points-package-edit-button-"]')
      await expect(editButton).toBeVisible()

      const editButtonTestId = await editButton.getAttribute('data-testid')
      expect(editButtonTestId).toBeTruthy()
      packageId = editButtonTestId!.replace('points-package-edit-button-', '')
      expect(packageId).toBeTruthy()
    })

    const reactivatedTimeRange = generatePromoTimeRange(30)

    await test.step('When: Admin extends the expired promo with a new active time range', async () => {
      await editPromoPackage(page, REALM_ID, packageId, {
        promoStartTime: reactivatedTimeRange.startTime,
        promoEndTime: reactivatedTimeRange.endTime,
      })
    })

    await test.step('Then: The package is reactivated and no longer shows "Expired"', async () => {
      const packageRow = findPackageRowByName(page, packageName)
      await expect(packageRow).toBeVisible()

      // "Expired" badge should no longer appear
      await expect(packageRow.getByText('Expired', { exact: true })).not.toBeVisible()

      // Package should still be promotional
      await expect(packageRow.getByText('Promotional')).toBeVisible()

      console.log(`[US-PP-018 S3] Expired promo "${packageName}" reactivated with new time range`)
    })
  })
})
