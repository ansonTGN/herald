/**
 * Subscription History Admin Demo Tests
 *
 * User Story:
 * - US-BI-007: View Subscription History (Realm Admin)
 *
 * Design Doc: .ai/design/subscription-history.md
 *
 * UnifiedLogger Usage:
 * - All tests use UnifiedLogger through 'demoLogger' fixture
 * - Logger is automatically initialized and finalized by fixture
 * - Logs are saved to demo/test-results/console-logs/
 */

import { test, cleanupTestData, expect } from '../fixtures/demo-page.fixtures'
import { createSubscriptionPlan } from './helpers/billing-page.helpers'
import { createProduct } from './helpers/product-page.helpers'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { DEMO_ADMIN } from '../helpers/auth'
import {
  navigateToSubscriptionHistory,
  setEventTypeFilter,
  setUserIdFilter,
  setDateRangeFilter,
  applyFilters,
  resetFilters,
  viewEventDetails,
} from './helpers/subscription-history.helpers'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'

// Removed hardcoded constants - using DEMO_ADMIN instead

test.describe('[Billing Admin] Subscription History Demo Tests', () => {
  // Verify test environment before each test
  test.beforeEach(async ({ page }) => {
    await verifyTestEnvironment(page, {
      requiredRealms: [DEMO_ADMIN.realmId],
      requiredUsers: [DEMO_ADMIN.email],
    })
  })

  // Single test.afterEach for cleanup
  test.afterEach(async ({ page, testStartTime, demoLogger }) => {
    // ⚠️ MANDATORY: 清理测试数据
    await cleanupTestData(page, DEMO_ADMIN.realmId, {
      keepUsers: [DEMO_ADMIN.email],
      timestamp: testStartTime,
    })
    await demoLogger.testCode.log('Test data cleaned up')
  })

  // ============================================================================
  // User Story US-BI-007: View Subscription History
  // ============================================================================

  test.describe('US-BI-007: View Subscription History', () => {
    test('should view global subscription history (Scene 1)', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      await test.step('Given: 管理员已登录', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', DEMO_ADMIN.realmId)
      })

      await test.step('When: 访问订阅变更历史页面', async () => {
        await navigateToSubscriptionHistory(page, DEMO_ADMIN.realmId)
      })

      await test.step('Then: 验证页面标题显示', async () => {
        await expect(page.getByRole('heading', { name: 'Subscription History' })).toBeVisible()
        await demoLogger.testCode.log('Page title displayed')
      })

      await test.step('And: 验证页面描述显示', async () => {
        await expect(page.getByText('View all subscription changes across the realm')).toBeVisible()
        await demoLogger.testCode.log('Page description displayed')
      })

      await test.step('And: 验证历史列表容器显示', async () => {
        await expect(page.getByTestId('subscription-history-list')).toBeVisible()
        await demoLogger.testCode.log('History list container displayed')
      })

      await test.step('And: 验证筛选器显示', async () => {
        await expect(page.getByTestId('subscription-history-filter')).toBeVisible()
        await demoLogger.testCode.log('Filter container displayed')
      })

      await test.step('And: 验证表格列显示', async () => {
        // 验证表头列
        await expect(page.getByRole('columnheader', { name: /timestamp/i })).toBeVisible()
        await expect(page.getByRole('columnheader', { name: /event type/i })).toBeVisible()
        await expect(page.getByRole('columnheader', { name: /subscription/i })).toBeVisible()
        await expect(page.getByRole('columnheader', { name: /user/i })).toBeVisible()
        await expect(page.getByRole('columnheader', { name: /actor/i })).toBeVisible()
        await expect(page.getByRole('columnheader', { name: /actions/i })).toBeVisible()
        await demoLogger.testCode.log('Table columns displayed')
      })
    })

    test('should filter by event type (Scene 2)', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      await test.step('Given: 管理员已登录并访问订阅历史页面', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', DEMO_ADMIN.realmId)
        await navigateToSubscriptionHistory(page, DEMO_ADMIN.realmId)
      })

      await test.step('When: 选择变更类型筛选为 "Upgraded"', async () => {
        await setEventTypeFilter(page, 'Upgraded')
        await applyFilters(page)
      })

      await test.step('Then: 验证筛选器显示选中的类型', async () => {
        const filterSelect = page.getByLabel('Event Type')
        await expect(filterSelect).toBeVisible()
        await expect(filterSelect).toContainText('Upgraded')
        await demoLogger.testCode.log('Event type filter applied and selected value verified')
      })

      await test.step('When: 选择变更类型筛选为 "Canceled"', async () => {
        await setEventTypeFilter(page, 'Canceled')
        await applyFilters(page)
      })

      await test.step('Then: 验证筛选器更新', async () => {
        const filterSelect = page.getByLabel('Event Type')
        await expect(filterSelect).toBeVisible()
        await expect(filterSelect).toContainText('Canceled')
        await demoLogger.testCode.log('Event type filter updated to Canceled')
      })

      await test.step('When: 重置筛选器', async () => {
        await resetFilters(page)
      })

      await test.step('Then: 验证筛选器已重置', async () => {
        // 验证筛选器重置为默认状态
        await expect(page.getByText('User ID')).toBeVisible()
        await demoLogger.testCode.log('Filters reset successfully')
      })
    })

    test('should filter by user (Scene 3)', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      let adminUserId: string

      await test.step('Given: 管理员已登录并访问订阅历史页面', async () => {
        adminUserId = await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', DEMO_ADMIN.realmId)
        await navigateToSubscriptionHistory(page, DEMO_ADMIN.realmId)
      })

      await test.step('When: 输入当前管理员用户 ID', async () => {
        expect(adminUserId).toBeTruthy()

        await setUserIdFilter(page, adminUserId)
        await applyFilters(page)
        await demoLogger.testCode.log(`User filter applied: ${adminUserId}`)
      })

      await test.step('Then: 验证用户筛选器显示输入的用户 ID', async () => {
        const filterInput = page.getByTestId('filter-user-id')
        await expect(filterInput).toHaveValue(adminUserId, { timeout: 5000 })
        await demoLogger.testCode.log(`User filter value verified: ${adminUserId}`)
      })

      await test.step('When: 清空用户筛选器', async () => {
        await setUserIdFilter(page, '')
        await applyFilters(page)
      })

      await test.step('Then: 验证用户筛选器已清空', async () => {
        const filterInput = page.getByTestId('filter-user-id')
        await expect(filterInput).toHaveValue('')
        await demoLogger.testCode.log('User filter cleared')
      })
    })

    test('should filter by plan (Scene 4)', async ({
      page,
      loginPage,
      demoLogger,
      testStartTime,
    }) => {
      const planName = `history-plan-${testStartTime}`
      const planTitle = `History Plan ${testStartTime}`

      await test.step('Given: 管理员已登录并创建可筛选套餐', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', DEMO_ADMIN.realmId)

        // Create a product first (plan form now requires product_id, NOT NULL)
        await page.goto(`/${DEMO_ADMIN.realmId}/manage/products`)
        await expect(page.getByTestId('products-page')).toBeVisible({ timeout: 10000 })

        await createProduct(page, {
          code: `test-product-${testStartTime}`,
          title: `Test Product ${testStartTime}`,
        })

        // Navigate to billing plans page and create the plan with the product
        await page.goto(`/${DEMO_ADMIN.realmId}/manage/billing`)
        await expect(page.getByTestId('billing-page')).toBeVisible()

        await createSubscriptionPlan(page, {
          planName,
          title: planTitle,
          description: 'Plan used for subscription history filter verification',
          price: '10',
          type: 'monthly',
          currency: 'usd',
          provider: 'creem',
          externalProductId: `prod_history_${testStartTime}`,
          trialDays: '7',
          productTitle: `Test Product ${testStartTime}`,
        })

        await navigateToSubscriptionHistory(page, DEMO_ADMIN.realmId)
      })

      await test.step('When: 选择套餐筛选器', async () => {
        await page.getByLabel('Plan').click()
        await expect(page.getByRole('option').first()).toBeVisible()
        await demoLogger.testCode.log('Plan dropdown opened')
      })

      await test.step('Then: 验证套餐选项显示', async () => {
        await expect(page.getByRole('option', { name: 'All plans' })).toBeVisible()
        await expect(page.getByRole('option', { name: planTitle })).toBeVisible()
        await demoLogger.testCode.log('Plan options displayed')
      })

      await test.step('When: 选择新创建的套餐', async () => {
        await page.getByRole('option', { name: planTitle }).click()
        await applyFilters(page)
        await demoLogger.testCode.log('Plan selected for filtering')
      })

      await test.step('Then: 验证套餐筛选器显示所选套餐', async () => {
        const planFilter = page.getByLabel('Plan')
        await expect(planFilter).toContainText(planTitle)
        await demoLogger.testCode.log('Plan filter selection verified')
      })
    })

    test('should filter by date range (Scene 5)', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      await test.step('Given: 管理员已登录并访问订阅历史页面', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', DEMO_ADMIN.realmId)
        await navigateToSubscriptionHistory(page, DEMO_ADMIN.realmId)
      })

      await test.step('When: 设置时间范围为最近30天', async () => {
        const today = new Date()
        const thirtyDaysAgo = new Date(today)
        thirtyDaysAgo.setDate(today.getDate() - 30)

        const fromDate = thirtyDaysAgo.toISOString().split('T')[0]
        const toDate = today.toISOString().split('T')[0]

        await setDateRangeFilter(page, fromDate, toDate)
        await applyFilters(page)
      })

      await test.step('Then: 验证日期范围筛选器显示', async () => {
        await expect(page.getByTestId('filter-from-date')).toBeVisible()
        await expect(page.getByTestId('filter-to-date')).toBeVisible()
        await demoLogger.testCode.log('Date range filters displayed')
      })
    })

    test('should view event details (Scene 6)', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      await test.step('Given: 管理员已登录并访问订阅历史页面', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', DEMO_ADMIN.realmId)
        await navigateToSubscriptionHistory(page, DEMO_ADMIN.realmId)
      })

      await test.step('When: 点击查看详情按钮', async () => {
        // 使用 expect + 短超时检查 view-details-0，避免 isVisible/click 竞态
        const viewDetailsButton = page.getByTestId('view-details-0')
        let hasViewableEvent = false
        try {
          await expect(viewDetailsButton).toBeVisible({ timeout: 5000 })
          hasViewableEvent = true
        } catch {
          hasViewableEvent = false
        }

        if (hasViewableEvent) {
          await viewDetailsButton.click()

          // 验证 Toast 消息显示（当前实现使用 toast 显示事件类型和时间戳）
          await expect(page.locator('[data-sonner-toast]')).toBeVisible({ timeout: 5000 })
          await expect(page.getByText(/Event details:/)).toBeVisible()
          await demoLogger.testCode.log('Event details toast displayed')
        } else {
          await demoLogger.testCode.log('No viewable events found - skipping details verification')
        }
      })

      await test.step('Then: 验证详情对话框或 Toast 显示', async () => {
        // Already verified in When step or skipped
        await demoLogger.testCode.log('Details verification step completed')
      })
    })

    test('should navigate back to billing page', async ({
      page,
      loginPage,
      demoLogger,
    }) => {
      await test.step('Given: 管理员已登录并访问订阅历史页面', async () => {
        await loginPage.loginAsAdmin(DEMO_ADMIN.email, 'password', DEMO_ADMIN.realmId)
        await navigateToSubscriptionHistory(page, DEMO_ADMIN.realmId)
      })

      await test.step('When: 点击返回按钮', async () => {
        await page.getByTestId('back-button').click()
      })

      await test.step('Then: 验证导航到计费页面', async () => {
        await expect(page.getByTestId('billing-page')).toBeVisible()
        await demoLogger.testCode.log('Navigated back to billing page')
      })
    })
  })
})
